# BotSpy

BotSpy pops open the conversation history of any coding agent: one adapter per agent (Claude Code, Cursor, Codex, Grok Bot, Antigravity, more later), one standard normalized schema, and one unified agent-session history.

The product is the `botspy` Rust **library crate** — library only for now: no binary, no executable, no server, no C API/FFI, no CLI. Metrics live in `examples/` as consumers of the public API, never in the library: a coding-session thanks-vs-F-bombs meter (`examples/meter`), local sentiment analysis with a VADER-style lexicon (`examples/sentiment`), and a stub showing where further session metrics would go (`examples/metrics_stub`). They import only the public API and the standard library, and never add a dependency to the core.

## How it works

```mermaid
flowchart TB
    subgraph disk["Agent session history on disk (read-only)"]
        CC["Claude Code: ~/.claude/projects JSONL transcripts"]
        CX["Codex: ~/.codex/sessions rollouts + state_5.sqlite index"]
        CU["Cursor: state.vscdb KV store + agent-transcripts"]
        GB["Grok Bot: sand-client-persistence blobs"]
        AG["Antigravity: conversation_summaries.db + brain transcripts"]
    end

    REG["Source registry (importer::create_source)<br/>SourceOptions: root override, home override"]

    subgraph layer["Typed adapter protocol (adapter::Adapter, importer::RecordStream)"]
        PROTO["discovery to SessionSummary lists<br/>streaming extraction one RawRecord at a time<br/>SkipCounter: malformed / unknown / partial<br/>snapshot::snapshot_sqlite: read-only WAL-safe copies for hot SQLite stores"]
        A1["adapters::claude_code"]
        A2["adapters::codex"]
        A3["adapters::cursor"]
        A4["adapters::grok_bot"]
        A5["adapters::antigravity"]
    end

    SCHEMA["Unified schema (session::Session)<br/>sessions, turns, messages, parts, raw blobs, provenance"]
    STORE["SessionStore: in-memory, FixtureAdapter-backed today"]
    QSTORE["SQLite-backed unified store (planned: Local Store Initiative)"]
    SCAN["Store-backed scanning tools (planned)"]
    QUERY["Query API, ch02_querying (planned)"]
    SEM["Semantic search: sqlite-vec + all-MiniLM-L6-v2 (planned)"]

    disk --> REG
    REG --> layer
    layer --> SCHEMA --> STORE
    STORE -.->|"planned"| QSTORE
    QSTORE -.->|"planned"| SCAN
    QSTORE -.->|"planned"| QUERY
    QUERY -.->|"planned"| SEM
```

The adapter layer in detail — the store-facing `Adapter` trait is implemented by the in-memory `FixtureAdapter` today; the per-agent sources already do discovery, streaming extraction, skip counting, and doctor diagnostics, and their store wiring is planned:

```mermaid
flowchart LR
    TRAIT["adapter::Adapter trait<br/>agent() / discover() / open()"]
    FIX["adapters::fixture<br/>in-memory FixtureAdapter (today's store impl)"]
    subgraph agents["Per-agent source adapters (adapters::)"]
        A1["claude_code<br/>JSONL, byte-offset resume"]
        A2["codex<br/>ordinal-ordered rollouts"]
        A3["cursor<br/>snapshot-copied KV store + CLI transcripts"]
        A4["grok_bot<br/>persistence blobs, seq order"]
        A5["antigravity<br/>transcripts + payload DB linkage"]
    end
    DOC["doctor diagnostics per source"]
    BDD["features/03_importers — Gherkin parity per agent"]
    WIRE["Bridge per-agent sources into the store (planned)"]

    TRAIT --- FIX
    TRAIT -.->|"planned"| WIRE
    agents --> DOC --> BDD
```

The query path is specified (Chapter 2, `features/02_querying/`) and lands with the Local Store Initiative:

```mermaid
flowchart LR
    CALLERS["Callers: scanning, examples, tools"]
    API["Query API (planned)"]
    LEX["SQLite FTS lexical query (planned)"]
    VEC["sqlite-vec embedding query (planned)"]
    MODEL["all-MiniLM-L6-v2 embeddings (planned)"]
    RANK["Hybrid ranking back to sessions, messages, parts (planned)"]

    CALLERS --> API
    API --> LEX
    API --> MODEL --> VEC
    LEX --> RANK
    VEC --> RANK
```

## Specs are the source of truth

BotSpy is a behavior-driven specification project. The Gherkin behavior specifications under `features/` are the backbone and the true source of the project; implementation code is considered generated from the specs. All planning is organized around features and their specs. Work starts by writing or refining the feature spec (with concrete examples), then the step definitions (Rust, `cucumber` crate), then the implementation. Run the specs with `cargo test --test bdd`.

## Development

```bash
cargo test              # unit tests + the bdd (Gherkin) suite
cargo test --test bdd   # only the Gherkin specs
cargo clippy --all-targets -- -D warnings
cargo fmt --all --check
```

Task management is Kanbus (see `AGENTS.md` and `CONTRIBUTING_AGENT.md`), driven by the `kbs` CLI.