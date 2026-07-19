use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};
use std::sync::Arc;
use tokio::sync::{mpsc, RwLock};

/// Address of an agent session. Internally stores device_id + conversation_id
/// for routing; the public wire format is `host:port/conversation_id`.
#[derive(Debug, Clone, Hash, Eq, PartialEq, Serialize, Deserialize)]
pub struct SessionAddress {
    pub device_id: String,
    pub conversation_id: String,
}

impl SessionAddress {
    /// Internal wire format: `device_id/conversation_id` (used for lock
    /// comparison and internal routing — not user-facing).
    pub fn to_wire(&self) -> String {
        format!("{}/{}", self.device_id, self.conversation_id)
    }

    /// Parse from wire. Supports two formats:
    /// - New: `host:port/conversation_id` (device_id = "host:port")
    /// - Legacy: `device_id/conversation_id`
    pub fn from_wire(s: &str) -> Option<Self> {
        // Split on the LAST '/' — conversation_id never contains '/',
        // but host:port doesn't either, so this handles both formats.
        let slash_pos = s.rfind('/')?;
        let first = &s[..slash_pos];
        let second = &s[slash_pos + 1..];
        if first.is_empty() || second.is_empty() {
            return None;
        }
        Some(Self {
            device_id: first.to_string(),
            conversation_id: second.to_string(),
        })
    }
}

/// Information about a registered session (sent to clients)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SessionInfo {
    /// Public address in `host:port/conversation_id` format.
    pub address: String,
    pub device_id: String,
    pub conversation_id: String,
    pub lock_held: bool,
    pub registered_at: i64,
    /// Whether this session has an active agent / external connection.
    #[serde(default)]
    pub is_active: bool,
    /// Conversation title from DB (None for external WS-only sessions).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub title: Option<String>,
}

/// Handle for a registered session — holds the output broadcast channel
/// and input lock state.
pub struct SessionHandle {
    pub address: SessionAddress,
    /// Sender for forwarding remote input into this session's agent loop.
    /// Tuple of (source_address, content).
    pub output_tx: mpsc::Sender<(String, String)>,
    /// Who currently holds the input lock (None = available).
    pub input_lock_holder: RwLock<Option<SessionAddress>>,
    pub conversation_title: RwLock<String>,
    pub registered_at: i64,
}

/// Shared registry of connected sessions, accessible from both the Gateway
/// axum handlers and the Tauri command layer.
pub struct SessionRegistry {
    sessions: RwLock<HashMap<SessionAddress, Arc<SessionHandle>>>,
    /// Bidirectional session connections: who is connected to whom.
    connections: RwLock<HashMap<SessionAddress, HashSet<SessionAddress>>>,
    /// device_id → gateway URL for cross-device forwarding.
    device_peers: RwLock<HashMap<String, String>>,
    /// Gateway client-visible address: `host:port` (e.g. "127.0.0.1:8080").
    /// Set before the gateway starts; used to build public session addresses.
    gateway_addr: RwLock<Option<String>>,
}

impl SessionRegistry {
    pub fn new() -> Self {
        Self {
            sessions: RwLock::new(HashMap::new()),
            connections: RwLock::new(HashMap::new()),
            device_peers: RwLock::new(HashMap::new()),
            gateway_addr: RwLock::new(None),
        }
    }

    /// Set the gateway's client-visible address (host:port) so that session
    /// addresses are formatted as `host:port/conversation_id`.
    pub async fn set_gateway_address(&self, host: &str, port: u16) {
        *self.gateway_addr.write().await = Some(format!("{}:{}", host, port));
    }

    /// Get the gateway client address, if set.
    pub async fn gateway_address(&self) -> Option<String> {
        self.gateway_addr.read().await.clone()
    }

    /// Build the public address string for a conversation.
    /// Format: `host:port/conversation_id` when gateway address is known,
    /// falls back to `device_id/conversation_id`.
    async fn build_address(&self, device_id: &str, conversation_id: &str) -> String {
        if let Some(ref gw) = *self.gateway_addr.read().await {
            format!("{}/{}", gw, conversation_id)
        } else {
            format!("{}/{}", device_id, conversation_id)
        }
    }

    pub async fn register(
        &self,
        address: SessionAddress,
        output_tx: mpsc::Sender<(String, String)>,
    ) -> Arc<SessionHandle> {
        let handle = Arc::new(SessionHandle {
            address: address.clone(),
            output_tx,
            input_lock_holder: RwLock::new(None),
            conversation_title: RwLock::new(String::new()),
            registered_at: chrono::Utc::now().timestamp(),
        });
        self.sessions
            .write()
            .await
            .insert(address, handle.clone());
        handle
    }

    pub async fn unregister(&self, address: &SessionAddress) {
        self.sessions.write().await.remove(address);
    }

    pub async fn get(&self, address: &SessionAddress) -> Option<Arc<SessionHandle>> {
        self.sessions.read().await.get(address).cloned()
    }

    /// List active sessions (those with running agents or external
    /// connections). Use `list_all()` to include inactive DB conversations.
    pub async fn list(&self) -> Vec<SessionInfo> {
        let sessions = self.sessions.read().await;
        let mut infos: Vec<SessionInfo> = Vec::with_capacity(sessions.len());
        for (addr, handle) in sessions.iter() {
            let lock_held = handle.input_lock_holder.try_read().map(|l| l.is_some()).unwrap_or(true);
            let address = self.build_address(&addr.device_id, &addr.conversation_id).await;
            let title = handle.conversation_title.read().await.clone();
            infos.push(SessionInfo {
                address,
                device_id: addr.device_id.clone(),
                conversation_id: addr.conversation_id.clone(),
                lock_held,
                registered_at: handle.registered_at,
                is_active: true,
                title: if title.is_empty() { None } else { Some(title) },
            });
        }
        infos.sort_by(|a, b| b.registered_at.cmp(&a.registered_at));
        infos
    }

    /// Returns the set of conversation IDs that are currently active.
    pub async fn active_conversation_ids(&self) -> std::collections::HashSet<String> {
        self.sessions
            .read()
            .await
            .keys()
            .map(|a| a.conversation_id.clone())
            .collect()
    }

    pub async fn acquire_lock(
        &self,
        address: &SessionAddress,
        requester: &SessionAddress,
    ) -> Result<(), String> {
        let handle = self
            .get(address)
            .await
            .ok_or_else(|| "Session not found".to_string())?;
        let mut lock = handle.input_lock_holder.write().await;
        if let Some(ref holder) = *lock {
            if holder != requester {
                return Err(format!("Input lock held by {}", holder.to_wire()));
            }
        }
        *lock = Some(requester.clone());
        Ok(())
    }

    pub async fn release_lock(
        &self,
        address: &SessionAddress,
        holder: &SessionAddress,
    ) {
        if let Some(handle) = self.get(address).await {
            let mut lock = handle.input_lock_holder.write().await;
            if lock.as_ref() == Some(holder) {
                *lock = None;
            }
        }
    }

    pub async fn set_title(&self, address: &SessionAddress, title: String) {
        if let Some(handle) = self.get(address).await {
            *handle.conversation_title.write().await = title;
        }
    }

    /// Merge active registry sessions with all DB conversations so every
    /// sidebar conversation appears as connectable, not just those with
    /// running agents.
    /// `db_conversations` is a list of (conversation_id, title) tuples.
    /// `this_device_id` is used as fallback when gateway address is not set.
    pub async fn list_with_db(
        &self,
        db_conversations: &[(String, String)],
        this_device_id: &str,
    ) -> Vec<SessionInfo> {
        let active_sessions = self.list().await;
        let active_ids = self.active_conversation_ids().await;
        let gw_addr = self.gateway_addr.read().await.clone();

        let mut result = Vec::with_capacity(db_conversations.len() + active_sessions.len());

        for (conv_id, title) in db_conversations {
            let is_active = active_ids.contains(conv_id);
            let active_info = active_sessions.iter().find(|s| &s.conversation_id == conv_id);

            let address = if let Some(ref gw) = gw_addr {
                format!("{}/{}", gw, conv_id)
            } else {
                format!("{}/{}", this_device_id, conv_id)
            };

            result.push(SessionInfo {
                address,
                device_id: this_device_id.to_string(),
                conversation_id: conv_id.clone(),
                lock_held: active_info.map(|s| s.lock_held).unwrap_or(false),
                registered_at: active_info.map(|s| s.registered_at).unwrap_or(0),
                is_active,
                title: if title.is_empty() {
                    None
                } else {
                    Some(title.clone())
                },
            });
        }

        // Include external WS-only sessions (not in DB)
        for s in &active_sessions {
            if !result.iter().any(|r| r.conversation_id == s.conversation_id) {
                result.push(s.clone());
            }
        }

        result.sort_by(|a, b| {
            b.is_active
                .cmp(&a.is_active)
                .then_with(|| b.registered_at.cmp(&a.registered_at))
        });

        result
    }

    // ── Session connection tracking ──────────────────────────────────────

    /// Record a bidirectional connection between two sessions.
    /// Output from either session will be forwarded to the other.
    pub async fn connect_sessions(&self, a: &SessionAddress, b: &SessionAddress) {
        let mut conns = self.connections.write().await;
        conns.entry(a.clone()).or_default().insert(b.clone());
        conns.entry(b.clone()).or_default().insert(a.clone());
    }

    /// Remove a bidirectional connection between two sessions.
    pub async fn disconnect_sessions(&self, a: &SessionAddress, b: &SessionAddress) {
        let mut conns = self.connections.write().await;
        if let Some(set) = conns.get_mut(a) {
            set.remove(b);
            if set.is_empty() {
                conns.remove(a);
            }
        }
        if let Some(set) = conns.get_mut(b) {
            set.remove(a);
            if set.is_empty() {
                conns.remove(b);
            }
        }
    }

    /// Return all session addresses connected to the given address.
    pub async fn get_connections(&self, address: &SessionAddress) -> Vec<SessionAddress> {
        self.connections
            .read()
            .await
            .get(address)
            .map(|s| s.iter().cloned().collect())
            .unwrap_or_default()
    }

    /// Non-blocking version of get_connections for use in hot paths
    /// (e.g. streaming output broadcast). Returns empty vec if lock
    /// is contended.
    pub fn try_get_connections(&self, address: &SessionAddress) -> Vec<SessionAddress> {
        self.connections
            .try_read()
            .ok()
            .and_then(|conns| conns.get(address).map(|s| s.iter().cloned().collect()))
            .unwrap_or_default()
    }

    /// Return SessionInfo for all sessions connected to the given address,
    /// filtered through the DB conversation list for titles.
    pub async fn get_connections_info(
        &self,
        address: &SessionAddress,
        db_conversations: &[(String, String)],
        this_device_id: &str,
    ) -> Vec<SessionInfo> {
        let connected = self.get_connections(address).await;
        let all_with_db = self.list_with_db(db_conversations, this_device_id).await;
        let connected_ids: HashSet<_> = connected.iter().map(|a| &a.conversation_id).collect();
        all_with_db
            .into_iter()
            .filter(|s| connected_ids.contains(&s.conversation_id))
            .collect()
    }

    // ── Cross-device peer management ──────────────────────────────────────

    pub async fn register_device_peer(&self, device_id: String, url: String) {
        self.device_peers.write().await.insert(device_id, url);
    }

    pub async fn get_device_peer(&self, device_id: &str) -> Option<String> {
        self.device_peers.read().await.get(device_id).cloned()
    }
}
