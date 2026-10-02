//! # Grok Bot importer
//!
//! *[Chapter 3: Importers / CDC](crate::spec::ch03_importers) ·
//! [index](crate) · mirrors `features/03_importers/grok_bot/`*
//!
//! The Grok Bot importer is the per-agent translation of the CDC pipeline
//! for Grok Bot's storage: plain-JSON `sand-client-persistence` blobs
//! (chat entry logs keyed by opaque hash names, an agent roster, and
//! cloud-agent records), plus `~/.grokbot/` settings.
//!
//! ## Pipeline translation
//!
//! - **Detection**: watch the persistence directory for new/changed blob
//!   files; map blob names to agents via the roster and `agentId` pointer
//!   blobs.
//! - **Normalization handoff**: entry-log records become messages in `seq`
//!   order (user/assistant, personas via author, voice-call entries as raw
//!   passthrough); cloud-agent records become session stubs with status
//!   and PR metadata pending `11_session_metadata.feature`.
//! - **Retries and partiality**: entry logs are capped at 200 entries
//!   locally with the full history server-side — sessions are partial
//!   local views by design, flagged as such pending
//!   `12_raw_blob_escape_hatch.feature`.
//!
//! ## Status: specified
//!
//! The scenarios live in [`adapter.feature`](../../../features/03_importers/grok_bot/adapter.feature)
//! and are embedded below, tagged `@wip` until this importer's
//! implementation lands.
//!
#![doc = concat!(
    "## Behavior specification\n\n",
    "### grok_bot/adapter.feature\n\n```gherkin\n",
    include_str!("../../../features/03_importers/grok_bot/adapter.feature"),
    "\n```\n"
)]
//!
//! No scenarios exist yet; `features/03_importers/grok_bot/` is a
//! placeholder and this page is the outline its specs will be written
//! against.
