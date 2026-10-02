//! Steps for optional timestamps and file metadata (spec 09_optional_timestamp).

use crate::steps::{current_session, save_session, BotSpyWorld};
use botspy::{KnownPart, Message, Part, Role};
use cucumber::{then, when};

const ORDER: [&str; 3] = ["first", "second", "third"];

#[when(regex = r#"I record 3 messages in file order without timestamps"#)]
fn record_untimestamped_messages(world: &mut BotSpyWorld) {
    let mut session = current_session(world);
    for text in ORDER {
        session.messages.push(Message {
            role: Role::User,
            parts: vec![Part::Known(KnownPart::Text {
                text: text.into(),
                extra: None,
            })],
            ..Message::default()
        });
    }
    save_session(world, session);
}

#[then(regex = r#"all 3 messages have no timestamp"#)]
fn all_messages_untimestamped(world: &mut BotSpyWorld) {
    let messages = current_session(world).messages;
    assert_eq!(messages.len(), 3, "expected 3 recorded messages");
    for (i, message) in messages.iter().enumerate() {
        assert!(
            message.timestamp.is_none(),
            "message {i} has a fabricated timestamp"
        );
    }
}

#[then(regex = r#"the messages are ordered by their source order, not by time"#)]
fn messages_keep_source_order(world: &mut BotSpyWorld) {
    let messages = current_session(world).messages;
    for (message, expected) in messages.iter().zip(ORDER) {
        let text = message.parts.first().map(|part| match part {
            Part::Known(KnownPart::Text { text, .. }) => text.as_str(),
            other => panic!("unexpected part: {other:?}"),
        });
        assert_eq!(text, Some(expected), "messages lost their source order");
    }
}

#[when(regex = r#"the transcript file was modified at "([^"]+)""#)]
fn set_file_modified_at(world: &mut BotSpyWorld, mtime: String) {
    let mut session = current_session(world);
    session.file_modified_at = Some(mtime);
    save_session(world, session);
}

#[then(regex = r#"no message has timestamp "([^"]+)""#)]
fn no_message_has_timestamp(world: &mut BotSpyWorld, ts: String) {
    for (i, message) in current_session(world).messages.iter().enumerate() {
        assert_ne!(
            message.timestamp.as_deref(),
            Some(ts.as_str()),
            "message {i} fabricated the file mtime into its timestamp"
        );
    }
}

#[then(regex = r#"the file modification time is recorded outside message timestamps"#)]
fn file_mtime_recorded_outside_messages(world: &mut BotSpyWorld) {
    let session = current_session(world);
    let mtime = session
        .file_modified_at
        .expect("the session does not record the file modification time");
    for message in &session.messages {
        assert_ne!(
            message.timestamp.as_deref(),
            Some(mtime.as_str()),
            "a message timestamp duplicates the file mtime"
        );
    }
}
