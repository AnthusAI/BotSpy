//! # Codex importer
//!
//! *[Chapter 3: Importers / CDC](crate::spec::ch03_importers) ·
//! [index](crate) · mirrors `features/03_importers/codex/`*
//!
//! The Codex importer is the per-agent translation of the CDC pipeline for
//! Codex's storage: rollout JSONL event logs under
//! `~/.codex/sessions/YYYY/MM/DD/`, indexed by `state_5.sqlite` (threads,
//! projects, spawn edges) and `thread_history_1.sqlite` (turn and item
//! projections with rollout byte offsets).
//!
//! ## Pipeline translation
//!
//! - **Detection**: watch the session index and the threads table for
//!   new/updated rollouts; follow each rollout by byte offset (the
//!   projection tables store the offsets).
//! - **Normalization handoff**: every record carries an explicit
//!   `ordinal` — the authoritative order; `response_item` records become
//!   messages (roles user/assistant/developer, reasoning summaries, tool
//!   calls and outputs); `token_usage_record` and `compacted` map to the
//!   pending usage and compaction specs.
//! - **Retries**: rollouts are paginated/compacted and appended live;
//!   partial trailing lines are retried, malformed records skipped and
//!   counted.
//!
//! ## Status: specified next
//!
//! No scenarios exist yet; `features/03_importers/codex/` is a placeholder
//! and this page is the outline its specs will be written against.
