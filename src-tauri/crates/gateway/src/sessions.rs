use axum::{
    extract::{
        ws::{Message, WebSocket},
        State, WebSocketUpgrade,
    },
    response::Response,
};
use futures::{SinkExt, StreamExt};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tokio::sync::mpsc;

use crate::server::GatewayAppState;
use crate::session_registry::{SessionAddress, SessionInfo, SessionRegistry};

// ── Client → Server messages ──────────────────────────────────────────────

#[derive(Deserialize)]
#[serde(tag = "type")]
enum SessionClientMessage {
    #[serde(rename = "session.connect")]
    Connect { conversation_id: String },
    #[serde(rename = "session.disconnect")]
    Disconnect { conversation_id: String },
    #[serde(rename = "session.list")]
    List,
    #[serde(rename = "session.input")]
    Input { target: String, content: String },
    #[serde(rename = "session.acquire_lock")]
    AcquireLock { target: String },
    #[serde(rename = "session.release_lock")]
    ReleaseLock { target: String },
}

// ── Server → Client messages ──────────────────────────────────────────────

#[derive(Serialize)]
#[serde(tag = "type")]
enum SessionServerMessage {
    #[serde(rename = "session.connected")]
    Connected {
        address: String,
    },
    #[serde(rename = "session.list_result")]
    ListResult {
        sessions: Vec<SessionInfo>,
    },
    #[serde(rename = "session.output")]
    Output {
        from: String,
        content: String,
    },
    #[serde(rename = "session.lock_acquired")]
    LockAcquired {
        target: String,
    },
    #[serde(rename = "session.lock_released")]
    LockReleased {
        target: String,
    },
    #[serde(rename = "session.lock_lost")]
    LockLost {
        target: String,
    },
    #[serde(rename = "session.error")]
    Error {
        message: String,
    },
}

/// GET /v1/sessions — WebSocket upgrade for session interop
///
/// No Gateway API key required. Sessions WebSocket is for agent-to-agent
/// communication within the local network — it does not expose any LLM
/// proxy functionality and is deliberately kept separate from the
/// Bearer-auth-protected API routes.
pub async fn sessions_handler(
    State(state): State<GatewayAppState>,
    ws: WebSocketUpgrade,
) -> Response {
    let registry = state.session_registry.clone();
    let device_id = state.this_device_id.clone();
    let db = state.db.clone();
    ws.on_upgrade(move |socket| handle_sessions_session(socket, registry, device_id, db))
}

async fn handle_sessions_session(
    socket: WebSocket,
    registry: Arc<SessionRegistry>,
    device_id: String,
    db: sea_orm::DatabaseConnection,
) {
    let (mut ws_sender, mut ws_receiver) = socket.split();
    // Channel for forwarding output (from registry) to the WS client
    let (forward_tx, mut forward_rx) = mpsc::unbounded_channel::<String>();

    let mut my_address: Option<SessionAddress> = None;
    // Held for the lifetime of this connection; kept alive so registry
    // unregistration + drop ordering is automatic when the task exits.
    #[allow(unused_assignments)]
    let mut _session_output_tx: Option<mpsc::Sender<(String, String)>> = None;

    // Helper: release all locks held by this connection
    async fn release_locks(registry: &SessionRegistry, addr: &SessionAddress) {
        let sessions = registry.list().await;
        for s in &sessions {
            let target = SessionAddress {
                device_id: s.device_id.clone(),
                conversation_id: s.conversation_id.clone(),
            };
            registry.release_lock(&target, addr).await;
        }
    }

    'main: loop {
        tokio::select! {
            // Outgoing forwarding: message from registry output_tx → WS client
            forward_json = forward_rx.recv() => {
                match forward_json {
                    Some(json) => {
                        if ws_sender.send(Message::Text(json.into())).await.is_err() {
                            break 'main;
                        }
                    }
                    None => {
                        // forward_tx closed; continue processing incoming only
                    }
                }
            }
            // Incoming WS message from client
            msg_result = ws_receiver.next() => {
                let msg = match msg_result {
                    Some(Ok(m)) => m,
                    Some(Err(e)) => {
                        tracing::debug!("Sessions WS recv error: {}", e);
                        break 'main;
                    }
                    None => break 'main,
                };

                let text = match msg {
                    Message::Text(t) => t,
                    Message::Close(_) => break 'main,
                    Message::Ping(data) => {
                        if ws_sender.send(Message::Pong(data)).await.is_err() {
                            break 'main;
                        }
                        continue 'main;
                    }
                    _ => continue 'main,
                };

                let client_msg: SessionClientMessage = match serde_json::from_str(&text) {
                    Ok(m) => m,
                    Err(e) => {
                        let json = serde_json::to_string(&SessionServerMessage::Error {
                            message: format!("Invalid message: {}", e),
                        }).unwrap();
                        let _ = ws_sender.send(Message::Text(json.into())).await;
                        continue 'main;
                    }
                };

                match client_msg {
                    SessionClientMessage::Connect { conversation_id } => {
                        let address = SessionAddress {
                            device_id: device_id.clone(),
                            conversation_id: conversation_id.clone(),
                        };

                        // Register this WS client in the session registry with a real
                        // output channel so it appears in session lists and can receive
                        // input forwarded from other sessions.
                        let (output_tx, mut output_rx) = mpsc::channel::<(String, String)>(256);
                        let _handle = registry.register(address.clone(), output_tx.clone()).await;
                        _session_output_tx = Some(output_tx);
                        my_address = Some(address.clone());

                        // Forward output from registry to WS client
                        let fwd = forward_tx.clone();
                        tokio::spawn(async move {
                            while let Some((source, content)) = output_rx.recv().await {
                                let json = serde_json::to_string(&SessionServerMessage::Output {
                                    from: source,
                                    content,
                                }).unwrap();
                                if fwd.send(json).is_err() {
                                    break;
                                }
                            }
                        });

                        let json = serde_json::to_string(&SessionServerMessage::Connected {
                            address: address.to_wire(),
                        }).unwrap();
                        if ws_sender.send(Message::Text(json.into())).await.is_err() {
                            break 'main;
                        }
                    }

                    SessionClientMessage::Disconnect { conversation_id: _ } => {
                        if let Some(ref addr) = my_address {
                            release_locks(&registry, addr).await;
                        }
                        let _ = ws_sender.send(Message::Close(None)).await;
                        break 'main;
                    }

                    SessionClientMessage::List => {
                        let sessions = match aqbot_core::repo::conversation::list_conversations(&db).await {
                            Ok(convs) => {
                                let pairs: Vec<(String, String)> = convs
                                    .into_iter()
                                    .map(|c| (c.id, c.title))
                                    .collect();
                                registry.list_with_db(&pairs, &device_id).await
                            }
                            Err(_) => registry.list().await,
                        };
                        let json = serde_json::to_string(&SessionServerMessage::ListResult { sessions }).unwrap();
                        if ws_sender.send(Message::Text(json.into())).await.is_err() {
                            break 'main;
                        }
                    }

                    SessionClientMessage::Input { target, content } => {
                        let source = my_address.as_ref();
                        match SessionAddress::from_wire(&target) {
                            Some(target_addr) => {
                                let handle = registry.get(&target_addr).await;
                                let source_addr_opt: Option<String> = source.map(|s| s.to_wire());
                                let can_send = match handle {
                                    Some(ref h) => {
                                        let lock = h.input_lock_holder.read().await;
                                        match (&source_addr_opt, lock.as_ref()) {
                                            (Some(src_wire), Some(holder)) if holder.to_wire() == *src_wire => true,
                                            _ => false,
                                        }
                                    }
                                    None => false,
                                };

                                if can_send {
                                    if let Some(h) = handle {
                                        let _ = h.output_tx.send((source_addr_opt.clone().unwrap_or_default(), content.clone())).await;
                                    }
                                } else {
                                    let err_msg = if handle.is_none() {
                                        format!("Target session '{}' not found", target)
                                    } else {
                                        "You must acquire the input lock first".into()
                                    };
                                    let json = serde_json::to_string(&SessionServerMessage::Error { message: err_msg }).unwrap();
                                    let _ = ws_sender.send(Message::Text(json.into())).await;
                                }
                            }
                            None => {
                                let json = serde_json::to_string(&SessionServerMessage::Error {
                                    message: format!("Invalid target address: {}", target),
                                }).unwrap();
                                let _ = ws_sender.send(Message::Text(json.into())).await;
                            }
                        }
                    }

                    SessionClientMessage::AcquireLock { target } => {
                        match SessionAddress::from_wire(&target) {
                            Some(target_addr) => {
                                let source = match my_address {
                                    Some(ref a) => a.clone(),
                                    None => {
                                        let json = serde_json::to_string(&SessionServerMessage::Error {
                                            message: "Connect first before acquiring locks".into(),
                                        }).unwrap();
                                        let _ = ws_sender.send(Message::Text(json.into())).await;
                                        continue 'main;
                                    }
                                };
                                match registry.acquire_lock(&target_addr, &source).await {
                                    Ok(()) => {
                                        let json = serde_json::to_string(&SessionServerMessage::LockAcquired {
                                            target: target_addr.to_wire(),
                                        }).unwrap();
                                        let _ = ws_sender.send(Message::Text(json.into())).await;
                                    }
                                    Err(e) => {
                                        let json = serde_json::to_string(&SessionServerMessage::Error { message: e }).unwrap();
                                        let _ = ws_sender.send(Message::Text(json.into())).await;
                                    }
                                }
                            }
                            None => {
                                let json = serde_json::to_string(&SessionServerMessage::Error {
                                    message: format!("Invalid target address: {}", target),
                                }).unwrap();
                                let _ = ws_sender.send(Message::Text(json.into())).await;
                            }
                        }
                    }

                    SessionClientMessage::ReleaseLock { target } => {
                        match SessionAddress::from_wire(&target) {
                            Some(target_addr) => {
                                let source = match my_address {
                                    Some(ref a) => a.clone(),
                                    None => continue 'main,
                                };
                                registry.release_lock(&target_addr, &source).await;
                                let json = serde_json::to_string(&SessionServerMessage::LockReleased {
                                    target: target_addr.to_wire(),
                                }).unwrap();
                                let _ = ws_sender.send(Message::Text(json.into())).await;
                            }
                            None => {
                                let json = serde_json::to_string(&SessionServerMessage::Error {
                                    message: format!("Invalid target address: {}", target),
                                }).unwrap();
                                let _ = ws_sender.send(Message::Text(json.into())).await;
                            }
                        }
                    }
                }
            }
        }
    }

    // On disconnect: release all locks and unregister from session registry
    if let Some(ref addr) = my_address {
        release_locks(&registry, addr).await;
        registry.unregister(addr).await;
    }

    tracing::debug!(
        "Sessions WS connection closed: {}",
        my_address.map(|a| a.to_wire()).unwrap_or_default()
    );
}
