//! The unified agent-session history.

use crate::adapter::Adapter;
use crate::schema::{Agent, Message, Timestamp};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::fmt;
use std::sync::Arc;

/// One coding-agent session: ordered messages plus identifiers.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Session {
    pub id: String,
    pub agent: Agent,
    pub project_id: String,
    pub started_at: Timestamp,
    pub last_activity_at: Timestamp,
    pub messages: Vec<Message>,
}

/// A lightweight view of a session for listing across agents.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SessionSummary {
    pub id: String,
    pub agent: Agent,
    pub project_id: String,
    pub started_at: Timestamp,
    pub last_activity_at: Timestamp,
    pub message_count: usize,
}

impl From<&Session> for SessionSummary {
    fn from(session: &Session) -> Self {
        Self {
            id: session.id.clone(),
            agent: session.agent,
            project_id: session.project_id.clone(),
            started_at: session.started_at.clone(),
            last_activity_at: session.last_activity_at.clone(),
            message_count: session.messages.len(),
        }
    }
}

/// No registered adapter knows the requested session id.
#[derive(Debug, Clone, PartialEq)]
pub struct UnknownSession {
    pub id: String,
}

impl fmt::Display for UnknownSession {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "unknown session id: {}", self.id)
    }
}

impl std::error::Error for UnknownSession {}

/// The unified history over every registered adapter.
pub struct SessionStore {
    adapters: Vec<Arc<dyn Adapter>>,
}

impl SessionStore {
    pub fn new() -> Self {
        Self {
            adapters: Vec::new(),
        }
    }

    pub fn register(&mut self, adapter: Arc<dyn Adapter>) {
        self.adapters.push(adapter);
    }

    /// Sessions across all registered agents, sorted by last activity with
    /// the most recent first (ties broken by id for determinism).
    pub fn list_sessions(&self) -> Vec<SessionSummary> {
        let mut all: Vec<SessionSummary> = self
            .adapters
            .iter()
            .flat_map(|adapter| adapter.discover())
            .collect();
        all.sort_by(|a, b| {
            b.last_activity_at
                .cmp(&a.last_activity_at)
                .then_with(|| a.id.cmp(&b.id))
        });
        all
    }

    /// Open one session by id, from whichever agent owns it.
    pub fn open(&self, id: &str) -> Result<Session, UnknownSession> {
        self.adapters
            .iter()
            .find_map(|adapter| adapter.open(id))
            .ok_or_else(|| UnknownSession { id: id.to_string() })
    }

    /// The messages of a session in order.
    pub fn messages(&self, id: &str) -> Result<Vec<Message>, UnknownSession> {
        self.open(id).map(|session| session.messages)
    }
}

impl Default for SessionStore {
    fn default() -> Self {
        Self::new()
    }
}

impl fmt::Debug for SessionStore {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("SessionStore")
            .field("adapters", &self.adapters.len())
            .finish()
    }
}

/// Bucket used by adapters that own sessions keyed by id.
pub type SessionMap = BTreeMap<String, Session>;
