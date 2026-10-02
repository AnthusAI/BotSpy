//! Chapters of the BotSpy specification, one per chapter.
//!
//! Each chapter mirrors one folder under `features/` and embeds the actual
//! `.feature` files it is backed by, so the rendered docs and the
//! executable specifications come from a single source of truth: the
//! Gherkin files. The narrative prose lives here in `src/spec/` (chosen
//! over `features/*/README.md` so chapter navigation, intra-doc links, and
//! doctests stay inside rustdoc); the specs stay in `features/` and are
//! pulled in with `include_str!`.
//!
//! - [Chapter 1: Structure](crate::spec::ch01_structure) —
//!   `features/01_structure/` (absorbs the old session-history and
//!   normalized-schema layers)
//! - [Chapter 2: Querying](crate::spec::ch02_querying) —
//!   `features/02_querying/` (specified next; implemented by the Local
//!   Store Initiative, BOTSPY-c7795b)
//! - [Chapter 3: Importers / CDC](crate::spec::ch03_importers) —
//!   `features/03_importers/` (specified next), with per-agent
//!   sub-chapters for Claude Code, Cursor, Codex, Grok Bot, and
//!   Antigravity

pub mod ch01_structure;
pub mod ch02_querying;
pub mod ch03_importers;
