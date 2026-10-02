//! Shared world and helpers for the behavior specifications.

pub mod schema_steps;
pub mod session_steps;

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
