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

## How to add a sixth agent

Every source obeys the same protocol: discovery finds sessions under the
source's roots and is incremental by identity; extraction streams records
one at a time with a `SkipCounter` (malformed / unknown / partial — skipped
and counted, never fatal); and the source is only ever read — there is no
write surface, and hot SQLite stores go through `snapshot::snapshot_sqlite`
(a read-only WAL-safe snapshot copy). Implement a source with the same
`discover()` / `extract()` shape as the five (`ClaudeCodeSource`,
`CursorSource` + `CursorCliSource`, `CodexSource`, `GrokBotSource`,
`AntigravitySource`), map its records onto the chapter-1 schema (park
anything you do not model in `Part::Extra` and each part's `extra`), and
write the Gherkin parity spec with synthetic fixtures under
`<agent>/adapter.feature`.

```mermaid
classDiagram
    class SessionStore {
        +register(adapter) void
        +list_sessions() Vec~SessionSummary~
        +open(id) Result~Session, UnknownSession~
    }
    class Adapter {
        <<trait>>
        +agent() Agent
        +discover() Vec~SessionSummary~
        +open(id) Option~Session~
    }
    class FixtureAdapter {
        +add_session(session) void
    }
    class RecordStream {
        <<trait>>
        +next_record() Option~RawRecord~
        +skipped() SkipCounter
        +peak_buffered() usize
    }
    class SkipCounter {
        +u64 malformed
        +u64 unknown
        +u64 partial
        +total() u64
    }
    class SourceOptions {
        +Option~PathBuf~ root
        +Option~PathBuf~ home
    }
    class create_source {
        <<registry function>>
        +create_source(name, options) Result~MemorySource, UnknownSource~
    }
    class ClaudeCodeSource {
        +discover() Vec~SessionSummary~
        +extract(session_id) Option~ClaudeExtraction~
    }
    class snapshot_sqlite {
        <<utility>>
        +snapshot_sqlite(source, dir) Result~PathBuf, SnapshotError~
    }

    SessionStore o-- "0..*" Adapter : holds registered adapters
    Adapter <|.. FixtureAdapter : today's store-backed impl
    ClaudeCodeSource ..> SkipCounter : extraction counts skips
    ClaudeCodeSource ..> snapshot_sqlite : used by CursorSource and AntigravitySource
    note for ClaudeCodeSource "Same discover()/extract() shape, each returning<br/>its own discovery diagnostics type:<br/>CursorSource + CursorCliSource, CodexSource,<br/>GrokBotSource, AntigravitySource.<br/>Bridging the sources into the Adapter-backed store is planned."
```
