//! Steps for the unified history across adapters
//! (spec 01_structure/unified_history.feature).

use crate::steps::{parse_agent, BotSpyWorld};
use botspy::adapters::fixture::FixtureAdapter;
use botspy::{KnownPart, Message, Part, Provenance, Role, Session};
use cucumber::{given, then, when};
use std::sync::Arc;

fn plain_session(id: &str, agent: &str, project: &str, last_activity: &str) -> Session {
    Session {
        id: id.to_string(),
        agent: parse_agent(agent),
        project_id: project.to_string(),
        started_at: last_activity.to_string(),
        last_activity_at: last_activity.to_string(),
        ..Session::default()
    }
}

fn add_to_store(world: &mut BotSpyWorld, agent: &str, session: Session) {
    let adapter = Arc::new(FixtureAdapter::new(parse_agent(agent)));
    adapter.add_session(session);
    world.store.register(adapter);
}

/// The quoted tokens of `"u1", "u2", and "u3"` in order: everything between
/// double quotes, which is exactly the ids.
fn quoted_list(text: &str) -> Vec<String> {
    text.split('"')
        .skip(1)
        .step_by(2)
        .map(str::to_string)
        .collect()
}

#[given(
    regex = r#"^fixture sessions "([^"]+)" from "([^"]+)", "([^"]+)" from "([^"]+)", "([^"]+)" from "([^"]+)", "([^"]+)" from "([^"]+)", and "([^"]+)" from "([^"]+)"$"#
)]
#[allow(clippy::too_many_arguments)]
fn fixture_sessions_from_five_agents(
    world: &mut BotSpyWorld,
    id1: String,
    agent1: String,
    id2: String,
    agent2: String,
    id3: String,
    agent3: String,
    id4: String,
    agent4: String,
    id5: String,
    agent5: String,
) {
    for (id, agent) in [
        (&id1, &agent1),
        (&id2, &agent2),
        (&id3, &agent3),
        (&id4, &agent4),
        (&id5, &agent5),
    ] {
        add_to_store(
            world,
            agent,
            plain_session(id, agent, "", "2026-10-01T09:00:00Z"),
        );
    }
}

#[given(
    regex = r#"^fixture sessions "([^"]+)" last active at "([^"]+)", "([^"]+)" last active at "([^"]+)", and "([^"]+)" last active at "([^"]+)" from "([^"]+)"$"#
)]
#[allow(clippy::too_many_arguments)]
fn fixture_sessions_last_active(
    world: &mut BotSpyWorld,
    id1: String,
    ts1: String,
    id2: String,
    ts2: String,
    id3: String,
    ts3: String,
    agent: String,
) {
    for (id, ts) in [(&id1, &ts1), (&id2, &ts2), (&id3, &ts3)] {
        add_to_store(world, &agent, plain_session(id, &agent, "", ts));
    }
}

#[given(regex = r#"^fixture session "([^"]+)" from "([^"]+)" reported by two discovery passes$"#)]
fn fixture_session_twice(world: &mut BotSpyWorld, id: String, agent: String) {
    for _ in 0..2 {
        add_to_store(
            world,
            &agent,
            plain_session(&id, &agent, "", "2026-10-01T09:00:00Z"),
        );
    }
}

#[given(
    regex = r#"^fixture session "([^"]+)" from "([^"]+)" whose message has provenance "([^"]+)" line ([0-9]+)$"#
)]
fn fixture_session_with_provenance(
    world: &mut BotSpyWorld,
    id: String,
    agent: String,
    file: String,
    line: usize,
) {
    let mut session = plain_session(&id, &agent, "", "2026-10-01T09:00:00Z");
    session.messages.push(Message {
        role: Role::User,
        parts: vec![Part::Known(KnownPart::Text {
            text: "merge me".to_string(),
            extra: None,
        })],
        provenance: Some(Provenance {
            source_file: file,
            line: Some(line as u64),
            ..Provenance::default()
        }),
        ..Message::default()
    });
    add_to_store(world, &agent, session);
}

#[given(
    regex = r#"^fixture sessions "([^"]+)" from "([^"]+)" in project "([^"]+)" and "([^"]+)" from "([^"]+)" in project "([^"]+)"$"#
)]
#[allow(clippy::too_many_arguments)]
fn fixture_sessions_with_projects(
    world: &mut BotSpyWorld,
    id1: String,
    agent1: String,
    project1: String,
    id2: String,
    agent2: String,
    project2: String,
) {
    add_to_store(
        world,
        &agent1,
        plain_session(&id1, &agent1, &project1, "2026-10-01T09:00:00Z"),
    );
    add_to_store(
        world,
        &agent2,
        plain_session(&id2, &agent2, &project2, "2026-10-01T09:05:00Z"),
    );
}

#[when(regex = r#"^the unified history is listed$"#)]
fn unified_history_listed(world: &mut BotSpyWorld) {
    world.listed = world.store.list_sessions();
}

#[when(regex = r#"^the unified history is opened for session "([^"]+)"$"#)]
fn unified_history_opened(world: &mut BotSpyWorld, id: String) {
    world.opened = Some(world.store.open(&id));
}

#[then(regex = r#"^it yields ([0-9]+) sessions?$"#)]
fn yields_count(world: &mut BotSpyWorld, count: usize) {
    assert_eq!(
        world.listed.len(),
        count,
        "the unified history listed the wrong number of sessions"
    );
}

#[then(regex = r#"^it yields sessions (.+)$"#)]
fn yields_ids(world: &mut BotSpyWorld, expected: String) {
    let expected = quoted_list(&expected);
    let actual: Vec<String> = world.listed.iter().map(|s| s.id.clone()).collect();
    assert_eq!(
        actual, expected,
        "the unified history listed the wrong sessions"
    );
}

#[then(regex = r#"^the order is (.+)$"#)]
fn order_is(world: &mut BotSpyWorld, expected: String) {
    let expected = quoted_list(&expected);
    let actual: Vec<String> = world.listed.iter().map(|s| s.id.clone()).collect();
    assert_eq!(actual, expected, "the unified history is out of order");
}

#[then(regex = r#"^it yields session "([^"]+)" once$"#)]
fn yields_once(world: &mut BotSpyWorld, id: String) {
    let count = world.listed.iter().filter(|s| s.id == id).count();
    assert_eq!(count, 1, "session {id} was listed {count} times");
}

#[then(regex = r#"^that message's provenance still names "([^"]+)" line ([0-9]+)$"#)]
fn provenance_survives(world: &mut BotSpyWorld, file: String, line: u64) {
    let session = world
        .opened
        .as_ref()
        .expect("no session was opened")
        .as_ref()
        .expect("opening the session failed");
    let message = session
        .messages
        .iter()
        .find(|message| message.provenance.is_some())
        .expect("no message with provenance");
    let provenance = message.provenance.as_ref().expect("no provenance");
    assert_eq!(provenance.source_file, file, "provenance lost its file");
    assert_eq!(provenance.line, Some(line), "provenance lost its line");
}

#[then(regex = r#"^session "([^"]+)" carries agent "([^"]+)" and project "([^"]+)"$"#)]
fn carries_agent_and_project(world: &mut BotSpyWorld, id: String, agent: String, project: String) {
    let summary = world
        .listed
        .iter()
        .find(|s| s.id == id)
        .unwrap_or_else(|| panic!("session {id} not listed"));
    assert_eq!(summary.agent, parse_agent(&agent));
    assert_eq!(summary.project_id, project);
}
