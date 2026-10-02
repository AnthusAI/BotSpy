//! Chapters of the BotSpy specification, one per spec layer.
//!
//! Each chapter mirrors one folder under `features/` and embeds the actual
//! `.feature` files it is backed by, so the rendered docs and the executable
//! specifications come from a single source of truth: the Gherkin files.
//! The narrative prose lives here in `src/spec/` (chosen over
//! `features/*/README.md` so chapter navigation, intra-doc links, and
//! doctests stay inside rustdoc); the specs stay in `features/` and are
//! pulled in with `include_str!`.
//!
//! - [Chapter 1: Session history](crate::spec::ch01_session_history) —
//!   `features/01_session_history/`
//! - [Chapter 2: Normalized schema](crate::spec::ch02_schema) —
//!   `features/02_schema/`

pub mod ch01_session_history;
pub mod ch02_schema;
