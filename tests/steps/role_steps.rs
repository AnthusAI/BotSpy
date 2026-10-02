//! Steps for role mapping conventions (spec 10_role_mapping).

use crate::steps::{last_message, record_message_on_current, BotSpyWorld};
use botspy::{KnownPart, Message, Origin, Part, Role};
use cucumber::{then, when};

/// The canonical injected-content fixture: the user's own words and the
/// `<system-reminder>` block the harness wrapped around them.
const USER_WORDS: &str = "please continue with the task";
const INJECTED: &str = "sandbox: network access disabled";

fn parse_origin(name: &str) -> Origin {
    Origin::parse(name).unwrap_or_else(|| panic!("unknown origin in feature: {name}"))
}

#[when(regex = r#"I record a developer message with text "([^"]+)""#)]
fn record_developer_message(world: &mut BotSpyWorld, text: String) {
    record_message_on_current(
        world,
        Message {
            role: Role::System,
            origin: Some(Origin::Developer),
            timestamp: Some("2026-10-01T09:00:00Z".to_string()),
            parts: vec![Part::Known(KnownPart::Text { text, extra: None })],
            ..Message::default()
        },
    );
}

#[when(
    regex = r#"I record an inter-agent message from "([^"]+)" to "([^"]+)" with text "([^"]+)""#
)]
fn record_inter_agent_message(
    world: &mut BotSpyWorld,
    author: String,
    recipient: String,
    text: String,
) {
    record_message_on_current(
        world,
        Message {
            role: Role::Assistant,
            origin: Some(Origin::AgentMessage),
            author: Some(author),
            recipient: Some(recipient),
            timestamp: Some("2026-10-01T09:00:00Z".to_string()),
            parts: vec![Part::Known(KnownPart::Text { text, extra: None })],
            ..Message::default()
        },
    );
}

#[when(regex = r#"I record an assistant message authored by "([^"]+)""#)]
fn record_authored_message(world: &mut BotSpyWorld, author: String) {
    record_message_on_current(
        world,
        Message {
            role: Role::Assistant,
            author: Some(author),
            timestamp: Some("2026-10-01T09:00:00Z".to_string()),
            parts: vec![Part::Known(KnownPart::Text {
                text: "the persona speaks".to_string(),
                extra: None,
            })],
            ..Message::default()
        },
    );
}

#[when(regex = r#"I record a user message whose text contains a "<system-reminder>" block"#)]
fn record_user_with_reminder(world: &mut BotSpyWorld) {
    let raw = format!("{USER_WORDS}\n<system-reminder>{INJECTED}</system-reminder>");
    let (own, injected) =
        split_injected(&raw).unwrap_or_else(|| panic!("no <system-reminder> block in {raw:?}"));
    record_message_on_current(
        world,
        Message {
            role: Role::User,
            timestamp: Some("2026-10-01T09:00:00Z".to_string()),
            parts: vec![
                Part::Known(KnownPart::Text {
                    text: own,
                    extra: None,
                }),
                Part::Known(KnownPart::System {
                    text: injected,
                    extra: None,
                }),
            ],
            ..Message::default()
        },
    );
}

/// Split injected `<system-reminder>` content out of user text: everything
/// before the block is the user's own words, the block body is injected.
fn split_injected(raw: &str) -> Option<(String, String)> {
    let start = raw.find("<system-reminder>")?;
    let body_end = raw.find("</system-reminder>")?;
    let own = raw[..start].trim().to_string();
    let inner = &raw[start + "<system-reminder>".len()..body_end];
    Some((own, inner.trim().to_string()))
}

#[when(regex = r#"I record a simulated user message with reason "([^"]+)""#)]
fn record_simulated_message(world: &mut BotSpyWorld, reason: String) {
    record_message_on_current(
        world,
        Message {
            role: Role::User,
            origin: Some(Origin::Simulated),
            timestamp: Some("2026-10-01T09:00:00Z".to_string()),
            parts: vec![Part::Known(KnownPart::Text {
                text: format!("simulated: {reason}"),
                extra: None,
            })],
            ..Message::default()
        },
    );
}

#[then(regex = r#"the last message has origin "([^"]+)""#)]
fn last_message_origin(world: &mut BotSpyWorld, origin: String) {
    let expected = parse_origin(&origin);
    let actual = last_message(world)
        .origin
        .expect("the last message has no origin");
    assert_eq!(actual, expected);
}

#[then(regex = r#"the last message has author "([^"]+)""#)]
fn last_message_author(world: &mut BotSpyWorld, author: String) {
    let actual = last_message(world)
        .author
        .expect("the last message has no author");
    assert_eq!(actual, author);
}

#[then(regex = r#"the last message has recipient "([^"]+)""#)]
fn last_message_recipient(world: &mut BotSpyWorld, recipient: String) {
    let actual = last_message(world)
        .recipient
        .expect("the last message has no recipient");
    assert_eq!(actual, recipient);
}

#[then(regex = r#"the last message has a text part with the user's own words"#)]
fn text_part_is_users_own_words(world: &mut BotSpyWorld) {
    let message = last_message(world);
    let found = message.parts.iter().any(|part| match part {
        Part::Known(KnownPart::Text { text, .. }) => text == USER_WORDS,
        _ => false,
    });
    assert!(found, "no text part carries the user's own words");
    assert!(
        !message.parts.iter().any(
            |part| matches!(part, Part::Known(KnownPart::Text { text, .. })
                if text.contains("<system-reminder>"))
        ),
        "injected content is still inside user text"
    );
}

#[then(regex = r#"the last message has a system part with the injected content"#)]
fn system_part_is_injected(world: &mut BotSpyWorld) {
    let message = last_message(world);
    let found = message.parts.iter().any(|part| match part {
        Part::Known(KnownPart::System { text, .. }) => text == INJECTED,
        _ => false,
    });
    assert!(found, "no system part carries the injected content");
}
