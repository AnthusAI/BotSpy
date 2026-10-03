# BotSpy

Pop open the conversation history of your coding agents.

BotSpy taps the on-disk history of **Claude Code, Cursor, Codex, Grok Bot,
and Antigravity** through dedicated read-only adapters, normalizes every
record into one schema, and gives you one command line for all of them.
Everything stays local: no server, no telemetry, and it never writes to
your agent data.

## What it does

- `botspy sources` — shows every agent history BotSpy can read, where it
  lives, and how many sessions each one holds
- `botspy doctor` — per-source diagnostics: what was found, what was
  skipped, and why
- `botspy sessions` — one unified session listing across all agents, most
  recent activity first
- `botspy show <session>` — opens one session and walks its messages,
  part by part
- `botspy snapshot <db>` — takes a read-only, WAL-safe snapshot copy of a
  live SQLite store

## Supported agents and where their data lives

BotSpy reads each agent where it really stores its history (default roots
under your home directory; `--root` and `--home` override them):

| Agent | Source name | Reads |
|---|---|---|
| Claude Code | `claude_code` | `~/.claude/projects/` — per-session JSONL transcripts |
| Cursor | `cursor` | `~/.cursor/state.vscdb` — the IDE's SQLite KV store (composers, bubbles, content-addressed blobs), read through a WAL-safe snapshot copy |
| Codex | `codex` | `~/.codex/` — rollout JSONL under `~/.codex/sessions/`, plus the `state_5.sqlite` / `thread_history_1.sqlite` thread indexes at the root |
| Grok Bot | `grok_bot` | `~/.grok/sand-client-persistence/` — entry logs, plus `roster.json` and `cloud-agents.json` |
| Antigravity | `antigravity` | `~/.gemini/antigravity/` — the `conversation_summaries.db` index, brain transcripts, and per-conversation payload databases (read through snapshot copies) |

Live SQLite databases are never opened in place: BotSpy takes a snapshot
copy over a read-only connection, so the source file, its WAL, and its
shm are never written and the live app never blocks.

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

## Usage

### See what BotSpy can read

```console
$ botspy sources
SOURCE        ROOT                                        SESSIONS  STATUS
claude_code   /tmp/demo/.claude/projects                  4         ok
cursor        /tmp/demo/.cursor/state.vscdb               2         ok
codex         /tmp/demo/.codex                            2         ok
grok_bot      …mp/demo/.grok/sand-client-persistence      3         ok
antigravity   /tmp/demo/.gemini/antigravity               2         ok
```

A source whose files are missing is listed as `missing`, never hidden.
Restrict to one source with `--source claude_code`; use `--no-truncate`
for full ids and paths; point at a different home with `--home` (or the
`BOTSPY_HOME` environment variable). Add `-o json` or `-o ndjson` for
machine-readable output — machine modes never truncate.

### Check the health of your sources

```console
$ botspy doctor
claude_code   ok    3 project dirs, 4 transcripts, 0 issues
cursor        ok    2 composers, 4 bubbles, 0 issues
codex         ok    2 threads, 0 issues
grok_bot      ok    2 entry logs, 1 cloud agents, 0 issues
antigravity   ok    2 conversations, 2 transcripts, 0 issues
```

Doctor reports; it does not gate: skipped files and missing stores are
data about your machine, not failures — the exit code stays `0`. Narrow
it with `botspy doctor --source cursor`.

### List sessions across every agent

```console
$ botspy sessions
SESSION   AGENT         PROJECT               MSGS  LAST ACTIVITY         TITLE
00000000  claude_code   …ynth-workspace-beta     1  2026-10-01T09:20:00Z  —
00000000  claude_code   …nth-workspace-alpha     1  2026-10-01T09:10:00Z  —
conv-syn  antigravity   corpus sweep             0  2026-10-01T09:05:00Z  —
00000000  claude_code   …nth-workspace-alpha     4  2026-10-01T09:00:15Z  Synthetic parser session
conv-syn  antigravity   parser work              0  2026-10-01T09:00:00Z  —
solo-abc  claude_code   extra-demo               2  2026-09-15T12:05:00Z  —
00000000  grok_bot                               0  —                     —
00000000  grok_bot                               0  —                     —
composer  cursor        parser work              0  —                     —
composer  cursor        corpus sweep             0  —                     —
peer-syn  grok_bot                               0  —                     —
t-synth-  codex         /workspace/alpha         0  —                     —
t-synth-  codex         /workspace/alpha         0  —                     —
13 sessions (use --no-truncate for full ids)
```

Sessions are ordered by most recent activity. Filters combine:

```console
$ botspy sessions --since 2026-10-01T09:05:00Z
SESSION   AGENT         PROJECT               MSGS  LAST ACTIVITY         TITLE
00000000  claude_code   …ynth-workspace-beta     1  2026-10-01T09:20:00Z  —
00000000  claude_code   …nth-workspace-alpha     1  2026-10-01T09:10:00Z  —
conv-syn  antigravity   corpus sweep             0  2026-10-01T09:05:00Z  —
3 sessions (use --no-truncate for full ids)
```

- `--source <name>` (repeatable), `--project <id>`, `--limit <n>`
- `--since <timestamp>` / `--until <timestamp>` keep only sessions with
  activity in the window (RFC 3339 timestamps; invalid values are
  rejected, never silently ignored)
- `-o json` for the full summaries:

```console
$ botspy sessions --source claude_code -o json
[
  {
    "id": "00000000-0000-4000-8000-000000000003",
    "agent": "claude_code",
    "project_id": "-Users-synth-workspace-beta",
    "started_at": "2026-10-01T09:20:00Z",
    "last_activity_at": "2026-10-01T09:20:00Z",
    "message_count": 1,
    "metadata": {}
  },
  …
]
```

The example output above comes from BotSpy's synthetic test corpus, so
the paths and titles are fixtures, not real sessions.

### Open one session

```console
$ botspy show solo-abc123
Session solo-abc (claude_code, extra-demo) — 2 messages, started 2026-09-15T12:00:00Z
#1 user 2026-09-15T12:00:00Z
  · text  hello from the fixture
#2 user 2026-09-15T12:05:00Z
  · text  second message
```

A session is addressed by the agent's own stable id, or any unambiguous
prefix of one (`botspy show solo-abc123` and a longer full id both work).
Drill into a single message by its 1-based ordinal:

```console
$ botspy show solo-abc123 --message 2
Session solo-abc (claude_code, extra-demo) — 2 messages, started 2026-09-15T12:00:00Z
#2 user 2026-09-15T12:05:00Z
  · text  second message
```

`-o json` prints the whole session; `-o ndjson` streams one message per
line. An unknown id is an error, never an empty result; an ambiguous
prefix is an error listing the candidates, never a guess.

## Everything stays local

- Adapters are read-only by construction — the protocol has no surface
  for writing to a source.
- Live SQLite stores in WAL mode are read through snapshot copies; the
  source file, WAL, and shm are never touched.
- There is no network, no telemetry, no server. BotSpy reads local files
  and prints local answers; nothing leaves the machine.

Both read-only behavior and WAL-safe snapshotting are pinned by executed
specifications: [`features/03_importers/contract.feature`](features/03_importers/contract.feature)
and [`features/03_importers/wal_snapshot.feature`](features/03_importers/wal_snapshot.feature).

## Docs and spec

- Documentation index: [`docs/README.md`](docs/README.md) — the
  architecture (C4 diagrams and the top-level data flow), the CLI, the
  local store, search, the source adapters, and the sensitive-data
  inventory:
  - [`docs/architecture.md`](docs/architecture.md)
  - [`docs/cli.md`](docs/cli.md)
  - [`docs/store.md`](docs/store.md)
  - [`docs/search.md`](docs/search.md)
  - [`docs/sensitive-data.md`](docs/sensitive-data.md)
  - [`docs/sources/`](docs/sources/) — one document per source adapter
  - [`docs/diagrams.md`](docs/diagrams.md) — how to update and
    re-render the diagrams
- API documentation: [docs.rs/botspy](https://docs.rs/botspy)
- The Gherkin specification lives under
  [`features/`](features/) and runs with `cargo test --test bdd`; the
  rustdoc (`cargo doc --open`) embeds the specs, one chapter per layer
  (structure, querying, importers, examples, CLI).
- To add support for another agent, follow
  [`features/03_importers/README.md`](features/03_importers/README.md).
- The original design narrative and detailed FAQ are preserved in
  [`docs/DESIGN.md`](docs/DESIGN.md).
- Repository: [github.com/AnthusAI/BotSpy](https://github.com/AnthusAI/BotSpy)

## License

MIT — see [LICENSE](LICENSE).