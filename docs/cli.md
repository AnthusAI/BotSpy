# The BotSpy CLI

The `botspy` command is a thin shell over the library. Every verb is
one library call plus rendering. The CLI holds no state of its own: it
reads the agent files, and it writes only BotSpy's own store and
snapshot files.

Today the CLI has six verbs: `sources`, `sessions`, `show`, `doctor`,
`snapshot`, and `scan`. The read-only verbs build an in-memory
`SessionStore` over the live source adapters and read from it. `scan`
reads and writes the local store at `<home>/.botspy/store.db`. The
other store-backed verbs (`search`, `import`, `stats`, `watch`) are
planned; open Kanbus task BOTSPY-98386a tracks them.

## Global flags

Every verb accepts these flags:

| Flag | Effect |
| --- | --- |
| `-s, --source <name>` | Select one source. Repeat the flag to select several. Names are the registry's own: `claude_code`, `cursor`, `codex`, `grok_bot`, `antigravity`. The names are exact; the CLI accepts no aliases. |
| `--root <path>` | Use `<path>` as the source root instead of the default. Requires exactly one `--source`. |
| `--home <path>` | Derive all roots and the store path from `<path>` instead of `$HOME`. |
| `-o, --output <mode>` | Output mode: `human` (default), `json`, or `ndjson`. |
| `--no-truncate` | Show full ids and paths in human mode. Machine modes never truncate. |
| `--version` | Print the version. |

The home directory comes from `--home`, else the `BOTSPY_HOME`
environment variable, else `$HOME`. The store lives at
`<home>/.botspy/store.db`; snapshots at `<home>/.botspy/snapshots/`.

## Output modes

- `human` — tables and text for a person. Truncates long values unless
  `--no-truncate` is set. Drops decoration when piped.
- `json` — serde JSON of the real types (`Session`, `SessionSummary`,
  `Message`, `Part`, ...). No second schema to keep in sync.
- `ndjson` — one record per line, streamed. Honors the laziness rule:
  taking the first ten sessions does not open the other two hundred.

## Exit codes

| Code | Meaning |
| --- | --- |
| 0 | Success — including empty results and skipped records. |
| 1 | Target not found — unknown session id or unknown source. |
| 2 | Usage error — bad flags. |
| 3 | Source failure — unreadable root or store, snapshot error. |

Skipped records are data, not errors. Skips never change the exit
code. The adapters count skips; the planned store-backed verbs will
print the skip summary (BOTSPY-98386a).

## The verbs

```
botspy
  sources    inspect the source registry: roots, session counts, status
  sessions   list sessions across every source, newest activity first
  show       open one session; walk its messages and parts
  doctor     per-source diagnostics
  snapshot   take a WAL-safe snapshot of a SQLite source and inspect it
  scan       mine the adapters into the local store, in a loop (CDC pass, no pruning)
```

Example output in this document comes from the synthetic test corpus,
so ids, paths, and titles are fixtures, not real sessions.

### `botspy sessions`

One unified listing across every registered adapter, most recent
activity first. `-o json` is exactly the serialized `[SessionSummary]`.

Flags: `--source`, `--project <id>`, `--since`/`--until` (RFC 3339
timestamps), `--limit N`. Planned flags (BOTSPY-98386a): `--tree`
(render the session graph: parents, subagents, forks, battles),
`--kind <part-kind>` (sessions containing that part kind).

```console
$ botspy sessions --project example-app --since 2026-08-28
SESSION   AGENT        PROJECT  MESSAGES  LAST ACTIVITY      TITLE
synth-1a2b0008  claude_code  example-app    117  2026-08-02 15:41  fix importer flake
synth-1a2b0009  cursor       example-app     40  2026-08-01 18:04  review: adapters
3 sessions (use --no-truncate for full ids)
```

### `botspy show <session-id>`

Open one session and walk it: a header line, then messages with their
parts. A session is addressed by its native id, or by any unambiguous
id prefix. An ambiguous prefix is an error that lists the candidates;
an unknown id exits 1.

Flags: `--message N` (1-based message ordinal). Planned flags
(BOTSPY-98386a): `--turn <id>`, `--kind <part-kind>`, `--role`,
`--thinking` (expand thinking traces), `--usage` (per-turn and thread
usage), `--raw` (the `extra` passthrough untouched), `--graph`
(parent/subagent/fork/battle/peer links and compaction windows).

```console
$ botspy show synth-1a2b --message 14
Session synth-1a2b0008 (claude_code, example-app) — 117 messages, started 2026-08-30T09:12Z
#14 assistant 2026-08-30T09:31Z turn t-7
  · text       Let me trace the snapshot path…
  · thinking   (190 lines)
  · tool_call  Read path="src/snapshot.rs" [ok]
  · tool_result  412 B
```

### `botspy scan`

Mine the adapters into the local store: one refresh-style CDC pass that
discovers sessions and lands new messages. Pruning is disabled — a
source that reports nothing (an unreadable root, an agent that has not
run yet) can never delete stored history. On an empty store the first
scan is the cold ingest; later scans touch only what changed.

A scan runs one full pass to start, then keeps running: it re-scans
every `--interval <SECONDS>` seconds (default 60) until it is
interrupted. The interval is measured from the end of the previous
pass, so a pass that outlasts the interval can never overlap the next
one — the loop is strictly one pass at a time. `scan` is an ordinary
foreground process: background it with your own tooling (`&`, tmux,
a process manager); there is no daemon, no pidfile, no detach. An
interrupt (SIGINT/SIGTERM) lets the current pass finish and commit —
the pass is one store transaction, so nothing is left half-written —
then stops the loop with exit code 130. Errors inside one pass are
reported in that pass's output and never kill the loop; the next pass
tries again. Pruning stays off for the whole loop.

Flags: `--source`, `--since <cutoff>` (an RFC 3339 timestamp, an ISO
date `YYYY-MM-DD`, or a relative duration like `7d`/`24h`),
`--store <path>` (default `<home>/.botspy/store.db`), `--dry-run`,
`--interval <SECONDS>` (default 60; values below 1 are a usage error),
`--once` (run exactly one pass and exit — the shape scripts and CI
want), plus output globals. Each pass emits a full report in the
chosen output mode — written and flushed as the pass finishes, so
piped or redirected output streams live: human gets a `pass N` banner
per report, JSON gets
one document per pass (with a `pass` field), and NDJSON streams one
line per source per pass. The cutoff filters what is read; nothing
already stored is ever deleted — sessions older than the cutoff are
skipped, not dropped. Zero sessions and open errors are reported per
source, never silently swallowed.

```console
$ botspy scan --once --since 7d
store  ~/.botspy/store.db
SOURCE       ADDED  UPDATED  UNCHANGED  SKIPPED  ERRORS  MESSAGES
claude_code      2        1         11        3       0        +4
cursor           0        1          9        0       0        +2
scanned 2 sources in 1.7s: 2 added, 2 updated, 20 unchanged, 3 skipped, 0 errors

$ botspy scan --interval 60
pass 1
store  ~/.botspy/store.db
...
```

### `botspy sources`

The source registry as a human view: every known source name, its
resolved root (after `--root`/`--home` overrides), the adapter behind
it, the session count, and a status. Discovery runs live: every call
walks the sources again. The verb prints no skip stats.

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

Flags: `--source` (detail one or more). Planned flag (BOTSPY-98386a):
`-v, --verbose` — per-source discovery detail, for example Cursor's
IDE store versus its CLI transcripts.

### `botspy doctor`

Per-source diagnostics: the root path, the discovered counts, and the
adapter's issues list. Doctor reports; it does not gate — the exit
code stays 0 unless a source is unreadable. Skipped files and missing
stores are data about your machine, not failures.

```console
$ botspy doctor
claude_code   ok    3 project dirs, 4 transcripts, 0 issues
cursor        ok    2 composers, 4 bubbles, 0 issues
codex         ok    2 threads, 0 issues
grok_bot      ok    2 entry logs, 1 cloud agents, 0 issues
antigravity   ok    2 conversations, 2 transcripts, 0 issues
```

Flags: `--source`. Planned flag (BOTSPY-98386a): `--parity` — the
golden round-trip check per agent, and digest verification that no
source file was mutated.

### `botspy snapshot <source>`

Take a WAL-safe snapshot of a SQLite source through a read-only
connection, prove the source was not mutated (digests of the database
and its `-wal`/`-shm` sidecars, before and after), and report row
counts. The positional argument is a source name or a path. Snapshots
land under `<home>/.botspy/snapshots/<source>/snapshot.db` by default;
`--out <dir>` overrides.

```console
$ botspy snapshot cursor
snapshot: ~/.botspy/snapshots/cursor/snapshot.db
events: 115,200   source unmutated (digest verified before/after)
```

Flags: `--source`, `--out <dir>`. Planned flag (BOTSPY-98386a):
`--list` — show what is cached.

## Planned verbs: `search`, `import`, `stats`, `watch`

The four verbs below are not implemented today. Open Kanbus task
BOTSPY-98386a tracks them. They will use the local store at
`<home>/.botspy/store.db`.

- `search <query>` — search the local store over messages and parts.
  `--mode text` gives FTS5 full-text search: quoted terms,
  punctuation and operators dropped, implicit AND, bm25 ranking.
  `--mode semantic` embeds the query text and matches by cosine
  similarity (floor 0.3). The store also fuses both result sets into
  one hybrid ranking (reciprocal-rank fusion plus best cosine
  similarity). See [`search.md`](search.md). Planned flags:
  `--mode text|semantic`, `--source`, `--project`, `--kind`, `--role`,
  `--context N` (surrounding messages, default 0), `--since`/`--until`,
  `--limit`, plus the output globals.
- `import` — run the pipeline: discover, extract, and write into the
  local store. Incremental by default: discovery is incremental by
  identity, and extraction resumes at stored byte offsets per session.
  `--full` re-extracts from byte zero. `--dry-run` reports what would
  land plus the skip counters, and writes nothing. Cursor's live store
  is read through a WAL-safe snapshot automatically.
- `stats` — aggregate counts, usage, and cost over the history:
  sessions, messages, turns, parts by kind, tokens, cost where agents
  recorded it, sessions per source and per project, compaction counts.
  Absent usage shows as `—`, never as 0. Planned flags: `--source`,
  `--project`, `--since`/`--until`, `--by <day|source|project>`.
- `watch` — follow the agents as they work: re-discovery on an
  interval (identity-incremental, so this is cheap), re-extraction at
  stored offsets for appended messages. NDJSON emits typed events:
  `session_new`, `session_changed`, `message_new`. SIGINT will exit
  with code 130. Planned flags: `--source`, `--interval <secs>`
  (default 2), plus the output globals.

## Composition

No REPL, no TUI. The fun is piping:

```bash
botspy sessions -o ndjson | jq -r .project_id | sort | uniq -c
botspy show a3f9 -o ndjson | jq -c 'select(.role=="assistant")'
botspy sources -o json | jq -r '.[] | select(.status != "ok") | .name'
botspy doctor -o ndjson | jq -c 'select(.issues | length > 0)'
```