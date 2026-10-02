//! Steps for turn grouping, timing, and errors (spec 04_turn_grouping).

use crate::steps::{current_session, last_message, record_message_on_current, BotSpyWorld};
use botspy::{Message, Turn, TurnPosition, TurnStatus};
use cucumber::{then, when};

fn ensure_turn(world: &mut BotSpyWorld, id: &str) {
    let mut session = current_session(world);
    session.turns.entry(id.to_string()).or_insert_with(|| Turn {
        id: id.to_string(),
        ..Turn::default()
    });
    crate::steps::save_session(world, session);
    world.last_turn = Some(id.to_string());
}

fn turn_by_id(world: &BotSpyWorld, id: &str) -> Turn {
    current_session(world)
        .turns
        .get(id)
        .cloned()
        .unwrap_or_else(|| panic!("turn {id} was never recorded"))
}

fn update_turn<F: FnOnce(&mut Turn)>(world: &mut BotSpyWorld, id: &str, f: F) {
    let mut session = current_session(world);
    let turn = session.turns.get_mut(id).expect("recorded turn vanished");
    f(turn);
    crate::steps::save_session(world, session);
}

#[when(regex = r#"^I record a user message in turn "([^"]+)"$"#)]
fn record_user_message_in_turn(world: &mut BotSpyWorld, turn_id: String) {
    ensure_turn(world, &turn_id);
    record_message_on_current(
        world,
        Message {
            role: botspy::Role::User,
            turn_id: Some(turn_id),
            ..Message::default()
        },
    );
}

#[when(regex = r#"I record an assistant reply in turn "([^"]+)""#)]
fn record_assistant_reply_in_turn(world: &mut BotSpyWorld, turn_id: String) {
    ensure_turn(world, &turn_id);
    record_message_on_current(
        world,
        Message {
            role: botspy::Role::Assistant,
            turn_id: Some(turn_id),
            ..Message::default()
        },
    );
}

#[when(
    regex = r#"I record a user message in turn "([^"]+)" at prompt index ([0-9]+) turn index ([0-9]+)"#
)]
fn record_user_message_with_position(
    world: &mut BotSpyWorld,
    turn_id: String,
    prompt_index: u64,
    turn_index: u64,
) {
    ensure_turn(world, &turn_id);
    record_message_on_current(
        world,
        Message {
            role: botspy::Role::User,
            turn_id: Some(turn_id),
            turn_position: Some(TurnPosition {
                prompt_index,
                turn_index,
            }),
            ..Message::default()
        },
    );
}

#[when(regex = r#"I record a sub-turn "([^"]+)" with parent turn "([^"]+)""#)]
fn record_sub_turn(world: &mut BotSpyWorld, id: String, parent: String) {
    ensure_turn(world, &parent);
    ensure_turn(world, &id);
    update_turn(world, &id, |turn| turn.parent_turn_id = Some(parent));
}

#[when(
    regex = r#"I record a completed turn "([^"]+)" with duration ([0-9]+) ms and time to first token ([0-9]+) ms"#
)]
fn record_completed_turn(world: &mut BotSpyWorld, id: String, duration_ms: u64, ttft_ms: u64) {
    ensure_turn(world, &id);
    update_turn(world, &id, |turn| {
        turn.status = Some(TurnStatus::Completed);
        turn.duration_ms = Some(duration_ms);
        turn.time_to_first_token_ms = Some(ttft_ms);
    });
}

#[when(
    regex = r#"I record a message in turn "([^"]+)" with step duration ([0-9]+) ms and turn duration ([0-9]+) ms"#
)]
fn record_message_with_step_and_turn_duration(
    world: &mut BotSpyWorld,
    turn_id: String,
    step_duration_ms: u64,
    turn_duration_ms: u64,
) {
    ensure_turn(world, &turn_id);
    record_message_on_current(
        world,
        Message {
            role: botspy::Role::Assistant,
            turn_id: Some(turn_id.clone()),
            step_duration_ms: Some(step_duration_ms),
            ..Message::default()
        },
    );
    update_turn(world, &turn_id, |turn| {
        turn.duration_ms = Some(turn_duration_ms);
    });
}

#[then(regex = r#"the last 2 messages share turn_id "([^"]+)""#)]
fn last_two_messages_share_turn(world: &mut BotSpyWorld, turn_id: String) {
    let messages = current_session(world).messages;
    let tail = &messages[messages.len() - 2..];
    for message in tail {
        assert_eq!(
            message.turn_id.as_deref(),
            Some(turn_id.as_str()),
            "message does not belong to turn {turn_id}"
        );
    }
}

#[then(regex = r#"^turn "([^"]+)" has parent_turn_id "([^"]+)"$"#)]
fn turn_has_parent(world: &mut BotSpyWorld, id: String, parent: String) {
    assert_eq!(
        turn_by_id(world, &id).parent_turn_id.as_deref(),
        Some(parent.as_str())
    );
}

#[then(regex = r#"turn "([^"]+)" has duration_ms ([0-9]+)"#)]
fn turn_has_duration(world: &mut BotSpyWorld, id: String, duration_ms: u64) {
    assert_eq!(turn_by_id(world, &id).duration_ms, Some(duration_ms));
}

#[then(regex = r#"turn "([^"]+)" has time_to_first_token_ms ([0-9]+)"#)]
fn turn_has_ttft(world: &mut BotSpyWorld, id: String, ttft_ms: u64) {
    assert_eq!(turn_by_id(world, &id).time_to_first_token_ms, Some(ttft_ms));
}

#[then(regex = r#"the last message records turn position prompt ([0-9]+) turn ([0-9]+)"#)]
fn last_message_turn_position(world: &mut BotSpyWorld, prompt_index: u64, turn_index: u64) {
    let actual = last_message(world)
        .turn_position
        .expect("the last message has no turn position");
    assert_eq!(actual.prompt_index, prompt_index);
    assert_eq!(actual.turn_index, turn_index);
}

#[then(regex = r#"the last message records step duration_ms ([0-9]+)"#)]
fn last_message_step_duration(world: &mut BotSpyWorld, step_duration_ms: u64) {
    assert_eq!(last_message(world).step_duration_ms, Some(step_duration_ms));
}
