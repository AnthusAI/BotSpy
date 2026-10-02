//! # Chapter 1 — Session history
//!
//! *Spec layer 1 · mirrors `features/01_session_history/` ·
//! [index](crate) · [next: the normalized schema](crate::spec::ch02_schema)*
//!
//! Everything in BotSpy starts from one idea: the conversations you have
//! with coding agents are already on your disk, one history per agent, in
//! one format per agent. BotSpy piles them into a single, ordinary thing
//! called a **session** — and then gives you the same three moves for every
//! agent: list what is there, open one, and walk through it in order.
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
//!   ([`Message`](crate::schema::Message); its anatomy is chapter 2's
//!   subject).
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
//!     messages: vec![],
//! });
//! let codex = Arc::new(FixtureAdapter::new(Agent::Codex));
//! codex.add_session(Session {
//!     id: "x1".into(),
//!     agent: Agent::Codex,
//!     project_id: "infra".into(),
//!     started_at: "2026-10-01T07:00:00Z".into(),
//!     last_activity_at: "2026-10-01T09:30:00Z".into(),
//!     messages: vec![],
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
//!                 provenance: None,
//!                 extra: None,
//!             })
//!             .collect(),
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
//! halfway. The adapter contract (chapter 4) makes this explicit: malformed
//! lines and unknown records are *skipped and counted, never fatal*, and a
//! missing source root means "absent", not "error". A session that opens is
//! the part of the history that parsed.
//!
//! **What "last activity" means.** `last_activity_at` is the timestamp of
//! the most recent thing that happened in the session, as recorded by the
//! agent itself. Real adapters derive it from the source of truth they
//! have: the last message timestamp when lines carry timestamps, otherwise
//! the session file's metadata (exactly the fallback the prototype used for
//! Cursor agent transcripts). The fixture adapter simply takes whatever you
//! put in the field. Because the sort key comes from the records, listing
//! is deterministic for a given snapshot of disk.
//!
#![doc = concat!(
    "## Behavior specification\n\n",
    "### list_sessions.feature\n\n```gherkin\n",
    include_str!("../../features/01_session_history/list_sessions.feature"),
    "\n```\n\n",
    "### open_session.feature\n\n```gherkin\n",
    include_str!("../../features/01_session_history/open_session.feature"),
    "\n```\n"
)]
