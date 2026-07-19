use futures::SinkExt;
use futures::StreamExt;
use std::collections::HashMap;
use std::sync::{Arc, LazyLock, Mutex};
use tokio::sync::mpsc;
use tokio_tungstenite::connect_async;
use tokio_tungstenite::tungstenite::Message;

use crate::session_registry::SessionRegistry;

/// Channel to send session commands to a remote device's Gateway.
/// Clone-able sender; the receiver runs inside the relay task.
pub type RemoteCommandSender = mpsc::UnboundedSender<String>;

/// Global map of active remote peer connections: device_id → command sender.
static REMOTE_PEERS: LazyLock<Mutex<HashMap<String, RemoteCommandSender>>> =
    LazyLock::new(|| Mutex::new(HashMap::new()));

/// Get the command sender for a remote device, if connected.
pub async fn get_remote_sender(device_id: &str) -> Option<RemoteCommandSender> {
    REMOTE_PEERS.lock().unwrap().get(device_id).cloned()
}

/// Register a remote peer command sender (called by the relay task).
pub async fn register_remote_peer(device_id: String, tx: RemoteCommandSender) {
    REMOTE_PEERS.lock().unwrap().insert(device_id, tx);
}

/// Remove a remote peer on disconnect.
pub async fn unregister_remote_peer(device_id: &str) {
    REMOTE_PEERS.lock().unwrap().remove(device_id);
}

/// Connect to a remote device's Gateway `/v1/sessions` WebSocket and maintain
/// a bidirectional relay for session commands and output.
///
/// No Gateway API key required — the sessions WebSocket is intended for
/// agent-to-agent communication and does not need LLM proxy authentication.
pub async fn connect_remote_peer(
    remote_device_id: String,
    remote_url: String,
    _registry: Arc<SessionRegistry>,
    _this_device_id: String,
) -> Result<(), String> {
    // Build the WebSocket URL — no auth required for session interop
    let ws_url = remote_url.trim_end_matches('/').to_string();

    let (ws_stream, _) = connect_async(&ws_url)
        .await
        .map_err(|e| format!("Failed to connect to remote gateway {}: {}", remote_url, e))?;

    let (mut ws_sender, mut ws_receiver) = ws_stream.split();

    // Channel for sending session commands to the remote
    let (cmd_tx, mut cmd_rx) = mpsc::unbounded_channel::<String>();

    // Register the sender so local tool actions can find it
    register_remote_peer(remote_device_id.clone(), cmd_tx.clone()).await;

    tracing::info!("Remote peer connected to {}", remote_url);

    // Forward commands from local → remote
    let forward_handle = tokio::spawn(async move {
        while let Some(json) = cmd_rx.recv().await {
            if ws_sender.send(Message::Text(json.into())).await.is_err() {
                break;
            }
        }
    });

    // Receive messages from remote → local
    while let Some(msg_result) = ws_receiver.next().await {
        let msg = match msg_result {
            Ok(m) => m,
            Err(e) => {
                tracing::warn!("Remote peer recv error: {}", e);
                break;
            }
        };

        let text = match msg {
            Message::Text(t) => t,
            Message::Close(_) | Message::Ping(_) | Message::Pong(_) => continue,
            _ => continue,
        };

        if let Ok(server_msg) = serde_json::from_str::<serde_json::Value>(&text) {
            let msg_type = server_msg["type"].as_str().unwrap_or("");
            match msg_type {
                "session.output" => {
                    tracing::debug!("Remote session output: {:?}", server_msg);
                }
                "session.list_result" => {
                    tracing::debug!("Remote session list received");
                }
                "session.error" => {
                    let err = server_msg["message"].as_str().unwrap_or("unknown error");
                    tracing::warn!("Remote peer error: {}", err);
                }
                _ => {
                    tracing::debug!("Remote peer msg: {}", msg_type);
                }
            }
        }
    }

    // Cleanup
    forward_handle.abort();
    unregister_remote_peer(&remote_device_id).await;
    tracing::info!("Remote peer disconnected from {}", remote_url);

    Ok(())
}

/// Send a session command to a remote device.
/// Returns Ok(true) if sent, Ok(false) if no connection to that device.
pub async fn send_to_remote(device_id: &str, command: &str) -> Result<bool, String> {
    if let Some(tx) = get_remote_sender(device_id).await {
        tx.send(command.to_string())
            .map_err(|_| "Remote peer channel closed".to_string())?;
        Ok(true)
    } else {
        Ok(false)
    }
}
