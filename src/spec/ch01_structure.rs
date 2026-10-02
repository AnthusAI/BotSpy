//! # Chapter 1 — Structure
//!
//! *Spec layer 1 · mirrors `features/01_structure/` ·
//! [index](crate) · [next: querying](crate::spec::ch02_querying)*
//!
//! Everything in BotSpy starts from a small set of structural ideas: the
//! conversations you have with coding agents are already on your disk, one
//! history per agent, in one format per agent. BotSpy normalizes them into
//! one shape — a **session**, holding ordered **messages**, each made of
//! typed **parts** — and then gives you the same three moves for every
//! agent: list what is there, open one, and walk through it in order.
//! This chapter covers that structure from the outside in.
//!
//! ## What a session is
//!
//! A [`Session`](crate::session::Session) is one conversation with one
//! agent, identified by four things and one list:
//!
//! - `id` — the session identifier as the agent itself recorded it;
//! - `agent` — which agent produced it
//!   ([`Agent`](crate::schema::Agent): Claude Code, Cursor, Codex, Grok
//!   Bot, Antigravity);
//! - `project_id` — the project the conversation belongs to;
//! - `started_at` and `last_activity_at` — RFC 3339 UTC timestamps;
//! - `messages` — the conversation itself, in order
//!   ([`Message`](crate::schema::Message)).
//!
//! You rarely construct sessions yourself: an
//! [`Adapter`](crate::adapter::Adapter) discovers them on disk. While you
//! are experimenting, the in-memory
//! [`FixtureAdapter`](crate::adapters::fixture::FixtureAdapter) plays the
//! role of a real adapter — every example in these docs uses it.
//!
//! ## Listing sessions across agents
//!
//! A [`SessionStore`](crate::session::SessionStore) is the unified history:
//! you register one adapter per agent and it merges everything they
//! discover. [`SessionStore::list_sessions`](crate::session::SessionStore::list_sessions) returns
//! [`SessionSummary`](crate::session::SessionSummary) values — a lightweight
//! view (id, agent, project, timestamps, message count) — **sorted by last
//! activity, most recent first**. Ties are broken by id so the order is
//! deterministic. This backs *Scenario: Sessions from two agents are listed
//! sorted by last activity* and *Scenario: Each listed session carries its
//! agent and project* in the embedded spec below.
//!
//! ```
//! use botspy::{Agent, FixtureAdapter, Session, SessionStore};
//! use std::sync::Arc;
//!
//! let mut store = SessionStore::new();
//!
//! let claude = Arc::new(FixtureAdapter::new(Agent::ClaudeCode));
//! claude.add_session(Session {
//!     id: "c2".into(),
//!     agent: Agent::ClaudeCode,
//!     project_id: "demo".into(),
//!     started_at: "2026-10-01T08:00:00Z".into(),
//!     last_activity_at: "2026-10-01T11:00:00Z".into(),
//!     ..Session::default()
//! });
//! let codex = Arc::new(FixtureAdapter::new(Agent::Codex));
//! codex.add_session(Session {
//!     id: "x1".into(),
//!     agent: Agent::Codex,
//!     project_id: "infra".into(),
//!     started_at: "2026-10-01T07:00:00Z".into(),
//!     last_activity_at: "2026-10-01T09:30:00Z".into(),
//!     ..Session::default()
//! });
//! store.register(claude);
//! store.register(codex);
//!
//! let summaries = store.list_sessions();
//! let ids: Vec<&str> = summaries.iter().map(|s| s.id.as_str()).collect();
//! assert_eq!(ids, ["c2", "x1"]); // c2's 11:00 activity beats x1's 09:30
//! ```
//!
//! ## Opening a session and walking its messages
//!
//! [`SessionStore::open`](crate::session::SessionStore::open) fetches one full session by id, from whichever
//! registered agent owns it. The messages of a session are ordered — you
//! walk them with an ordinary iterator (or
//! [`SessionStore::messages`](crate::session::SessionStore::messages), which returns them in order):
//!
//! ```
//! use botspy::{Agent, FixtureAdapter, KnownPart, Part, Role, Session, SessionStore};
//! use std::sync::Arc;
//!
//! fn text_session(id: &str, lines: &[&str]) -> Session {
//!     Session {
//!         id: id.into(),
//!         agent: Agent::ClaudeCode,
//!         project_id: "demo".into(),
//!         started_at: "2026-10-01T09:00:00Z".into(),
//!         last_activity_at: "2026-10-01T09:30:00Z".into(),
//!         messages: lines
//!             .iter()
//!             .enumerate()
//!             .map(|(i, line)| botspy::Message {
//!                 role: if i % 2 == 0 { Role::User } else { Role::Assistant },
//!                 parts: vec![Part::Known(KnownPart::Text {
//!                     text: (*line).into(),
//!                     extra: None,
//!                 })],
//!                 timestamp: format!("2026-10-01T09:{:02}:00Z", i + 1),
//!                 ..botspy::Message::default()
//!             })
//!             .collect(),
//!         ..Session::default()
//!     }
//! }
//!
//! let mut store = SessionStore::new();
//! let adapter = Arc::new(FixtureAdapter::new(Agent::ClaudeCode));
//! adapter.add_session(text_session(
//!     "c1",
//!     &[
//!         "please fix the bug",
//!         "the bug is in parser.rs",
//!         "why is the bug in parser.rs?",
//!         "the grammar rule never terminates",
//!     ],
//! ));
//! store.register(adapter);
//!
//! let session = store.open("c1")?;
//! let roles: Vec<Role> = session.messages.iter().map(|m| m.role).collect();
//! assert_eq!(roles, [Role::User, Role::Assistant, Role::User, Role::Assistant]);
//! # Ok::<(), botspy::UnknownSession>(())
//! ```
//!
//! ## The unknown-id error
//!
//! Asking for a session id that no adapter knows is an error, never a
//! silent empty result: [`SessionStore::open`](crate::session::SessionStore::open) returns
//! [`UnknownSession`](crate::session::UnknownSession), which carries the
//! offending id and displays it plainly. See *Scenario Outline: Opening an
//! unknown session id reports an error* in the embedded spec (its
//! `Examples` table exercises two different bad ids).
//!
//! ```
//! use botspy::{Agent, FixtureAdapter, SessionStore};
//! use std::sync::Arc;
//!
//! let mut store = SessionStore::new();
//! let adapter = Arc::new(FixtureAdapter::new(Agent::ClaudeCode));
//! store.register(adapter);
//!
//! let err = store.open("nope").unwrap_err();
//! assert_eq!(err.id, "nope");
//! assert_eq!(err.to_string(), "unknown session id: nope");
//! ```
//!
//! ## Messages and roles
//!
//! Inside a session, every message has a [`Role`](crate::schema::Role)
//! (`User`, `Assistant`, `System`, `Tool`), an ordered `parts` list, an
//! RFC 3339 UTC `timestamp`, optional
//! [`Provenance`](crate::schema::Provenance), and an optional `extra`
//! escape hatch (below). Roles back the conversations you know: you ask,
//! the agent answers (*Scenario: A session's messages are typed user and
//! assistant messages* in the embedded spec). Shades beyond those four —
//! developer messages, inter-agent mail, injected and simulated user
//! content — map by documented conventions; those conventions are
//! specified in `10_role_mapping.feature` and are pending (see "Pending
//! schema work" below).
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
//!   `call_id`, with an optional outcome
//!   ([`PartStatus`](crate::schema::PartStatus): ok, error, interrupted; tool
//!   calls carry a status of their own too, e.g. Cursor's loading).
//! - [`KnownPart::Attachment`](crate::schema::KnownPart::Attachment) — a file the conversation touched: `path`,
//!   `mime` type, `size`. Inline bytes (Claude Code's base64 images) and
//!   blob-backed data do not fit this yet — specified in
//!   `08_inline_data.feature`, pending;
//! - [`KnownPart::System`](crate::schema::KnownPart::System) — system-level prompts and notices.
//!
//! A tool call and its result are two parts in the message stream, joined
//! by the call id (*Scenario: A tool call is followed by its linked tool
//! result*). The result may come back on a later `Tool`-role message; the
//! linkage is by `call_id`, not position.
//!
//! ```
//! use botspy::{KnownPart, Message, Part, Role, ToolArguments};
//!
//! let call = Part::Known(KnownPart::ToolCall {
//!     id: "call_9".into(),
//!     name: "read_file".into(),
//!     arguments: Some(ToolArguments::from_value(
//!         serde_json::json!({"path": "src/main.rs"}),
//!     )),
//!     status: None,
//!     extra: None,
//! });
//! let result = Part::Known(KnownPart::ToolResult {
//!     call_id: "call_9".into(),
//!     text: Some("fn main() {}".into()),
//!     status: None,
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
//!     ..Message::default()
//! };
//! let tool = Message {
//!     role: Role::Tool,
//!     parts: vec![result],
//!     timestamp: "2026-10-01T09:02:01Z".into(),
//!     ..Message::default()
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
//! source file and line* in the embedded spec. The native record identity —
//! the agent's own record id and type, an ordinal sort key (Codex numbers
//! every record; Cursor orders bubbles by header list), and the native
//! parent pointer (Claude Code's uuid/parentUuid tree) — is on
//! [`Provenance`](crate::schema::Provenance) too: `record_id`, `record_type`,
//! `ordinal`, and `parent_record`.
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
//!         ..Provenance::default()
//!     }),
//!     ..Message::default()
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
//! ## Going further
//!
//! **Mixed agents.** There is nothing special about listing across two
//! agents — [`SessionStore::list_sessions`](crate::session::SessionStore::list_sessions) simply merges every adapter's
//! discoveries. One caveat is deliberate: session ids are only guaranteed
//! unique *within* one agent. When two agents use the same id, the first
//! registered adapter that knows it wins; ids remain agent-scoped by
//! design.
//!
//! **Partial or corrupt histories.** Real transcripts get truncated mid-
//! line, contain records from formats the agent no longer writes, or stop
//! halfway. The importer contract (chapter 3) makes this explicit: malformed
//! lines and unknown records are *skipped and counted, never fatal*, and a
//! missing source root means "absent", not "error". A session that opens is
//! the part of the history that parsed.
//!
//! **What "last activity" means.** `last_activity_at` is the timestamp of
//! the most recent thing that happened in the session, as recorded by the
//! agent itself. Real importers derive it from the source of truth they
//! have: the last message timestamp when lines carry timestamps, otherwise
//! the session file's metadata (exactly the fallback the prototype used for
//! Cursor agent transcripts). The fixture adapter simply takes whatever you
//! put in the field. Because the sort key comes from the records, listing
//! is deterministic for a given snapshot of disk.
//!
//! ## Pending schema work (specified, not implemented)
//!
//! The on-disk storage survey of all five agents produced twelve schema
//! recommendations. They are written as Gherkin specs first, tagged `@wip`
//! (excluded from the executable run until implemented), and embedded at
//! the end of this chapter. In priority order:
//!
//! 1. `01_usage_tokens.feature` — usage and cost metrics on messages and
//!    sessions (input/output/cache/reasoning tokens, `cost_usd`, model,
//!    session-level rate-limit/plan state); usage an agent does not persist
//!    is absent, never zero. Implemented: [`Usage`](crate::schema::Usage),
//!    [`SessionCost`](crate::schema::SessionCost),
//!    [`RateLimitState`](crate::schema::RateLimitState), and the
//!    [`Session::turn_usage`](crate::session::Session::turn_usage) /
//!    [`Session::thread_usage`](crate::session::Session::thread_usage)
//!    accumulators.
//! 2. `02_tool_result_status.feature` — error and status flags on tool
//!    results, tool calls, messages, and turns. Implemented:
//!    [`PartStatus`](crate::schema::PartStatus) on tool results and calls,
//!    [`TurnStatus`](crate::schema::TurnStatus) with abort reasons and
//!    errors on turns, and the message error flag.
//! 3. `03_session_graph.feature` — parent/root links, sub-agent kind and
//!    name, fork points, best-of-N battles, and peer ids. Implemented:
//!    [`Session::parent_id`](crate::session::Session::parent_id) /
//!    `root_id`, [`SubagentInfo`](crate::schema::SubagentInfo),
//!    [`ForkPoint`](crate::schema::ForkPoint),
//!    [`BattleLink`](crate::schema::BattleLink), and
//!    [`Peer`](crate::schema::Peer).
//! 4. `04_turn_grouping.feature` — `turn_id`/`parent_turn_id` with
//!    turn-level timing and errors.
//! 5. `05_compaction.feature` — compaction events with pre/post token
//!    counts, summary messages, and compaction window chains. Implemented:
//!    [`CompactionEvent`](crate::schema::CompactionEvent) (with
//!    `logical_parent_message_id` via
//!    [`Message::id`](crate::schema::Message::id)),
//!    [`CompactionWindow`](crate::schema::CompactionWindow), the message
//!    compaction-summary flag, and residual context tokens.
//! 6. `provenance.feature` (later scenarios) — native record id, record
//!    type, ordinal sort key, and native parent pointer. Implemented:
//!    [`Provenance::record_id`](crate::schema::Provenance::record_id),
//!    `record_type`, `ordinal`, and `parent_record`.
//! 7. `07_tool_call_arguments.feature` — raw-string tool-call arguments,
//!    kept raw, parsed only when the raw string is valid JSON. Implemented:
//!    [`ToolArguments`](crate::schema::ToolArguments) (`value` for
//!    structured objects, `raw` kept verbatim, `parsed` from the first
//!    parse — never double-decoded).
//! 8. `08_inline_data.feature` — inline-data part with media type, content
//!    hash, and `data_ref` for blob-backed bytes. Implemented:
//!    [`KnownPart::InlineData`](crate::schema::KnownPart::InlineData) with
//!    [`content_hash`](crate::schema::content_hash) (SHA-256) and
//!    `data_ref` for blob-backed bytes.
//! 9. `09_optional_timestamp.feature` — optional timestamps; never
//!    fabricated from file metadata.
//! 10. `10_role_mapping.feature` — developer, inter-agent, injected, and
//!     simulated message conventions.
//! 11. `11_session_metadata.feature` — optional Session metadata: title,
//!     git coordinates, cwd, archived, pr_url, status, app version, models.
//! 12. `12_raw_blob_escape_hatch.feature` — `{blob_hash, container}` blob
//!     references for opaque content, plus partial/cloud-cache session
//!     flags.
//!
//! Until a scenario's implementation lands, the structs above (no error
//! flag on [`KnownPart::ToolResult`](crate::schema::KnownPart::ToolResult),
//! narrow [`Provenance`](crate::schema::Provenance)) are the interim truth,
//! and the `@wip` scenarios are the specification those structs will be
//! changed to satisfy.
//!
#![doc = concat!(
    "## Behavior specification\n\n",
    "### list_sessions.feature\n\n```gherkin\n",
    include_str!("../../features/01_structure/list_sessions.feature"),
    "\n```\n\n",
    "### open_session.feature\n\n```gherkin\n",
    include_str!("../../features/01_structure/open_session.feature"),
    "\n```\n\n",
    "### message_parts.feature\n\n```gherkin\n",
    include_str!("../../features/01_structure/message_parts.feature"),
    "\n```\n\n",
    "### provenance.feature\n\n```gherkin\n",
    include_str!("../../features/01_structure/provenance.feature"),
    "\n```\n\n",
    "### extra_passthrough.feature\n\n```gherkin\n",
    include_str!("../../features/01_structure/extra_passthrough.feature"),
    "\n```\n\n",
    "### 01_usage_tokens.feature\n\n```gherkin\n",
    include_str!("../../features/01_structure/01_usage_tokens.feature"),
    "\n```\n\n",
    "### 02_tool_result_status.feature\n\n```gherkin\n",
    include_str!("../../features/01_structure/02_tool_result_status.feature"),
    "\n```\n\n",
    "### 03_session_graph.feature\n\n```gherkin\n",
    include_str!("../../features/01_structure/03_session_graph.feature"),
    "\n```\n\n",
    "### 04_turn_grouping.feature (pending)\n\n```gherkin\n",
    include_str!("../../features/01_structure/04_turn_grouping.feature"),
    "\n```\n\n",
    "### 05_compaction.feature\n\n```gherkin\n",
    include_str!("../../features/01_structure/05_compaction.feature"),
    "\n```\n\n",
    "### 07_tool_call_arguments.feature\n\n```gherkin\n",
    include_str!("../../features/01_structure/07_tool_call_arguments.feature"),
    "\n```\n\n",
    "### 08_inline_data.feature\n\n```gherkin\n",
    include_str!("../../features/01_structure/08_inline_data.feature"),
    "\n```\n\n",
    "### 09_optional_timestamp.feature (pending)\n\n```gherkin\n",
    include_str!("../../features/01_structure/09_optional_timestamp.feature"),
    "\n```\n\n",
    "### 10_role_mapping.feature (pending)\n\n```gherkin\n",
    include_str!("../../features/01_structure/10_role_mapping.feature"),
    "\n```\n\n",
    "### 11_session_metadata.feature (pending)\n\n```gherkin\n",
    include_str!("../../features/01_structure/11_session_metadata.feature"),
    "\n```\n\n",
    "### 12_raw_blob_escape_hatch.feature (pending)\n\n```gherkin\n",
    include_str!("../../features/01_structure/12_raw_blob_escape_hatch.feature"),
    "\n```\n"
)]
