//! Steps for error and status flags (spec 02_tool_result_status).

use crate::steps::{current_session, last_message, record_message_on_current, BotSpyWorld};
use botspy::{KnownPart, Message, Part, PartStatus, Role, Turn, TurnStatus};
use cucumber::{then, when};

fn parse_status(name: &str) -> PartStatus {
    PartStatus::parse(name).unwrap_or_else(|| panic!("unknown status in feature: {name}"))
}

fn ensure_turn(world: &mut BotSpyWorld) -> String {
    let mut session = current_session(world);
    let id = "turn-1".to_string();
    session.turns.entry(id.clone()).or_insert_with(|| Turn {
        id: id.clone(),
        ..Turn::default()
    });
    crate::steps::save_session(world, session);
    world.last_turn = Some(id.clone());
    id
}

fn update_last_turn<F: FnOnce(&mut Turn)>(world: &mut BotSpyWorld, f: F) {
    let turn_id = world.last_turn.clone().expect("no turn was recorded");
    let mut session = current_session(world);
    let turn = session
        .turns
        .get_mut(&turn_id)
        .expect("recorded turn vanished");
    f(turn);
    crate::steps::save_session(world, session);
}

#[when(regex = r#"I record a tool result for call "([^"]+)" that is an error with text "([^"]+)""#)]
fn record_error_tool_result(world: &mut BotSpyWorld, call_id: String, text: String) {
    record_message_on_current(
        world,
        Message {
            role: Role::Tool,
            parts: vec![Part::Known(KnownPart::ToolResult {
                call_id,
                text: Some(text),
                status: Some(PartStatus::Error),
                extra: None,
            })],
            ..Message::default()
        },
    );
}

#[when(
    regex = r#"I record a tool result for call "([^"]+)" that was interrupted with output "([^"]+)""#
)]
fn record_interrupted_tool_result(world: &mut BotSpyWorld, call_id: String, text: String) {
    record_message_on_current(
        world,
        Message {
            role: Role::Tool,
            parts: vec![Part::Known(KnownPart::ToolResult {
                call_id,
                text: Some(text),
                status: Some(PartStatus::Interrupted),
                extra: None,
            })],
            ..Message::default()
        },
    );
}

#[when(regex = r#"I record a tool call "([^"]+)" named "([^"]+)" with status "([^"]+)""#)]
fn record_tool_call_status(world: &mut BotSpyWorld, id: String, name: String, status: String) {
    record_message_on_current(
        world,
        Message {
            role: Role::Assistant,
            parts: vec![Part::Known(KnownPart::ToolCall {
                id,
                name,
                arguments: None,
                status: Some(parse_status(&status)),
                extra: None,
            })],
            ..Message::default()
        },
    );
}

#[when(regex = r#"I record a turn that was aborted with reason "([^"]+)""#)]
fn record_aborted_turn(world: &mut BotSpyWorld, reason: String) {
    ensure_turn(world);
    update_last_turn(world, |turn| {
        turn.status = Some(TurnStatus::Aborted);
        turn.aborted_reason = Some(reason);
    });
}

#[when(regex = r#"I record a turn that failed with error "([^"]+)""#)]
fn record_failed_turn(world: &mut BotSpyWorld, error: String) {
    ensure_turn(world);
    update_last_turn(world, |turn| {
        turn.status = Some(TurnStatus::Failed);
        turn.error = Some(error);
    });
}

#[when(regex = r#"I record an error event with text "([^"]+)""#)]
fn record_error_event(world: &mut BotSpyWorld, text: String) {
    record_message_on_current(
        world,
        Message {
            role: Role::System,
            is_error: true,
            parts: vec![Part::Known(KnownPart::System { text, extra: None })],
            ..Message::default()
        },
    );
}

#[when(regex = r#"I record a turn end marker with status "([^"]+)""#)]
fn record_turn_end_marker(world: &mut BotSpyWorld, status: String) {
    ensure_turn(world);
    let status = parse_status(&status);
    update_last_turn(world, move |turn| {
        turn.status = match status {
            PartStatus::Failed => Some(TurnStatus::Failed),
            PartStatus::Success | PartStatus::Ok => Some(TurnStatus::Completed),
            other => panic!("not a turn-end status: {other:?}"),
        };
    });
}

#[then(regex = r#"the last message has role "([^"]+)""#)]
fn last_message_role(world: &mut BotSpyWorld, role: String) {
    let expected = crate::steps::session_steps::parse_role(&role);
    let actual = last_message(world).role;
    assert_eq!(actual, expected, "unexpected role on the last message");
}

#[then(regex = r#"part ([0-9]+) has status "([^"]+)""#)]
fn part_has_status(world: &mut BotSpyWorld, index: usize, status: String) {
    let expected = parse_status(&status);
    let part = &last_message(world).parts[index - 1];
    let actual = match part {
        Part::Known(KnownPart::ToolCall { status, .. }) => *status,
        Part::Known(KnownPart::ToolResult { status, .. }) => *status,
        other => panic!("part {index} carries no status: {other:?}"),
    };
    assert_eq!(actual, Some(expected), "part {index} status");
}

#[then(regex = r#"the last message is marked as an error"#)]
fn last_message_is_error(world: &mut BotSpyWorld) {
    assert!(
        last_message(world).is_error,
        "the last message is not marked as an error"
    );
}

#[then(regex = r#"the turn is marked aborted with reason "([^"]+)""#)]
fn turn_aborted(world: &mut BotSpyWorld, reason: String) {
    let turn_id = world.last_turn.clone().expect("no turn was recorded");
    let turn = current_session(world)
        .turns
        .get(&turn_id)
        .expect("recorded turn vanished")
        .clone();
    assert_eq!(turn.status, Some(TurnStatus::Aborted));
    assert_eq!(turn.aborted_reason.as_deref(), Some(reason.as_str()));
}

#[then(regex = r#"the turn is marked failed with error "([^"]+)""#)]
fn turn_failed_with_error(world: &mut BotSpyWorld, error: String) {
    let turn_id = world.last_turn.clone().expect("no turn was recorded");
    let turn = current_session(world)
        .turns
        .get(&turn_id)
        .expect("recorded turn vanished")
        .clone();
    assert_eq!(turn.status, Some(TurnStatus::Failed));
    assert_eq!(turn.error.as_deref(), Some(error.as_str()));
}

#[then(regex = r#"the turn is marked failed with status "([^"]+)""#)]
fn turn_failed_with_status(world: &mut BotSpyWorld, status: String) {
    assert_eq!(parse_status(&status), PartStatus::Failed);
    let turn_id = world.last_turn.clone().expect("no turn was recorded");
    let turn = current_session(world)
        .turns
        .get(&turn_id)
        .expect("recorded turn vanished")
        .clone();
    assert_eq!(turn.status, Some(TurnStatus::Failed));
}

#[then(regex = r#"the turn is marked completed with status "([^"]+)""#)]
fn turn_completed(world: &mut BotSpyWorld, status: String) {
    assert_eq!(parse_status(&status), PartStatus::Success);
    let turn_id = world.last_turn.clone().expect("no turn was recorded");
    let turn = current_session(world)
        .turns
        .get(&turn_id)
        .expect("recorded turn vanished")
        .clone();
    assert_eq!(turn.status, Some(TurnStatus::Completed));
}
