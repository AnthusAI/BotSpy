//! Steps for compaction and summaries (spec 05_compaction).

use crate::steps::{current_session, last_message, save_session, BotSpyWorld};
use botspy::{CompactionEvent, CompactionWindow, KnownPart, Message, Part, Role};
use cucumber::{then, when};

fn message_id_of(message: &Message) -> String {
    message
        .id
        .clone()
        .unwrap_or_else(|| format!("msg-{}", message.timestamp))
}

#[when(
    regex = r#"I record a compaction boundary with pre tokens ([0-9]+) and post tokens ([0-9]+)"#
)]
fn record_compaction_boundary(world: &mut BotSpyWorld, pre: u64, post: u64) {
    let mut session = current_session(world);
    let parent = Message {
        id: Some("pre-compaction-context".to_string()),
        role: Role::Assistant,
        timestamp: "2026-10-01T09:05:00Z".to_string(),
        parts: vec![Part::Known(KnownPart::Text {
            text: "context about to be compacted".to_string(),
            extra: None,
        })],
        ..Message::default()
    };
    let parent_id = message_id_of(&parent);
    session.messages.push(parent);
    session.compactions.push(CompactionEvent {
        pre_tokens: Some(pre),
        post_tokens: Some(post),
        logical_parent_message_id: Some(parent_id),
    });
    save_session(world, session);
}

#[when(regex = r#"I record a compaction summary message with text "([^"]+)""#)]
fn record_compaction_summary(world: &mut BotSpyWorld, text: String) {
    let mut session = current_session(world);
    session.messages.push(Message {
        role: Role::User,
        is_compaction_summary: true,
        timestamp: "2026-10-01T09:06:00Z".to_string(),
        parts: vec![Part::Known(KnownPart::Text { text, extra: None })],
        ..Message::default()
    });
    save_session(world, session);
}

#[when(regex = r#"I record a checkpoint step with text "([^"]+)""#)]
fn record_checkpoint_step(world: &mut BotSpyWorld, text: String) {
    let mut session = current_session(world);
    session.messages.push(Message {
        role: Role::Assistant,
        is_compaction_summary: true,
        timestamp: "2026-10-01T09:06:00Z".to_string(),
        parts: vec![Part::Known(KnownPart::Text { text, extra: None })],
        ..Message::default()
    });
    save_session(world, session);
}

#[when(
    regex = r#"I record a compaction window "([^"]+)" replacing history after window "([^"]+)""#
)]
fn record_compaction_window(world: &mut BotSpyWorld, window: String, previous: String) {
    let mut session = current_session(world);
    session.compaction_windows.insert(
        window.clone(),
        CompactionWindow {
            previous_window: Some(previous),
            retained_context: Some("replacement-history".to_string()),
        },
    );
    save_session(world, session);
}

#[when(regex = r#"I record residual context of ([0-9]+) tokens"#)]
fn record_residual_context(world: &mut BotSpyWorld, tokens: u64) {
    let mut session = current_session(world);
    session.residual_context_tokens = Some(tokens);
    save_session(world, session);
}

#[then(regex = r#"the session has a compaction event"#)]
fn has_compaction_event(world: &mut BotSpyWorld) {
    let session = current_session(world);
    assert!(
        !session.compactions.is_empty(),
        "session records no compaction events"
    );
}

#[then(regex = r#"the compaction event has pre_tokens ([0-9]+)"#)]
fn compaction_pre_tokens(world: &mut BotSpyWorld, expected: u64) {
    let session = current_session(world);
    let event = session.compactions.last().expect("no compaction events");
    assert_eq!(event.pre_tokens, Some(expected));
}

#[then(regex = r#"the compaction event has post_tokens ([0-9]+)"#)]
fn compaction_post_tokens(world: &mut BotSpyWorld, expected: u64) {
    let session = current_session(world);
    let event = session.compactions.last().expect("no compaction events");
    assert_eq!(event.post_tokens, Some(expected));
}

#[then(regex = r#"the compaction event points at its logical parent message"#)]
fn compaction_points_at_parent(world: &mut BotSpyWorld) {
    let session = current_session(world);
    let event = session.compactions.last().expect("no compaction events");
    let parent_id = event
        .logical_parent_message_id
        .as_ref()
        .expect("compaction event has no logical parent");
    assert!(
        session
            .messages
            .iter()
            .any(|m| m.id.as_deref() == Some(parent_id.as_str())),
        "logical parent {parent_id} is not a message of this session"
    );
}

#[then(regex = r#"the last message is marked as a compaction summary"#)]
fn last_message_is_compaction_summary(world: &mut BotSpyWorld) {
    assert!(
        last_message(world).is_compaction_summary,
        "the last message is not marked as a compaction summary"
    );
}

#[then(regex = r#"the session has a compaction window "([^"]+)""#)]
fn has_compaction_window(world: &mut BotSpyWorld, window: String) {
    let session = current_session(world);
    assert!(
        session.compaction_windows.contains_key(&window),
        "no compaction window {window}"
    );
}

#[then(regex = r#"compaction window "([^"]+)" has previous window "([^"]+)""#)]
fn compaction_window_previous(world: &mut BotSpyWorld, window: String, previous: String) {
    let session = current_session(world);
    let entry = session
        .compaction_windows
        .get(&window)
        .unwrap_or_else(|| panic!("no compaction window {window}"));
    assert_eq!(entry.previous_window.as_deref(), Some(previous.as_str()));
}

#[then(regex = r#"compaction window "([^"]+)" records its retained context"#)]
fn compaction_window_retained(world: &mut BotSpyWorld, window: String) {
    let session = current_session(world);
    let entry = session
        .compaction_windows
        .get(&window)
        .unwrap_or_else(|| panic!("no compaction window {window}"));
    assert!(
        entry.retained_context.is_some(),
        "window {window} records no retained context"
    );
}

#[then(regex = r#"the session records residual context ([0-9]+) tokens"#)]
fn session_residual_context(world: &mut BotSpyWorld, tokens: u64) {
    let session = current_session(world);
    assert_eq!(session.residual_context_tokens, Some(tokens));
}

#[then(regex = r#"the session has no compaction events"#)]
fn no_compaction_events(world: &mut BotSpyWorld) {
    let session = current_session(world);
    assert!(
        session.compactions.is_empty() && session.compaction_windows.is_empty(),
        "compaction events should be absent"
    );
}
