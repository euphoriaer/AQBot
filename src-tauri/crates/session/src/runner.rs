//! InputRunner trait - the abstraction the Session queue calls to execute one input.
//!
//! The Tauri layer implements this to call into the existing agent execution
//! logic. Keeping it as a trait means the Session crate has no dependency on
//! Tauri / SeaORM / the agent SDK, so it stays unit-testable.

use async_trait::async_trait;
use std::sync::Arc;

use crate::input::{InputRequest, InputSource};

/// Context passed to the runner. Contains everything the runner needs to
/// execute an input: conversation_id, content, provider/model, source, and
/// a cancellation token.
#[derive(Clone, Debug)]
pub struct InputRunContext {
    pub handle_id: String,
    pub conversation_id: String,
    pub content: String,
    pub provider_id: String,
    pub model_id: String,
    pub source: InputSource,
    pub cancel_token: tokio_util::sync::CancellationToken,
}

impl InputRunContext {
    pub fn from_request(req: &InputRequest) -> Self {
        Self {
            handle_id: req.handle_id.clone(),
            conversation_id: req.conversation_id.clone(),
            content: req.content.clone(),
            provider_id: req.provider_id.clone(),
            model_id: req.model_id.clone(),
            source: req.source.clone(),
            cancel_token: req.cancel_token.clone(),
        }
    }
}

/// Executes one input. Implemented by the Tauri layer.
#[async_trait]
pub trait InputRunner: Send + Sync {
    /// Run the given input to completion. Return `Err` on failure.
    ///
    /// The runner is responsible for:
    /// - Persisting user message
    /// - Spawning the agent
    /// - Emitting UI events (agent-stream-text, agent-done, etc.)
    /// - Persisting assistant message
    /// - Honoring the cancel token
    async fn run(&self, ctx: InputRunContext) -> Result<(), String>;
}

/// Convenience type alias.
pub type SharedInputRunner = Arc<dyn InputRunner>;
