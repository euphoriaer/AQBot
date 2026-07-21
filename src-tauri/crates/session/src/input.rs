//! Input data structures for the Session queue.

use aqbot_gateway::session_registry::SessionAddress;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tokio::sync::{oneshot, RwLock};
use tokio_util::sync::CancellationToken;

/// Where an input originated.
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(tag = "kind", content = "detail")]
pub enum InputSource {
    /// Input from the local UI / user.
    Local,
    /// Input from a remote session peer.
    Remote { from: SessionAddress },
}

/// Lifecycle status of a queued input.
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(tag = "state", content = "detail")]
pub enum InputStatus {
    /// Waiting in the queue. The number is the 0-indexed queue position.
    Queued(usize),
    /// Currently being processed by the agent.
    Running,
    /// Finished successfully.
    Done,
    /// Cancelled before or during execution.
    Cancelled,
    /// Execution failed. The string is the error message.
    Failed(String),
}

/// A single input request enqueued into a Session.
///
/// `status` and `cancel_token` are shared so that the queue processor and
/// external callers (cancel/list) can observe and mutate state.
pub struct InputRequest {
    /// Unique identifier for this input (UUID).
    pub handle_id: String,
    /// Target conversation / session.
    pub conversation_id: String,
    /// The user's prompt content.
    pub content: String,
    /// Provider/model override (empty string = use conversation default).
    pub provider_id: String,
    pub model_id: String,
    /// Where this input came from.
    pub source: InputSource,
    /// When the input was enqueued (UTC).
    pub enqueued_at: DateTime<Utc>,
    /// Shared status. Updated by the queue processor and read by list_queue.
    pub status: Arc<RwLock<InputStatus>>,
    /// Cancellation token. Triggered by `cancel_input` to abort a running input
    /// or remove a queued one.
    pub cancel_token: CancellationToken,
    /// Optional one-shot for blocking callers that want to wait for completion.
    /// `None` means fire-and-forget. Wrapped in a Mutex so the queue processor
    /// can take it out when the input finishes.
    pub done_tx: Arc<std::sync::Mutex<Option<oneshot::Sender<Result<(), String>>>>>,
}

impl InputRequest {
    pub fn new(
        conversation_id: String,
        content: String,
        provider_id: String,
        model_id: String,
        source: InputSource,
    ) -> Self {
        Self {
            handle_id: uuid::Uuid::new_v4().to_string(),
            conversation_id,
            content,
            provider_id,
            model_id,
            source,
            enqueued_at: Utc::now(),
            status: Arc::new(RwLock::new(InputStatus::Queued(0))),
            cancel_token: CancellationToken::new(),
            done_tx: Arc::new(std::sync::Mutex::new(None)),
        }
    }
}

/// Lightweight handle returned to callers of `enqueue`.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct InputHandle {
    pub handle_id: String,
    pub conversation_id: String,
}

/// Snapshot of an input's state for UI display.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct InputHandleInfo {
    pub handle_id: String,
    pub conversation_id: String,
    pub source: InputSource,
    pub status: InputStatus,
    /// First 80 chars of content for preview.
    pub preview: String,
    pub enqueued_at: DateTime<Utc>,
}
