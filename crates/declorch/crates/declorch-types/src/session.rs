//! Session data type — conversation history container.
//!
//! Pure data structure with no storage backend dependency.
//! The `SessionStore` (SQLite-backed) lives in `declorch-memory`.

use crate::agent::{AgentId, SessionId};
use crate::message::Message;

/// A conversation session with message history.
#[derive(Debug, Clone)]
pub struct Session {
    /// Session ID.
    pub id: SessionId,
    /// Owning agent ID.
    pub agent_id: AgentId,
    /// Conversation messages.
    pub messages: Vec<Message>,
    /// Estimated token count for the context window.
    pub context_window_tokens: u64,
    /// Optional human-readable session label.
    pub label: Option<String>,
}
