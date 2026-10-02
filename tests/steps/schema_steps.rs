//! Steps for the normalized schema (layer 2).

use crate::steps::{adapter_for, adapter_owning, data_rows, header, BotSpyWorld};
use botspy::{Adapter, KnownPart, Message, Part, Provenance};
use cucumber::{given, then, when};
use serde_json::Value;

#[given(regex = r#"a fixture session "([^"]+)" from agent "([^"]+)" in project "([^"]+)""#)]
fn fixture_session(world: &mut BotSpyWorld, id: String, agent: String, project: String) {
    let adapter = adapter_for(world, &agent);
    adapter.add_session(new_session(&id, &agent, &project));
    world.current_session = Some(id);
}

fn new_session(id: &str, agent: &str, project: &str) -> botspy::Session {
    botspy::Session {
        id: id.to_string(),
        agent: crate::steps::parse_agent(agent),
        project_id: project.to_string(),
        started_at: "2026-10-01T09:00:00Z".to_string(),
        last_activity_at: "2026-10-01T09:00:00Z".to_string(),
        ..botspy::Session::default()
    }
}

/// Build one part from a table row, interpreting columns by header name.
fn part_from_row(columns: &[String], row: &[String]) -> Part {
    let cell = |name: &str| -> String {
        columns
            .iter()
            .position(|c| c == name)
            .and_then(|index| row.get(index))
            .cloned()
            .unwrap_or_default()
    };
    let kind = cell("kind");
    let text = cell("text");
    match kind.as_str() {
        "text" => Part::Known(KnownPart::Text { text, extra: None }),
        "thinking" => Part::Known(KnownPart::Thinking { text, extra: None }),
        "system" => Part::Known(KnownPart::System { text, extra: None }),
        "tool_call" => Part::Known(KnownPart::ToolCall {
            id: cell("id"),
            name: cell("name"),
            arguments: None,
            status: None,
            extra: None,
        }),
        "tool_result" => Part::Known(KnownPart::ToolResult {
            call_id: cell("call_id"),
            text: (!text.is_empty()).then_some(text),
            status: None,
            extra: None,
        }),
        "attachment" => Part::Known(KnownPart::Attachment {
            path: cell("path"),
            mime: cell("mime"),
            size: cell("size")
                .parse()
                .expect("attachment size must be a number"),
            extra: None,
        }),
        other => {
            let mut raw = match serde_json::from_str::<Value>(&cell("raw")) {
                Ok(Value::Object(map)) => Value::Object(map),
                _ => Value::Object(serde_json::Map::new()),
            };
            raw["kind"] = Value::String(other.to_string());
            Part::Extra(raw)
        }
    }
}

fn record_message(world: &mut BotSpyWorld, message: Message) {
    let id = world
        .current_session
        .clone()
        .expect("no current fixture session");
    let adapter = adapter_owning(world, &id).expect("current session not found");
    let mut session = adapter.open(&id).expect("current session vanished");
    let last_activity = message.timestamp.clone();
    session.messages.push(message);
    session.last_activity_at = last_activity;
    adapter.add_session(session);
}

#[when(regex = r#"I record a message with role "([^"]+)" at "([^"]+)" and parts:"#)]
fn record_parts(
    world: &mut BotSpyWorld,
    role: String,
    timestamp: String,
    step: &cucumber::gherkin::Step,
) {
    let columns = header(step);
    let parts = data_rows(step)
        .map(|row| part_from_row(&columns, row))
        .collect();
    record_message(
        world,
        Message {
            role: crate::steps::session_steps::parse_role(&role),
            parts,
            timestamp,
            ..Message::default()
        },
    );
}

#[when(
    regex = r#"I record a message with role "([^"]+)" at "([^"]+)" with text "([^"]+)" and provenance "([^"]+)" line ([0-9]+)"#
)]
fn record_provenance_line(
    world: &mut BotSpyWorld,
    role: String,
    timestamp: String,
    text: String,
    file: String,
    line: u64,
) {
    record_message(
        world,
        Message {
            role: crate::steps::session_steps::parse_role(&role),
            parts: vec![Part::Known(KnownPart::Text { text, extra: None })],
            timestamp,
            provenance: Some(Provenance {
                source_file: file,
                line: Some(line),
                row: None,
            }),
            ..Message::default()
        },
    );
}

#[when(
    regex = r#"I record a message with role "([^"]+)" at "([^"]+)" with text "([^"]+)" and provenance "([^"]+)" row ([0-9]+)"#
)]
fn record_provenance_row(
    world: &mut BotSpyWorld,
    role: String,
    timestamp: String,
    text: String,
    file: String,
    row: u64,
) {
    record_message(
        world,
        Message {
            role: crate::steps::session_steps::parse_role(&role),
            parts: vec![Part::Known(KnownPart::Text { text, extra: None })],
            timestamp,
            provenance: Some(Provenance {
                source_file: file,
                line: None,
                row: Some(row),
            }),
            ..Message::default()
        },
    );
}

fn last_message(world: &BotSpyWorld) -> Message {
    let id = world
        .current_session
        .as_deref()
        .expect("no current fixture session");
    let adapter = adapter_owning(world, id).expect("current session not found");
    let session = adapter.open(id).expect("current session vanished");
    session
        .messages
        .last()
        .cloned()
        .expect("no messages recorded")
}

#[then(regex = r#"the last message has ([0-9]+) parts? with kinds "([^"]+)""#)]
fn last_message_kinds(world: &mut BotSpyWorld, count: usize, kinds: String) {
    let message = last_message(world);
    let actual: Vec<String> = message
        .parts
        .iter()
        .map(|part| part.kind_name().expect("part has no kind name").to_string())
        .collect();
    let expected: Vec<&str> = kinds.split(", ").collect();
    assert_eq!(message.parts.len(), count);
    assert_eq!(actual, expected);
}

#[then(regex = r#"part ([0-9]+) is a tool_call with id "([^"]+)" and name "([^"]+)""#)]
fn part_tool_call(world: &mut BotSpyWorld, index: usize, id: String, name: String) {
    let message = last_message(world);
    match &message.parts[index - 1] {
        Part::Known(KnownPart::ToolCall {
            id: part_id,
            name: part_name,
            ..
        }) => {
            assert_eq!(part_id, &id);
            assert_eq!(part_name, &name);
        }
        other => panic!("part {index} is not a tool_call: {other:?}"),
    }
}

#[then(regex = r#"part ([0-9]+) references call "([^"]+)""#)]
fn part_tool_result(world: &mut BotSpyWorld, index: usize, call_id: String) {
    let message = last_message(world);
    match &message.parts[index - 1] {
        Part::Known(KnownPart::ToolResult {
            call_id: ref_id, ..
        }) => {
            assert_eq!(ref_id, &call_id);
        }
        other => panic!("part {index} is not a tool_result: {other:?}"),
    }
}

#[then(
    regex = r#"part ([0-9]+) is an attachment with path "([^"]+)", mime "([^"]+)" and size ([0-9]+)"#
)]
fn part_attachment(world: &mut BotSpyWorld, index: usize, path: String, mime: String, size: u64) {
    let message = last_message(world);
    match &message.parts[index - 1] {
        Part::Known(KnownPart::Attachment {
            path: part_path,
            mime: part_mime,
            size: part_size,
            ..
        }) => {
            assert_eq!(part_path, &path);
            assert_eq!(part_mime, &mime);
            assert_eq!(*part_size, size);
        }
        other => panic!("part {index} is not an attachment: {other:?}"),
    }
}

#[then(regex = r#"the last message provenance is file "([^"]+)" line ([0-9]+)"#)]
fn provenance_line(world: &mut BotSpyWorld, file: String, line: u64) {
    let message = last_message(world);
    let provenance = message
        .provenance
        .as_ref()
        .expect("message has no provenance");
    assert_eq!(provenance.source_file, file);
    assert_eq!(provenance.line, Some(line));
    assert_eq!(provenance.row, None);
}

#[then(regex = r#"the last message provenance is file "([^"]+)" row ([0-9]+)"#)]
fn provenance_row(world: &mut BotSpyWorld, file: String, row: u64) {
    let message = last_message(world);
    let provenance = message
        .provenance
        .as_ref()
        .expect("message has no provenance");
    assert_eq!(provenance.source_file, file);
    assert_eq!(provenance.row, Some(row));
    assert_eq!(provenance.line, None);
}

#[then(regex = r#"part ([0-9]+) is not one of the fixed kinds"#)]
fn part_is_extra(world: &mut BotSpyWorld, index: usize) {
    let message = last_message(world);
    let part = &message.parts[index - 1];
    assert!(
        part.as_extra().is_some(),
        "part {index} should be raw passthrough, got {part:?}"
    );
}

#[then(regex = r#"part ([0-9]+) preserves the raw kind "([^"]+)""#)]
fn part_raw_kind(world: &mut BotSpyWorld, index: usize, kind: String) {
    let message = last_message(world);
    let raw = message.parts[index - 1]
        .as_extra()
        .expect("part is not raw passthrough");
    assert_eq!(raw.get("kind"), Some(&Value::String(kind)));
}

#[then(regex = r#"part ([0-9]+) preserves raw field "([^"]+)" equal to "([^"]+)""#)]
fn part_raw_field(world: &mut BotSpyWorld, index: usize, field: String, expected: String) {
    let message = last_message(world);
    let raw = message.parts[index - 1]
        .as_extra()
        .expect("part is not raw passthrough");
    let actual = raw
        .get(&field)
        .map(|value| match value {
            Value::String(s) => s.clone(),
            other => other.to_string(),
        })
        .unwrap_or_default();
    assert_eq!(actual, expected);
}
