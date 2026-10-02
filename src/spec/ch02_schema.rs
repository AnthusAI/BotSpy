//! # Chapter 2 — The normalized schema
//!
//! *Spec layer 2 · mirrors `features/02_schema/` ·
//! [index](crate) ·
//! [previous: session history](crate::spec::ch01_session_history)*
//!
//! Chapter 1 treated a session as a black box with a `messages` list. This
//! chapter opens that box. Every agent writes messages differently —
//! Claude Code's JSONL lines, Cursor's SQLite agent transcripts, Codex's
//! JSON sessions — and the point of BotSpy is that after normalization you
//! never care: there is exactly one message shape, [`Message`](crate::schema::Message),
//! made of ordered **parts**.
//!
//! ## Messages and roles
//!
//! A [`Message`](crate::schema::Message) has a
//! [`Role`](crate::schema::Role) (`User`, `Assistant`, `System`, `Tool`),
//! an ordered `parts` list, an RFC 3339 UTC `timestamp`, optional
//! [`Provenance`](crate::schema::Provenance), and an optional `extra`
//! escape hatch (below). Roles back the conversations you know: you ask,
//! the agent answers (*Scenario: A session's messages are typed user and
//! assistant messages* in the embedded spec).
//!
//! ## Parts: the anatomy of a message
//!
//! Parts are the typed content of a message. The fixed kinds are the
//! [`KnownPart`](crate::schema::KnownPart) variants:
//!
//! - [`KnownPart::Text`](crate::schema::KnownPart::Text) — plain conversational text;
//! - [`KnownPart::Thinking`](crate::schema::KnownPart::Thinking) — the agent's reasoning trace, kept separate
//!   from what it said out loud;
//! - [`KnownPart::ToolCall`](crate::schema::KnownPart::ToolCall) — the agent invoking a tool: a call `id`, a
//!   tool `name`, and optional `arguments`;
//! - [`KnownPart::ToolResult`](crate::schema::KnownPart::ToolResult) — what came back, tied to its call by
//!   `call_id`;
//! - [`KnownPart::Attachment`](crate::schema::KnownPart::Attachment) — a file the conversation touched: `path`,
//!   `mime` type, `size`;
//! - [`KnownPart::System`](crate::schema::KnownPart::System) — system-level prompts and notices.
//!
//! A tool call and its result are two parts in the message stream, joined
//! by the call id (*Scenario: A tool call is followed by its linked tool
//! result*). The result may come back on a later `Tool`-role message; the
//! linkage is by `call_id`, not position.
//!
//! ```
//! use botspy::{KnownPart, Message, Part, Role};
//!
//! let call = Part::Known(KnownPart::ToolCall {
//!     id: "call_9".into(),
//!     name: "read_file".into(),
//!     arguments: Some(serde_json::json!({"path": "src/main.rs"})),
//!     extra: None,
//! });
//! let result = Part::Known(KnownPart::ToolResult {
//!     call_id: "call_9".into(),
//!     text: Some("fn main() {}".into()),
//!     extra: None,
//! });
//!
//! let assistant = Message {
//!     role: Role::Assistant,
//!     parts: vec![
//!         Part::Known(KnownPart::Thinking { text: "need to read main.rs first".into(), extra: None }),
//!         call,
//!     ],
//!     timestamp: "2026-10-01T09:02:00Z".into(),
//!     provenance: None,
//!     extra: None,
//! };
//! let tool = Message {
//!     role: Role::Tool,
//!     parts: vec![result],
//!     timestamp: "2026-10-01T09:02:01Z".into(),
//!     provenance: None,
//!     extra: None,
//! };
//!
//! // A message is a conversation chunk; its parts are its content.
//! assert_eq!(assistant.parts.len(), 2);
//! assert_eq!(tool.parts[0].kind_name(), Some("tool_result"));
//! ```
//!
//! Attachments work the same way — path, MIME type, size
//! (*Scenario: An attachment part carries path, mime and size*):
//!
//! ```
//! use botspy::{KnownPart, Part};
//!
//! let attachment = Part::Known(KnownPart::Attachment {
//!     path: "docs/spec.md".into(),
//!     mime: "text/markdown".into(),
//!     size: 4123,
//!     extra: None,
//! });
//! assert_eq!(attachment.kind_name(), Some("attachment"));
//! ```
//!
//! ## Provenance: where each message came from
//!
//! Every message can carry [`Provenance`](crate::schema::Provenance): the
//! `source_file` it was read from, plus a `line` (JSONL transcript lines)
//! or a `row` (SQLite-style transcript rows). This is what makes "show me
//! the raw record behind this normalized message" possible — the whole
//! point of a library named BotSpy. See *Scenario: A message records its
//! source file and line* in the embedded spec.
//!
//! ```
//! use botspy::{Provenance, Role, KnownPart, Message, Part};
//!
//! let message = Message {
//!     role: Role::User,
//!     parts: vec![Part::Known(KnownPart::Text { text: "hello".into(), extra: None })],
//!     timestamp: "2026-10-01T09:01:00Z".into(),
//!     provenance: Some(Provenance {
//!         source_file: "~/.claude/projects/demo/session-abc.jsonl".into(),
//!         line: Some(17),
//!         row: None,
//!     }),
//!     extra: None,
//! };
//!
//! let provenance = message.provenance.as_ref().unwrap();
//! assert_eq!(provenance.source_file, "~/.claude/projects/demo/session-abc.jsonl");
//! assert_eq!(provenance.line, Some(17));
//! ```
//!
//! ## The `extra` escape hatch
//!
//! Normalization must never lose information. Three mechanisms guarantee
//! that:
//!
//! 1. **Per-part**: every [`KnownPart`](crate::schema::KnownPart) variant
//!    carries an `extra` field for agent-specific attributes the schema
//!    does not model yet.
//! 2. **Per-message**: `Message::extra` holds unknown top-level fields.
//! 3. **Unknown kinds**: a part whose `kind` matches none of the fixed
//!    kinds falls through to [`Part::Extra`](crate::schema::Part::Extra) —
//!    the raw JSON object, including its `kind`, preserved verbatim
//!    (*Scenario: An unknown part kind is preserved in raw form*).
//!    [`Part::as_extra`](crate::schema::Part::as_extra) gets the raw
//!    payload back out.
//!
//! ```
//! use botspy::Part;
//! use serde_json::json;
//!
//! let part: Part = serde_json::from_value(json!({
//!     "kind": "voice_note",
//!     "duration_ms": 8200,
//!     "transcript": "can you hear me now"
//! }))?;
//!
//! // Not one of the fixed kinds -> raw passthrough, nothing dropped.
//! assert_eq!(part.kind_name(), Some("voice_note"));
//! let raw = part.as_extra().unwrap();
//! assert_eq!(raw["duration_ms"], 8200);
//! assert_eq!(raw["transcript"], "can you hear me now");
//! # Ok::<(), serde_json::Error>(())
//! ```
//!
//! ## How adapters map into the schema
//!
//! Adapters do the lossy-looking part so the schema can stay lossless. An
//! [`Adapter`](crate::adapter::Adapter) implementation reads its agent's
//! native format and maps every record into
//! [`Message`](crate::schema::Message) parts: transcript lines become
//! `Text`, reasoning blocks become `Thinking`, `tool_use`/`function_call`
//! records become `ToolCall`, and so on. Anything the adapter cannot map
//! goes to `Extra` rather than being dropped, and every record keeps its
//! `Provenance`. The per-agent mappings and the shared adapter contract are
//! spec layer 4 — chapter 4, specified next.
//!
#![doc = concat!(
    "## Behavior specification\n\n",
    "### message_parts.feature\n\n```gherkin\n",
    include_str!("../../features/02_schema/message_parts.feature"),
    "\n```\n\n",
    "### provenance.feature\n\n```gherkin\n",
    include_str!("../../features/02_schema/provenance.feature"),
    "\n```\n\n",
    "### extra_passthrough.feature\n\n```gherkin\n",
    include_str!("../../features/02_schema/extra_passthrough.feature"),
    "\n```\n"
)]
