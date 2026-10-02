//! The normalized schema: messages made of typed parts.

use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::BTreeMap;

/// Version of the normalized schema.
pub const SCHEMA_VERSION: u32 = 1;

/// RFC 3339 UTC timestamp.
pub type Timestamp = String;

/// Who produced a message.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Role {
    #[default]
    User,
    Assistant,
    System,
    Tool,
}

/// A coding-agent source.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Agent {
    #[default]
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

/// Token usage an agent persisted for one message, one turn, or a whole
/// thread. Counters an agent does not record stay `None`, never zero.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct Usage {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub input_tokens: Option<u64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub output_tokens: Option<u64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub cache_read_tokens: Option<u64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub cache_write_tokens: Option<u64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub cached_input_tokens: Option<u64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub reasoning_tokens: Option<u64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub model: Option<String>,
}

impl Usage {
    /// Accumulate `other` into `self`: counters add up when both sides have
    /// them, otherwise the present side wins. The model is kept from either.
    pub fn add(&mut self, other: &Usage) {
        for (mine, theirs) in [
            (&mut self.input_tokens, &other.input_tokens),
            (&mut self.output_tokens, &other.output_tokens),
            (&mut self.cache_read_tokens, &other.cache_read_tokens),
            (&mut self.cache_write_tokens, &other.cache_write_tokens),
            (&mut self.cached_input_tokens, &other.cached_input_tokens),
            (&mut self.reasoning_tokens, &other.reasoning_tokens),
        ] {
            *mine = match (*mine, *theirs) {
                (Some(a), Some(b)) => Some(a + b),
                (None, Some(b)) => Some(b),
                (a, None) => a,
            };
        }
        self.model = self.model.take().or_else(|| other.model.clone());
    }
}

/// The cost state an agent persisted for a session.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct SessionCost {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub total_cost_usd: Option<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub total_api_duration_ms: Option<u64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub total_tool_duration_ms: Option<u64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub total_lines_added: Option<u64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub total_lines_removed: Option<u64>,
    /// Cost attributed per model, for agents that record it that way.
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub per_model: BTreeMap<String, ModelCost>,
}

/// Cost attributed to one model within a session.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct ModelCost {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub cost_usd: Option<f64>,
}

/// Rate-limit and plan state an agent persisted for a session.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct RateLimitState {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub plan_type: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub used_percent: Option<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub window_minutes: Option<u64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub resets_at: Option<Timestamp>,
}

/// One turn of a session: a user request plus everything the agent did to
/// answer it. Turn-level facts live here, not on the messages.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct Turn {
    pub id: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub parent_turn_id: Option<String>,
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
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct Message {
    pub role: Role,
    pub parts: Vec<Part>,
    pub timestamp: Timestamp,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub provenance: Option<Provenance>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub turn_id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub usage: Option<Usage>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub extra: Option<Value>,
}
