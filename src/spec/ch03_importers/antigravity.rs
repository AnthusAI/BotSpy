//! # Antigravity importer
//!
//! *[Chapter 3: Importers / CDC](crate::spec::ch03_importers) ·
//! [index](crate) · mirrors `features/03_importers/antigravity/`*
//!
//! The Antigravity importer is the per-agent translation of the CDC
//! pipeline for Antigravity's storage: one SQLite DB per conversation
//! under `~/.gemini/antigravity/conversations/` (protobuf payload blobs),
//! the `conversation_summaries.db` index, and the human-readable
//! `brain/<id>/.system_generated/logs/transcript.jsonl`.
//!
//! ## Pipeline translation
//!
//! - **Detection**: watch `conversation_summaries.db` for new/updated
//!   conversations; follow each conversation's `transcript.jsonl` by line
//!   offset. SQLite DBs are copied out (hot WAL) before reading.
//! - **Normalization handoff**: prefer `transcript.jsonl` (readable types:
//!   `USER_INPUT`, `PLANNER_RESPONSE`, `CHECKPOINT`, `ERROR_MESSAGE`,
//!   `step_index`, `created_at`); the protobuf DB is the lossless fallback
//!   for linkage the JSONL lacks (tool-call ids, tool results). CHECKPOINT
//!   records map to the pending compaction spec.
//! - **Retries**: WAL files change under the importer; reads are retried
//!   against a snapshot copy; malformed steps are skipped and counted.
//!
//! ## Status: specified next
//!
//! No scenarios exist yet; `features/03_importers/antigravity/` is a
//! placeholder and this page is the outline its specs will be written
//! against.
