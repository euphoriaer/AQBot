//! Output record types - what `Session::emit_output` writes to its three sinks.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use serde_json::Value;

/// Token usage for a record (optional).
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct TokenCounts {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub input: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub output: Option<u64>,
}

/// Role of a session record. Matches agent loop semantics.
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SessionRecordRole {
    User,
    Assistant,
    Tool,
    System,
}

impl SessionRecordRole {
    pub fn as_str(&self) -> &'static str {
        match self {
            SessionRecordRole::User => "user",
            SessionRecordRole::Assistant => "assistant",
            SessionRecordRole::Tool => "tool",
            SessionRecordRole::System => "system",
        }
    }
}

/// A single record emitted by a Session. Written to:
/// 1. UI via Tauri events
/// 2. `.session` JSONL file
/// 3. Subscriber broadcast channel
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct SessionRecord {
    /// ISO 8601 timestamp.
    pub ts: DateTime<Utc>,
    /// Monotonically increasing sequence within the session.
    pub seq: u64,
    pub role: SessionRecordRole,
    /// Main text content (user prompt, assistant response, tool output, etc.).
    pub content: String,
    /// Optional thinking content (assistant only).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub thinking: Option<String>,
    /// Optional tool call id (tool records only).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tool_call_id: Option<String>,
    /// Optional tool name (tool records only).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tool: Option<String>,
    /// Optional structured tool input.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tool_input: Option<Value>,
    /// Optional token usage (assistant only).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tokens: Option<TokenCounts>,
}

impl SessionRecord {
    pub fn user(seq: u64, content: impl Into<String>) -> Self {
        Self {
            ts: Utc::now(),
            seq,
            role: SessionRecordRole::User,
            content: content.into(),
            thinking: None,
            tool_call_id: None,
            tool: None,
            tool_input: None,
            tokens: None,
        }
    }

    pub fn assistant(seq: u64, content: impl Into<String>) -> Self {
        Self {
            ts: Utc::now(),
            seq,
            role: SessionRecordRole::Assistant,
            content: content.into(),
            thinking: None,
            tool_call_id: None,
            tool: None,
            tool_input: None,
            tokens: None,
        }
    }
}
