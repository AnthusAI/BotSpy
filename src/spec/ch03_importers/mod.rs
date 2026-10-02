//! # Chapter 3 — Importers / Change Data Capture
//!
//! *Spec layer 3 · mirrors `features/03_importers/` ·
//! [index](crate) ·
//! [previous: querying](crate::spec::ch02_querying)*
//!
//! Chapters 1 and 2 describe what the normalized history looks like and
//! how it is queried. Chapter 3 is how data gets there: **importers**,
//! organized as a change-data-capture pipeline. This chapter owns the
//! pipeline mechanics; each per-agent importer is only the per-agent
//! translation of the same pipeline.
//!
//! ## The pipeline
//!
//! An importer watches its agent's source — a transcript directory, a
//! SQLite database, a set of JSON blobs — and emits normalized records as
//! they appear:
//!
//! 1. **Detection.** The importer notices new or changed transcripts.
//!    Append-only sources (Claude Code JSONL) are followed by offset;
//!    file-per-session sources by mtime and size; SQLite-backed sources by
//!    row timestamps and indexes. Detection is incremental by design.
//! 2. **Normalization handoff.** Each detected record is handed to the
//!    normalization step, which maps it into the chapter-1 structure —
//!    messages, parts, provenance — and emits it downstream.
//! 3. **Retries.** Sources are live app data (hot WAL databases, files
//!    being appended), so reads can fail mid-record. A failed read is
//!    retried with backoff; a permanently malformed record is skipped and
//!    counted, never fatal.
//!
//! ## Bulk loading is not the model
//!
//! Importers do not "load everything once". Each importer watches its
//! source and emits normalized records as they appear — the store is fed
//! by a stream of changes, not by periodic full imports. A first run may
//! catch up on existing transcripts, but the steady state is CDC: watch,
//! detect, normalize, emit.
//!
//! ## Per-agent importers
//!
//! Each adapter is just the per-agent translation of the pipeline above:
//!
//! - [Claude Code](crate::spec::ch03_importers::claude_code) — JSONL
//!   transcripts under `~/.claude/projects/`, including sub-agent files.
//! - [Cursor](crate::spec::ch03_importers::cursor) — the IDE KV store and
//!   the CLI agent transcripts under `~/.cursor/`.
//! - [Codex](crate::spec::ch03_importers::codex) — rollout JSONL under
//!   `~/.codex/sessions/` plus the SQLite thread indexes.
//! - [Grok Bot](crate::spec::ch03_importers::grok_bot) —
//!   `sand-client-persistence` blobs; partial local views by design.
//! - [Antigravity](crate::spec::ch03_importers::antigravity) —
//!   per-conversation DBs and `transcript.jsonl` under
//!   `~/.gemini/antigravity/`.
//!
//! ## Status: contract specified
//!
//! The adapter contract — the in-process face of this pipeline — is
//! specified in [`contract.feature`](../../../features/03_importers/contract.feature)
//! and embedded below, tagged `@wip` until its implementation lands. The
//! per-agent specs (below) translate the same pipeline to each source.
//!
#![doc = concat!(
    "## Behavior specification\n\n",
    "### contract.feature\n\n```gherkin\n",
    include_str!("../../../features/03_importers/contract.feature"),
    "\n```\n"
)]

pub mod antigravity;
pub mod claude_code;
pub mod codex;
pub mod cursor;
pub mod grok_bot;
