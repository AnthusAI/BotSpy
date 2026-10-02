# Chapter 3 — Importers / Change Data Capture (placeholder)

Specs for the CDC pipeline (detecting new or changed transcripts, the
normalization handoff, retries) and for each per-agent importer live here
once written. Bulk loading is not the model: each importer watches its
source and emits normalized records as they appear.

Per-agent importer specs: `claude_code/adapter.feature`, `cursor/adapter.feature`,
`codex/adapter.feature`, `grok_bot/adapter.feature`, `antigravity/adapter.feature`
(all tagged `@wip` until each importer's implementation lands).
Shared contract: `contract.feature` (implemented); WAL snapshotting:
`wal_snapshot.feature` (implemented). Narrative pages: `src/spec/ch03_importers/`.