//! # BotSpy — one library for every coding agent's conversation history
//!
//! BotSpy pops open the conversation history of any coding agent and
//! normalizes it into **one unified agent-session history**. One adapter per
//! agent — Claude Code, Cursor, Codex, Grok Bot, Antigravity, more later —
//! taps that agent's on-disk chat histories; the normalized schema makes
//! user messages, agent messages, thinking traces, tool calls and their
//! results, attachments, and per-record provenance look the same no matter
//! which agent produced them.
//!
//! ## What BotSpy is not
//!
//! BotSpy is a **library only**: no CLI, no binary, no server, no C
//! interface/FFI. What ships is the `botspy` library crate. Metrics such as
//! the coding-session thanks-vs-F-bombs meter or local sentiment analysis
//! are **example consumers** of the library, not part of it.
//!
//! ## Specs are the source of truth
//!
//! BotSpy is a behavior-driven specification project. The Gherkin
//! specifications under `features/` are the backbone and the true source of
//! the project; implementation code is considered generated from the specs.
//! These docs are therefore organized around the specifications, not around
//! code modules: each chapter below is one spec layer and mirrors one folder
//! under `features/`, and every chapter embeds the actual `.feature` files
//! it is backed by.
//!
//! ## How these docs are organized
//!
//! Each chapter is one spec layer, mirroring the `features/` tree:
//!
//! 1. [Session history](crate::spec::ch01_session_history) —
//!    what a session is; listing sessions across agents sorted by last
//!    activity; opening a session by id; walking its messages in order; the
//!    unknown-id error. Mirrors `features/01_session_history/`.
//! 2. [Normalized schema](crate::spec::ch02_schema) —
//!    messages and roles; typed parts (text, thinking, tool calls and
//!    results, attachments, system); per-message provenance; the `extra`
//!    escape hatch for agent-specific fields. Mirrors `features/02_schema/`.
//! 3. Query and export — filter by agent, kind, time, and project; JSON and
//!    JSONL export. Mirrors `features/03_query_export/` (specified next).
//! 4. Adapters — one per agent, sharing the [`crate::adapter::Adapter`]
//!    contract. Mirrors `features/04_adapters/` (specified next).
//!
//! ## A 60-second drill-down
//!
//! Build a store, register an adapter with in-memory fixture data (real
//! adapters read agent directories instead), list sessions, open one, and
//! walk its messages:
//!
//! ```
//! use botspy::{Agent, FixtureAdapter, KnownPart, Message, Part, Role, Session, SessionStore};
//! use std::sync::Arc;
//!
//! let mut store = SessionStore::new();
//! let adapter = Arc::new(FixtureAdapter::new(Agent::ClaudeCode));
//! adapter.add_session(Session {
//!     id: "c1".into(),
//!     agent: Agent::ClaudeCode,
//!     project_id: "demo".into(),
//!     started_at: "2026-10-01T09:00:00Z".into(),
//!     last_activity_at: "2026-10-01T10:00:00Z".into(),
//!     messages: vec![Message {
//!         role: Role::User,
//!         parts: vec![Part::Known(KnownPart::Text {
//!             text: "please fix the bug".into(),
//!             extra: None,
//!         })],
//!         timestamp: "2026-10-01T09:01:00Z".into(),
//!         provenance: None,
//!         extra: None,
//!     }],
//! });
//! store.register(adapter);
//!
//! // Sessions from every registered agent, most recent activity first.
//! let summaries = store.list_sessions();
//! assert_eq!(summaries[0].id, "c1");
//!
//! // Open one and walk its messages in order.
//! let session = store.open("c1")?;
//! assert_eq!(session.messages.len(), 1);
//! assert!(matches!(&session.messages[0].parts[0], Part::Known(KnownPart::Text { .. })));
//!
//! // An id no adapter knows is an error, never a silent empty result.
//! assert!(store.open("nope").is_err());
//! # Ok::<(), botspy::UnknownSession>(())
//! ```
//!
//! Continue with [chapter 1: session history](crate::spec::ch01_session_history).

pub mod adapter;
pub mod adapters;
pub mod schema;
pub mod session;
pub mod spec;

pub use adapter::Adapter;
pub use adapters::fixture::FixtureAdapter;
pub use schema::{Agent, KnownPart, Message, Part, Provenance, Role, Timestamp, SCHEMA_VERSION};
pub use session::{Session, SessionStore, SessionSummary, UnknownSession};
