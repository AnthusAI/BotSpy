//! Shared world and helpers for the behavior specifications.

pub mod arguments_steps;
pub mod compaction_steps;
pub mod graph_steps;
pub mod inline_steps;
pub mod schema_steps;
pub mod session_steps;
pub mod status_steps;
pub mod usage_steps;

use botspy::{
    Adapter, Agent, FixtureAdapter, Message, Session, SessionStore, SessionSummary, UnknownSession,
};
use cucumber::World;
use std::collections::BTreeMap;
use std::sync::Arc;

#[derive(Debug, Default, World)]
pub struct BotSpyWorld {
    pub store: SessionStore,
    /// Fixture adapters keyed by agent name as written in the features.
    pub adapters: BTreeMap<String, Arc<FixtureAdapter>>,
    pub listed: Vec<SessionSummary>,
    pub opened: Option<Result<Session, UnknownSession>>,
    pub walked: Option<Result<Vec<Message>, UnknownSession>>,
    /// Session id the schema steps record into.
    pub current_session: Option<String>,
    /// Turn id of the most recently recorded turn, when a step made one.
    pub last_turn: Option<String>,
}

pub fn parse_agent(name: &str) -> Agent {
    match name {
        "claude_code" | "claude-code" => Agent::ClaudeCode,
        "cursor" => Agent::Cursor,
        "codex" => Agent::Codex,
        "grok_bot" | "grok-bot" => Agent::GrokBot,
        "antigravity" => Agent::Antigravity,
        other => panic!("unknown agent in feature: {other}"),
    }
}

/// Get or lazily create (and register) the fixture adapter for an agent.
pub fn adapter_for(world: &mut BotSpyWorld, agent: &str) -> Arc<FixtureAdapter> {
    if let Some(existing) = world.adapters.get(agent) {
        return existing.clone();
    }
    let adapter = Arc::new(FixtureAdapter::new(parse_agent(agent)));
    world.store.register(adapter.clone());
    world.adapters.insert(agent.to_string(), adapter.clone());
    adapter
}

/// Table rows below the header row.
pub fn data_rows(step: &cucumber::gherkin::Step) -> impl Iterator<Item = &[String]> {
    let rows: &[Vec<String>] = step
        .table
        .as_ref()
        .map(|table| table.rows.as_slice())
        .unwrap_or_default();
    rows.iter().skip(1).map(Vec::as_slice)
}

pub fn header(step: &cucumber::gherkin::Step) -> Vec<String> {
    step.table
        .as_ref()
        .map(|table| table.rows[0].clone())
        .unwrap_or_default()
}

/// Locate the adapter that owns a session id.
pub fn adapter_owning(world: &BotSpyWorld, id: &str) -> Option<Arc<FixtureAdapter>> {
    world
        .adapters
        .values()
        .find(|adapter| adapter.open(id).is_some())
        .cloned()
}

/// Load the current fixture session (by id) through its owning adapter.
pub fn current_session(world: &BotSpyWorld) -> Session {
    let id = world
        .current_session
        .as_deref()
        .expect("no current fixture session");
    find_session(world, id).expect("current session not found")
}

/// Find any fixture session by id across all fixture adapters.
pub fn find_session(world: &BotSpyWorld, id: &str) -> Option<Session> {
    world.adapters.values().find_map(|a| a.open(id))
}

/// Persist changes to the current fixture session.
pub fn save_session(world: &mut BotSpyWorld, session: Session) {
    let id = session.id.clone();
    let adapter = adapter_owning(world, &id).expect("current session not found");
    adapter.add_session(session);
}

/// Persist changes to a session looked up by id (any fixture adapter).
pub fn save_session_by_id(world: &mut BotSpyWorld, session: Session) {
    let id = session.id.clone();
    let adapter = adapter_owning(world, &id)
        .unwrap_or_else(|| panic!("no fixture adapter owns session {id}"));
    adapter.add_session(session);
}

/// The most recently recorded message of the current fixture session.
pub fn last_message(world: &BotSpyWorld) -> Message {
    current_session(world)
        .messages
        .last()
        .cloned()
        .expect("no messages recorded")
}

/// Record one more message on the current fixture session, refreshing
/// `last_activity_at` from the message timestamp when it has one.
pub fn record_message_on_current(world: &mut BotSpyWorld, message: Message) {
    let mut session = current_session(world);
    if !message.timestamp.is_empty() {
        session.last_activity_at = message.timestamp.clone();
    }
    session.messages.push(message);
    save_session(world, session);
}
