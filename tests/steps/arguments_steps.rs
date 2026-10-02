//! Steps for tool-call arguments that are not JSON objects (spec
//! 07_tool_call_arguments).

use crate::steps::{last_message, record_message_on_current, BotSpyWorld};
use botspy::{KnownPart, Message, Part, Role, ToolArguments};
use cucumber::{then, when};
use serde_json::Value;

/// Gherkin step text does not unescape `\"`, so a quoted string parameter
/// written as `\"` arrives as backslash-quote. Features use that escaping
/// to embed quote characters; undo it where the feature means a quote.
fn gherkin_unescape(text: &str) -> String {
    text.replace("\\\"", "\"")
}

fn record_tool_call(world: &mut BotSpyWorld, id: String, name: String, arguments: ToolArguments) {
    record_message_on_current(
        world,
        Message {
            role: Role::Assistant,
            parts: vec![Part::Known(KnownPart::ToolCall {
                id,
                name,
                arguments: Some(arguments),
                status: None,
                extra: None,
            })],
            ..Message::default()
        },
    );
}

#[when(regex = r#"I record a custom tool call "([^"]+)" named "([^"]+)" with raw input "(.*)"$"#)]
fn record_custom_tool_call(world: &mut BotSpyWorld, id: String, name: String, raw: String) {
    record_tool_call(world, id, name, raw_arguments(&raw));
}

#[when(regex = r#"I record a tool call "([^"]+)" named "([^"]+)" with params "(.*)"$"#)]
fn record_params_tool_call(world: &mut BotSpyWorld, id: String, name: String, raw: String) {
    record_tool_call(world, id, name, raw_arguments(&raw));
}

#[when(regex = r#"I record a tool call "([^"]+)" named "([^"]+)" with arguments (.*)$"#)]
fn record_arguments_tool_call(world: &mut BotSpyWorld, id: String, name: String, raw: String) {
    let value: Value = serde_json::from_str(&raw)
        .unwrap_or_else(|err| panic!("arguments are not valid JSON: {err}: {raw}"));
    record_tool_call(world, id, name, ToolArguments::from_value(value));
}

/// Raw arguments: kept verbatim, parsed when the raw string is valid JSON —
/// also after undoing Gherkin-level `\"` escapes, since a feature file must
/// escape quotes to embed them in a step parameter.
fn raw_arguments(raw: &str) -> ToolArguments {
    let parsed = serde_json::from_str(raw)
        .ok()
        .or_else(|| serde_json::from_str(&gherkin_unescape(raw)).ok());
    ToolArguments {
        value: None,
        raw: Some(raw.to_string()),
        parsed,
    }
}

fn tool_arguments(world: &mut BotSpyWorld, index: usize) -> ToolArguments {
    let part = &last_message(world).parts[index - 1];
    match part {
        Part::Known(KnownPart::ToolCall { arguments, .. }) => {
            arguments.clone().expect("tool call carries no arguments")
        }
        other => panic!("part {index} is not a tool_call: {other:?}"),
    }
}

#[then(regex = r#"part ([0-9]+) has arguments raw "(.*)"$"#)]
fn arguments_raw(world: &mut BotSpyWorld, index: usize, expected: String) {
    let arguments = tool_arguments(world, index);
    assert_eq!(arguments.raw.as_deref(), Some(expected.as_str()));
}

#[then(regex = r#"part ([0-9]+) has arguments parsed with path "([^"]+)""#)]
fn arguments_parsed_path(world: &mut BotSpyWorld, index: usize, path: String) {
    let arguments = tool_arguments(world, index);
    let parsed = arguments
        .parsed
        .expect("raw arguments were not parsed though they are valid JSON");
    assert_eq!(parsed.get("path"), Some(&Value::String(path)));
}

#[then(regex = r#"part ([0-9]+) has arguments value at "([^"]+)" equal to "(.*)"$"#)]
fn arguments_value_at(world: &mut BotSpyWorld, index: usize, key: String, expected: String) {
    let arguments = tool_arguments(world, index);
    let value = arguments
        .value
        .expect("tool call carries no structured arguments");
    assert_eq!(
        value.get(&key),
        Some(&Value::String(gherkin_unescape(&expected))),
        "argument value at {key} was double-decoded or lost"
    );
}
