//! Tauri commands for the unified Session class.
//!
//! Phase 1: provides `session_enqueue_input`, `session_cancel_input`,
//! `session_list_queue`, `session_cancel_active`. The `AgentInputRunner`
//! wraps the existing `agent_query` so the Session queue can drive it.

use crate::AppState;
use aqbot_core::types::AttachmentInput;
use aqbot_session::{
    make_blocking_request, InputHandle, InputHandleInfo, InputRunner, InputRunContext,
    InputSource, SessionManager, SessionRecord,
};
use async_trait::async_trait;
use serde::Deserialize;
use std::path::PathBuf;
use std::sync::Arc;
use tauri::{AppHandle, Emitter, Manager, State};

/// Emit a `session-queue-updated` event so the UI can refetch the queue for
/// the given conversation. Called after enqueue/cancel/agent-done transitions.
pub fn emit_queue_updated(app: &AppHandle, conversation_id: &str) {
    let _ = app.emit(
        "session-queue-updated",
        serde_json::json!({ "conversation_id": conversation_id }),
    );
}

/// Adapter that lets the Session crate call into the existing agent_query
/// logic. Holds an AppHandle so it can reach AppState via Tauri's state
/// container.
pub struct AgentInputRunner {
    pub app: AppHandle,
}

#[async_trait]
impl InputRunner for AgentInputRunner {
    async fn run(&self, ctx: InputRunContext) -> Result<(), String> {
        let app = self.app.clone();
        let conversation_id = ctx.conversation_id.clone();
        let prompt = ctx.content.clone();
        let attachments = ctx.attachments.clone();
        let provider_id = ctx.provider_id.clone();
        let model_id = ctx.model_id.clone();
        let cancel_token = ctx.cancel_token.clone();

        // Spawn the agent_query future in a task so we can race it against
        // cancellation. agent_query internally checks RUNNING_AGENTS and
        // returns "Agent is already running" if another query for the same
        // conversation is in flight - but the Session's queue guarantees
        // only one input runs at a time per conversation, so this check
        // always passes here.
        let query_app = app.clone();
        let query_handle = tokio::spawn(async move {
            let state = query_app.state::<AppState>();
            let app_clone = query_app.clone();
            crate::commands::agent::agent_query(
                app_clone,
                state,
                conversation_id,
                prompt,
                provider_id,
                model_id,
                Some(attachments),
            )
            .await
        });

        let mut query_handle = query_handle;
        tokio::select! {
            result = &mut query_handle => {
                match result {
                    Ok(Ok(())) => Ok(()),
                    Ok(Err(e)) => Err(e),
                    Err(join_err) => Err(format!("Agent task panicked: {}", join_err)),
                }
            }
            _ = cancel_token.cancelled() => {
                // Abort the spawned agent_query task to prevent it from
                // continuing in the background (which would leave
                // RUNNING_AGENTS held and block the next input).
                query_handle.abort();
                // Best-effort cancel: fire agent_cancel which triggers the
                // cancel token stored by agent_query in agent_cancel_tokens
                // and clears RUNNING_AGENTS / session registry state.
                let cancel_app = app.clone();
                let cancel_conv = ctx.conversation_id.clone();
                tokio::spawn(async move {
                    let state = cancel_app.state::<AppState>();
                    let _ =
                        crate::commands::agent::agent_cancel(state, cancel_conv).await;
                })
                .await
                .ok();
                Err("cancelled".to_string())
            }
        }
    }
}

/// Serializable enqueue request for `session_enqueue_input`.
#[derive(Debug, Deserialize)]
pub struct EnqueueInputRequest {
    pub conversation_id: String,
    pub content: String,
    #[serde(default)]
    pub attachments: Vec<AttachmentInput>,
    #[serde(default)]
    pub provider_id: Option<String>,
    #[serde(default)]
    pub model_id: Option<String>,
    /// True if this input is from a remote session peer. The Tauri layer uses
    /// this to populate `InputSource`. For Phase 1, all inputs are local;
    /// Phase 2 routes remote WS input directly here.
    #[serde(default)]
    pub remote: bool,
    /// Optional remote source address (only meaningful if `remote=true`).
    #[serde(default)]
    pub remote_device_id: Option<String>,
}

/// Enqueue an input. Returns immediately with a handle; the input runs when
/// the session's queue reaches it.
#[tauri::command]
pub async fn session_enqueue_input(
    app: AppHandle,
    state: State<'_, AppState>,
    request: EnqueueInputRequest,
) -> Result<InputHandle, String> {
    let manager = state.session_manager.clone();
    let provider_id = request.provider_id.unwrap_or_default();
    let model_id = request.model_id.unwrap_or_default();
    let conversation_id = request.conversation_id.clone();
    let source = if request.remote {
        InputSource::Remote {
            from: aqbot_gateway::session_registry::SessionAddress {
                device_id: request.remote_device_id.unwrap_or_default(),
                conversation_id: conversation_id.clone(),
            },
        }
    } else {
        InputSource::Local
    };
    let (req, _rx) = make_blocking_request(
        conversation_id.clone(),
        request.content,
        request.attachments,
        provider_id,
        model_id,
        source,
    );
    // Fire-and-forget enqueue. Callers that want to block on completion can
    // use a future `session_wait_for_done(handle_id)` command (Phase 4).
    let handle = manager.enqueue(req).await;
    emit_queue_updated(&app, &conversation_id);
    Ok(handle)
}

/// Cancel a queued or running input by handle_id. Returns true if found.
#[tauri::command]
pub async fn session_cancel_input(
    app: AppHandle,
    state: State<'_, AppState>,
    conversation_id: String,
    handle_id: String,
) -> Result<bool, String> {
    let cancelled = state
        .session_manager
        .cancel_input(&conversation_id, &handle_id)
        .await;
    emit_queue_updated(&app, &conversation_id);
    Ok(cancelled)
}

/// Cancel the currently-running input for a conversation. Returns true if
/// an active input was cancelled.
#[tauri::command]
pub async fn session_cancel_active(
    app: AppHandle,
    state: State<'_, AppState>,
    conversation_id: String,
) -> Result<bool, String> {
    let cancelled = state.session_manager.cancel_active(&conversation_id).await;
    emit_queue_updated(&app, &conversation_id);
    Ok(cancelled)
}

/// Cancel the running input AND all queued inputs for a conversation.
/// Returns the number of inputs cancelled.
#[tauri::command]
pub async fn session_cancel_all(
    app: AppHandle,
    state: State<'_, AppState>,
    conversation_id: String,
) -> Result<usize, String> {
    let count = state.session_manager.cancel_all(&conversation_id).await;
    emit_queue_updated(&app, &conversation_id);
    Ok(count)
}

/// List the queue (running + queued) for a conversation.
#[tauri::command]
pub async fn session_list_queue(
    state: State<'_, AppState>,
    conversation_id: String,
) -> Result<Vec<InputHandleInfo>, String> {
    Ok(state.session_manager.list_queue(&conversation_id).await)
}

/// Read session history from the `.session` file. Returns up to `limit`
/// most recent records (or all if limit is None).
#[tauri::command]
pub async fn session_get_history(
    state: State<'_, AppState>,
    conversation_id: String,
    limit: Option<usize>,
) -> Result<Vec<SessionRecord>, String> {
    state
        .session_manager
        .get_history(&conversation_id, limit)
        .map_err(|e| e.to_string())
}

/// Convenience constructor for AppState setup.
pub fn new_manager(app: AppHandle, app_data_dir: PathBuf) -> Arc<SessionManager> {
    let runner: Arc<dyn InputRunner> = Arc::new(AgentInputRunner { app: app.clone() });
    let sessions_dir = app_data_dir.join("sessions");
    // Callback fired whenever any session's queue state changes. Emits a
    // Tauri event so the UI can refetch `session_list_queue` for the active
    // conversation. Covers processor_loop transitions (Queued -> Running ->
    // Done/Failed/Cancelled) that the command-layer emits don't catch.
    let on_queue_changed: aqbot_session::QueueChangeCallback = Arc::new(move |conversation_id: &str| {
        emit_queue_updated(&app, conversation_id);
    });
    Arc::new(SessionManager::new(runner, sessions_dir, Some(on_queue_changed)))
}
