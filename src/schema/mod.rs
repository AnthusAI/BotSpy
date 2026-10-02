//! The normalized schema: messages made of typed parts.

use serde::{Deserialize, Serialize};
use serde_json::Value;

/// Version of the normalized schema.
pub const SCHEMA_VERSION: u32 = 1;

/// RFC 3339 UTC timestamp.
pub type Timestamp = String;

/// Who produced a message.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Role {
    User,
    Assistant,
    System,
    Tool,
}

/// A coding-agent source.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Agent {
    ClaudeCode,
    Cursor,
    Codex,
    GrokBot,
    Antigravity,
}

/// A part with one of the fixed kinds.
///
/// `extra` carries agent-specific fields the schema does not model yet, so
/// nothing is lost on normalization.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum KnownPart {
    Text {
        text: String,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        extra: Option<Value>,
    },
    Thinking {
        text: String,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        extra: Option<Value>,
    },
    ToolCall {
        id: String,
        name: String,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        arguments: Option<Value>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        extra: Option<Value>,
    },
    ToolResult {
        call_id: String,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        text: Option<String>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        extra: Option<Value>,
    },
    Attachment {
        path: String,
        mime: String,
        size: u64,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        extra: Option<Value>,
    },
    System {
        text: String,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        extra: Option<Value>,
    },
}

/// A typed content part.
///
/// Anything outside the fixed kinds falls through to [`Part::Extra`], the raw
/// passthrough: the original JSON object (including its `kind`) is preserved
/// verbatim.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(untagged)]
pub enum Part {
    Known(KnownPart),
    Extra(Value),
}

impl Part {
    /// The kind name, for known kinds and for raw parts that carry a `kind`.
    pub fn kind_name(&self) -> Option<&str> {
        match self {
            Part::Known(known) => Some(match known {
                KnownPart::Text { .. } => "text",
                KnownPart::Thinking { .. } => "thinking",
                KnownPart::ToolCall { .. } => "tool_call",
                KnownPart::ToolResult { .. } => "tool_result",
                KnownPart::Attachment { .. } => "attachment",
                KnownPart::System { .. } => "system",
            }),
            Part::Extra(raw) => raw.get("kind").and_then(Value::as_str),
        }
    }

    /// The raw passthrough payload when this part is not one of the fixed kinds.
    pub fn as_extra(&self) -> Option<&Value> {
        match self {
            Part::Extra(raw) => Some(raw),
            _ => None,
        }
    }
}

/// Where a record came from on disk.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Provenance {
    pub source_file: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub line: Option<u64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub row: Option<u64>,
}

/// A message inside a session: ordered typed parts.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Message {
    pub role: Role,
    pub parts: Vec<Part>,
    pub timestamp: Timestamp,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub provenance: Option<Provenance>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub extra: Option<Value>,
}
