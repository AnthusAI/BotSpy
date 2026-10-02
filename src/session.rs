//! The unified agent-session history.

use crate::adapter::Adapter;
use crate::schema::{
    Agent, BattleLink, CompactionEvent, CompactionWindow, ForkPoint, Message, Peer, RateLimitState,
    SessionCost, SubagentInfo, Timestamp, Turn, Usage,
};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::fmt;
use std::sync::Arc;

/// One coding-agent session: ordered messages plus identifiers.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct Session {
    pub id: String,
    pub agent: Agent,
    pub project_id: String,
    pub started_at: Timestamp,
    pub last_activity_at: Timestamp,
    pub messages: Vec<Message>,
    /// Turn registry: turns referenced by the session's messages.
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub turns: BTreeMap<String, Turn>,
    /// Usage the agent persisted at thread/session scope.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub usage: Option<Usage>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub cost: Option<SessionCost>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub rate_limit: Option<RateLimitState>,
    /// Parent session, when this session is a child in the session graph.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub parent_id: Option<String>,
    /// The root of this session's graph subtree.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub root_id: Option<String>,
    /// Sub-agent identity, when this session was spawned as a sub-agent.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub subagent: Option<SubagentInfo>,
    /// Fork point, when this session forked from its parent.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub fork: Option<ForkPoint>,
    /// Best-of-N battle this session competes in.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub battle: Option<BattleLink>,
    /// Peer cloud agents with no local transcript.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub peers: Vec<Peer>,
    /// Compaction boundaries recorded by the agent.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub compactions: Vec<CompactionEvent>,
    /// Compaction window chain, keyed by window id (Codex).
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub compaction_windows: BTreeMap<String, CompactionWindow>,
    /// Legacy compaction hint (Cursor contextResiduals): tokens the agent
    /// reported still in context.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub residual_context_tokens: Option<u64>,
}

/// A lightweight view of a session for listing across agents.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct SessionSummary {
    pub id: String,
    pub agent: Agent,
    pub project_id: String,
    pub started_at: Timestamp,
    pub last_activity_at: Timestamp,
    pub message_count: usize,
}

impl Session {
    /// Usage accumulated over one turn: the sum of the usages its messages
    /// record. `None` when the turn has no message carrying usage.
    pub fn turn_usage(&self, turn_id: &str) -> Option<Usage> {
        let mut total = Usage::default();
        let mut any = false;
        for message in &self.messages {
            if message.turn_id.as_deref() == Some(turn_id) {
                if let Some(usage) = &message.usage {
                    total.add(usage);
                    any = true;
                }
            }
        }
        any.then_some(total)
    }

    /// Usage accumulated over the whole thread: the sum of the turn usages.
    pub fn thread_usage(&self) -> Option<Usage> {
        let mut total = Usage::default();
        let mut any = false;
        for turn_id in self.turns.keys() {
            if let Some(usage) = self.turn_usage(turn_id) {
                total.add(&usage);
                any = true;
            }
        }
        any.then_some(total)
    }
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
