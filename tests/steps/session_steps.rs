//! Steps for the unified session history (layer 1).

use crate::steps::{adapter_for, adapter_owning, data_rows, BotSpyWorld};
use botspy::{Adapter, KnownPart, Part, Role, Session};
use cucumber::{given, then, when};

pub fn parse_role(name: &str) -> Role {
    match name {
        "user" => Role::User,
        "assistant" => Role::Assistant,
        "system" => Role::System,
        "tool" => Role::Tool,
        other => panic!("unknown role in feature: {other}"),
    }
}

#[given(regex = r#"the fixture adapter for agent "([^"]+)" with sessions:"#)]
fn fixture_sessions(world: &mut BotSpyWorld, agent: String, step: &cucumber::gherkin::Step) {
    let adapter = adapter_for(world, &agent);
    for row in data_rows(step) {
        adapter.add_session(Session {
            id: row[0].clone(),
            agent: crate::steps::parse_agent(&agent),
            project_id: row[1].clone(),
            started_at: row[2].clone(),
            last_activity_at: row[3].clone(),
            messages: Vec::new(),
        });
    }
}

#[given(regex = r#"the session "([^"]+)" has messages:"#)]
fn session_messages(world: &mut BotSpyWorld, id: String, step: &cucumber::gherkin::Step) {
    let adapter = adapter_owning(world, &id).expect("session not found in fixture adapters");
    let mut session = adapter.open(&id).expect("session vanished");
    session.messages = data_rows(step)
        .map(|row| botspy::Message {
            role: parse_role(&row[0]),
            timestamp: row[1].clone(),
            parts: vec![Part::Known(KnownPart::Text {
                text: row[2].clone(),
                extra: None,
            })],
            provenance: None,
            extra: None,
        })
        .collect();
    adapter.add_session(session);
}

#[when(regex = r"^I list sessions$")]
fn list_sessions(world: &mut BotSpyWorld) {
    world.listed = world.store.list_sessions();
}

#[when(regex = r#"I open session "([^"]+)""#)]
fn open_session(world: &mut BotSpyWorld, id: String) {
    world.opened = Some(world.store.open(&id));
}

#[when(regex = r#"I walk the messages of session "([^"]+)""#)]
fn walk_messages(world: &mut BotSpyWorld, id: String) {
    world.walked = Some(world.store.messages(&id));
}

#[then(regex = r#"the sessions are listed in order "([^"]+)""#)]
fn listed_in_order(world: &mut BotSpyWorld, expected: String) {
    let expected: Vec<&str> = expected.split(", ").collect();
    let actual: Vec<String> = world.listed.iter().map(|s| s.id.clone()).collect();
    assert_eq!(actual, expected, "sessions not in expected order");
}

#[then(regex = r#"session "([^"]+)" is listed with agent "([^"]+)" and project "([^"]+)""#)]
fn listed_with_agent_and_project(
    world: &mut BotSpyWorld,
    id: String,
    agent: String,
    project: String,
) {
    let summary = world
        .listed
        .iter()
        .find(|s| s.id == id)
        .unwrap_or_else(|| panic!("session {id} not listed"));
    assert_eq!(summary.agent, crate::steps::parse_agent(&agent));
    assert_eq!(summary.project_id, project);
}

#[then(
    regex = r#"opening session "([^"]+)" succeeds with agent "([^"]+)", project "([^"]+)" and ([0-9]+) messages"#
)]
fn opening_succeeds(
    world: &mut BotSpyWorld,
    id: String,
    agent: String,
    project: String,
    count: usize,
) {
    let session = world
        .opened
        .as_ref()
        .expect("no session was opened")
        .as_ref()
        .unwrap_or_else(|err| panic!("opening {id} failed: {err}"));
    assert_eq!(session.id, id);
    assert_eq!(session.agent, crate::steps::parse_agent(&agent));
    assert_eq!(session.project_id, project);
    assert_eq!(session.messages.len(), count);
}

#[then(regex = r#"walking its messages yields roles "([^"]+)" in order"#)]
fn walked_roles(world: &mut BotSpyWorld, expected: String) {
    let expected: Vec<Role> = expected.split(", ").map(parse_role).collect();
    let messages = world
        .walked
        .as_ref()
        .expect("messages were not walked")
        .as_ref()
        .expect("walking failed");
    let actual: Vec<Role> = messages.iter().map(|m| m.role).collect();
    assert_eq!(actual, expected);
}

#[then(regex = r#"message ([0-9]+) has a text part "([^"]+)""#)]
fn message_text_part(world: &mut BotSpyWorld, index: usize, text: String) {
    let messages = world
        .walked
        .as_ref()
        .expect("messages were not walked")
        .as_ref()
        .expect("walking failed");
    let message = &messages[index - 1];
    let found = message.parts.iter().any(|part| {
        matches!(
            part,
            Part::Known(KnownPart::Text { text: t, extra: None }) if t == &text
        )
    });
    assert!(found, "message {index} has no text part {text:?}");
}

#[then(regex = r#"an unknown-session error is reported for "([^"]+)""#)]
fn unknown_session_error(world: &mut BotSpyWorld, id: String) {
    let opened = world.opened.as_ref().expect("no session was opened");
    match opened {
        Err(err) => assert_eq!(err.id, id),
        Ok(_) => panic!("expected an unknown-session error for {id}"),
    }
}
