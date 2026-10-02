//! The unified agent-session history.

use crate::adapter::Adapter;
use crate::schema::{
    Agent, BattleLink, CompactionEvent, CompactionWindow, ForkPoint, Message, PartialHistory, Peer,
    RateLimitState, SessionCost, SessionMetadata, SubagentInfo, Timestamp, Turn, Usage,
};
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};
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
    /// Optional metadata the agent recorded about the session.
    #[serde(default)]
    pub metadata: SessionMetadata,
    /// The session's local records cover only part of the real history.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub partial: Option<PartialHistory>,
    /// The transcript file's own mtime, when the agent's records carry no
    /// usable timestamps. Never fabricated into message timestamps.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub file_modified_at: Option<Timestamp>,
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
    /// The session's optional metadata, for listing across agents.
    #[serde(default)]
    pub metadata: SessionMetadata,
    /// Set when the session is only a partial view of the real history.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub partial: Option<PartialHistory>,
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
            metadata: session.metadata.clone(),
            partial: session.partial.clone(),
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
    /// the most recent first (ties broken by id for determinism). The same
    /// session reported by two discovery passes lists once: first report
    /// wins.
    pub fn list_sessions(&self) -> Vec<SessionSummary> {
        let mut all: Vec<SessionSummary> = self
            .adapters
            .iter()
            .flat_map(|adapter| adapter.discover())
            .collect();
        let mut seen = BTreeSet::new();
        all.retain(|summary| seen.insert(summary.id.clone()));
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

#[cfg(test)]
mod tests {
    use super::SessionStore;
    use crate::adapters::fixture::FixtureAdapter;
    use crate::session::Session;
    use std::sync::Arc;

    fn session(id: &str, last_activity: &str) -> Session {
        Session {
            id: id.to_string(),
            agent: crate::Agent::ClaudeCode,
            started_at: last_activity.to_string(),
            last_activity_at: last_activity.to_string(),
            ..Session::default()
        }
    }

    #[test]
    fn duplicate_reports_of_one_session_list_once() {
        let mut store = SessionStore::new();
        for _ in 0..2 {
            let adapter = Arc::new(FixtureAdapter::new(crate::Agent::Cursor));
            adapter.add_session(session("u9", "2026-10-01T09:00:00Z"));
            store.register(adapter);
        }
        let listed = store.list_sessions();
        assert_eq!(listed.len(), 1, "the same session listed twice");
        assert_eq!(listed[0].id, "u9");
    }

    #[test]
    fn listing_sorts_by_last_activity_most_recent_first() {
        let mut store = SessionStore::new();
        let adapter = Arc::new(FixtureAdapter::new(crate::Agent::Codex));
        adapter.add_session(session("u6", "2026-09-30T18:00:00Z"));
        adapter.add_session(session("u7", "2026-10-01T12:00:00Z"));
        adapter.add_session(session("u8", "2026-10-01T09:00:00Z"));
        store.register(adapter);
        let ids: Vec<String> = store.list_sessions().into_iter().map(|s| s.id).collect();
        assert_eq!(ids, vec!["u7", "u8", "u6"]);
    }
}
