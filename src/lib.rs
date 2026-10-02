//! BotSpy pops open the conversation history of any coding agent.
//!
//! One adapter per agent (Claude Code, Cursor, Codex, Grok Bot, Antigravity,
//! more later), one standard normalized schema, one unified agent-session
//! history. The Gherkin behavior specifications under `features/` are the
//! backbone and the true source of the project; implementation code is
//! considered generated from the specs.

pub mod adapter;
pub mod adapters;
pub mod schema;
pub mod session;

pub use adapter::Adapter;
pub use adapters::fixture::FixtureAdapter;
pub use schema::{Agent, KnownPart, Message, Part, Provenance, Role, Timestamp, SCHEMA_VERSION};
pub use session::{Session, SessionStore, SessionSummary, UnknownSession};
