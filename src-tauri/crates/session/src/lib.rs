//! Unified Session class for AQBot.
//!
//! A `Session` bridges multi-input (local + remote) and multi-output (UI events
//! + file + subscribers). Inputs are FIFO-queued per session; each input gets
//! a handle that can be cancelled from the UI or by another session.

mod input;
mod manager;
mod record;
mod runner;

pub use input::{
    InputHandle, InputHandleInfo, InputRequest, InputSource, InputStatus,
};
pub use manager::{make_blocking_request, Session, SessionManager};
pub use record::{SessionRecord, SessionRecordRole, TokenCounts};
pub use runner::{InputRunner, InputRunContext};
