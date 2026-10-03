# BotSpy

Pop open the conversation history of your coding agents.

BotSpy reads the on-disk session histories of **Claude Code, Cursor,
Codex, Grok Bot, and Antigravity** — locally, and read-only. One
adapter per agent, one normalized schema, one local SQLite store with
full-text and semantic search. Everything stays on your machine: no
server, no telemetry, and no writes to your agent data.

## The system in one picture

BotSpy is one local CLI and library on your machine. The coding agents
write their session data to their own local files; BotSpy reads those
files read-only and shows you the results.

<picture>
  <source media="(prefers-color-scheme: dark)" srcset="https://raw.githubusercontent.com/AnthusAI/BotSpy/develop/docs/diagrams/context.dark.png">
  <img src="https://raw.githubusercontent.com/AnthusAI/BotSpy/develop/docs/diagrams/context.light.png" alt="C4 context diagram: user, BotSpy, and the coding agents on the machine">
</picture>

## What BotSpy does

- **Reads, never writes.** Each agent's history is read where it lives
  on disk. Live SQLite stores are copied first through a read-only,
  WAL-safe snapshot, so the source file, its WAL, and its shm are never
  touched and the live app never blocks.
- **One normalized schema.** Every record from every agent becomes a
  `Session`, `Message`, and `Part`, with provenance back to the exact
  source file, line, or row. Records an adapter does not know are
  skipped and counted, never fatal.
- **One local store.** One SQLite database at `~/.botspy/store.db`
  (WAL mode) holds sessions, messages, parts, a full-text index (FTS5),
  and a vector index (sqlite-vec, with embeddings from
  all-MiniLM-L6-v2 computed on your CPU).
- **Search three ways.** Full-text search, semantic search, or the
  hybrid fusion of both.
- **One command line over all agents.** `botspy sources`, `doctor`,
  `sessions`, `show`, and `snapshot` — a thin shell over the library;
  every verb is one library call plus rendering.
- **Sensitive-data visibility.** BotSpy never hides where session
  content lives. Every architecture diagram marks the sensitive places,
  and [`docs/sensitive-data.md`](https://github.com/AnthusAI/BotSpy/blob/develop/docs/sensitive-data.md)
  lists every path BotSpy reads, copies, or writes.

## How the data flows

Five agents' files feed five source adapters. The adapters convert
their records into the unified schema, ingest writes the store, and the
query surface answers over it — text, semantic, or hybrid. The diagram
shows every source and every exact path.

<picture>
  <source media="(prefers-color-scheme: dark)" srcset="https://raw.githubusercontent.com/AnthusAI/BotSpy/develop/docs/diagrams/dataflow.dark.png">
  <img src="https://raw.githubusercontent.com/AnthusAI/BotSpy/develop/docs/diagrams/dataflow.light.png" alt="Top-level data flow: five sources, snapshot copies, adapters, unified schema, ingest, store, query, output">
</picture>

## One machine, one store

On one machine there are four data places and one process: the agent
session files (read-only), temporary snapshot copies for live SQLite
files, the local store at `~/.botspy/store.db`, and a model cache that
holds model weights only, no session data. The BotSpy CLI is the one
process; it holds no data of its own. Delete `~/.botspy/` and no BotSpy
copy of your sessions remains.

<picture>
  <source media="(prefers-color-scheme: dark)" srcset="https://raw.githubusercontent.com/AnthusAI/BotSpy/develop/docs/diagrams/containers.dark.png">
  <img src="https://raw.githubusercontent.com/AnthusAI/BotSpy/develop/docs/diagrams/containers.light.png" alt="C4 container diagram: CLI, agent files, snapshot copies, store, model cache">
</picture>

## Everything stays local

- Adapters are read-only by construction — the protocol has no surface
  for writing to a source.
- Live SQLite stores in WAL mode are read through snapshot copies; the
  source file, WAL, and shm are never written.
- There is no network, no telemetry, no server. BotSpy reads local
  files and prints local answers; nothing leaves the machine.
- **One honest exception:** semantic search runs a real embedding model
  (quantized all-MiniLM-L6-v2 via ONNX on your CPU). Its weights and
  tokenizer are fetched once — only when you explicitly run the setup
  step (`cargo run --example setup_models`), digest-verified, and
  cached under `~/.botspy/models/` (`BOTSPY_MODELS` overrides the
  location). After that, everything is fully offline.

Both read-only behavior and WAL-safe snapshotting are pinned by
executed specifications:
[`features/03_importers/contract.feature`](https://github.com/AnthusAI/BotSpy/blob/develop/features/03_importers/contract.feature)
and
[`features/03_importers/wal_snapshot.feature`](https://github.com/AnthusAI/BotSpy/blob/develop/features/03_importers/wal_snapshot.feature).

## Supported agents and where their data lives

| Agent | Source name | Reads |
|---|---|---|
| Claude Code | `claude_code` | `~/.claude/projects/` — per-session JSONL transcripts |
| Cursor | `cursor` | `~/.cursor/state.vscdb` and CLI agent transcripts, read through a WAL-safe snapshot copy |
| Codex | `codex` | `~/.codex/sessions/` rollout JSONL plus the thread index at the root |
| Grok Bot | `grok_bot` | `~/.grok/sand-client-persistence/` entry logs, plus roster and cloud-agent files |
| Antigravity | `antigravity` | `~/.gemini/antigravity/` summaries DB, brain transcripts, and payload databases, read through snapshot copies |

Default roots sit under your home directory; `--root` and `--home`
override them. The full inventory of every exact path, in every
category, is in
[`docs/sensitive-data.md`](https://github.com/AnthusAI/BotSpy/blob/develop/docs/sensitive-data.md).

## Install

```bash
cargo install botspy --locked
```

Prerequisites:

- A Rust toolchain — install via [rustup](https://rustup.rs).
- A C compiler: SQLite is bundled and compiled at build time.
  - macOS: Xcode Command Line Tools (`xcode-select --install`)
  - Linux: `build-essential` (Debian/Ubuntu) or your distribution's
    `gcc`/`clang` packages
  - Windows: the Microsoft C++ Build Tools (MSVC)

BotSpy is built and tested on Linux (x86_64 and arm64, glibc and musl),
macOS (Apple Silicon and Intel), and Windows (MSVC).

## Learn more

The docs pages are the deep reference; this README stays the overview.

- [`docs/architecture.md`](https://github.com/AnthusAI/BotSpy/blob/develop/docs/architecture.md)
  — the C4 views and the end-to-end data flow
- [`docs/cli.md`](https://github.com/AnthusAI/BotSpy/blob/develop/docs/cli.md)
  — the `botspy` command: every verb, flag, output mode, and exit code
- [`docs/store.md`](https://github.com/AnthusAI/BotSpy/blob/develop/docs/store.md)
  — the local store: where it lives, its tables, ingest, and
  incremental refresh; how to use BotSpy as a library
- [`docs/search.md`](https://github.com/AnthusAI/BotSpy/blob/develop/docs/search.md)
  — text search, semantic search, hybrid search, and the one-time model
  download
- [`docs/sensitive-data.md`](https://github.com/AnthusAI/BotSpy/blob/develop/docs/sensitive-data.md)
  — every path BotSpy reads, copies, or writes
- [`docs/sources/`](https://github.com/AnthusAI/BotSpy/blob/develop/docs/sources/)
  — one document per source adapter, plus the shared adapter protocol
- [`docs/diagrams.md`](https://github.com/AnthusAI/BotSpy/blob/develop/docs/diagrams.md)
  — the diagram sources and how to re-render them
- [`docs/DESIGN.md`](https://github.com/AnthusAI/BotSpy/blob/develop/docs/DESIGN.md)
  — the original design narrative and detailed FAQ
- API documentation: [docs.rs/botspy](https://docs.rs/botspy)
- Executable specifications: the Gherkin behavior specs live under
  [`features/`](https://github.com/AnthusAI/BotSpy/blob/develop/features/)
  and run with `cargo test --test bdd`; the specs are the source of
  truth, and the implementation is generated from them
- Example consumers of the public API: [`examples/`](https://github.com/AnthusAI/BotSpy/blob/develop/examples/)
  — a session meter, local sentiment analysis, and a metrics stub
- Repository: [github.com/AnthusAI/BotSpy](https://github.com/AnthusAI/BotSpy)

## License

MIT — see [LICENSE](LICENSE).