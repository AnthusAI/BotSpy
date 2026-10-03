# The BotSpy CLI

The `botspy` command is a thin shell over the library. Every verb is
one library call plus rendering. The CLI holds no state of its own: it
reads agent files and the local store, and it writes only BotSpy's own
store and snapshot files.

The store-backed verbs (`search`, `import`, `stats`, `watch`) use the
local store at `<home>/.botspy/store.db`. `sources`, `sessions`,
`show`, `doctor`, and `snapshot` read the agent files directly.

## Global flags

Every verb accepts these flags:

| Flag | Effect |
| --- | --- |
| `-s, --source <name>` | Select one source. Repeat the flag to select several. Names are the registry's own: `claude_code`, `cursor`, `codex`, `grok_bot`, `antigravity` (kebab-case aliases accepted). |
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
| 130 | Interrupted — SIGINT during `watch`. |

Skipped records are data, not errors. Every verb that extracts prints
its skip counts, and skips never change the exit code.

## The verbs

```
botspy
  sessions   list sessions across every source, newest activity first
  show       open one session; walk its messages, turns, and parts
  search     find messages and parts by text, semantically, or both
  import     run the pipeline into the local store (dry-run supported)
  sources    inspect the source registry: roots, session counts, skips
  doctor     per-source diagnostics
  snapshot   take a WAL-safe snapshot of a SQLite source and inspect it
  stats      aggregate counts, usage, and cost over the history
  watch      follow new sessions and messages as agents write them
```

### `botspy sessions`

One unified listing across every registered adapter, most recent
activity first. `-o json` is exactly the serialized `[SessionSummary]`.

Flags: `--source`, `--project <id>`, `--since`/`--until` (RFC 3339
timestamps), `--limit N`, `--tree` (render the session graph:
parents, subagents, forks, battles), `--kind <part-kind>` (sessions
containing that part kind).

```console
$ botspy sessions --project myapp --since 2026-09-28
SESSION   AGENT        PROJECT  MESSAGES  LAST ACTIVITY      TITLE
a3f9c1e2  claude_code  myapp         128  2026-10-02 15:41  fix WAL deadlock
b77d31aa  cursor       myapp          62  2026-10-01 18:04  composer: importers
3 sessions (use --no-truncate for full ids)
```

### `botspy show <session-id>`

Open one session and walk it: a header line, then messages with their
parts. A session is addressed by its native id, or by any unambiguous
id prefix. An ambiguous prefix is an error that lists the candidates;
an unknown id exits 1.

Flags: `--message N` (1-based message ordinal), `--turn <id>`,
`--kind <part-kind>`, `--role`, `--thinking` (expand thinking
traces), `--usage` (per-turn and thread usage), `--raw` (the `extra`
passthrough untouched), `--graph` (parent/subagent/fork/battle/peer
links and compaction windows).

```console
$ botspy show a3f9 --message 14
Session a3f9c1e2 (claude_code, myapp) — 128 messages, started 2026-09-30T09:12Z
#14 assistant 2026-09-30T09:31Z turn t-7
  · text       Let me check the WAL handling…
  · thinking   (190 lines)
  · tool_call  Read path="src/snapshot.rs" [ok]
  · tool_result  412 B
```

### `botspy search <query>`

Search the local store over messages and parts.

- `--mode text` — FTS5 full-text search: quoted terms, punctuation and
  operators dropped, implicit AND, bm25 ranking.
- `--mode semantic` — vector search: the query text is embedded and
  matched by cosine similarity (floor 0.3).
- The store also fuses both result sets into one hybrid ranking
  (reciprocal-rank fusion plus best cosine similarity). See
  [`search.md`](search.md).

NDJSON hits carry `session_id`, `message_ordinal`, `role`, `part_kind`,
`text`, and `provenance`.

Flags: `--mode text|semantic`, `--source`, `--project`, `--kind`,
`--role`, `--context N` (surrounding messages, default 0),
`--since`/`--until`, `--limit`, plus output globals.

```console
$ botspy search "WAL deadlock" --source cursor --context 1
b77d31aa (cursor) #42 assistant 2026-10-01T18:04Z
  …the WAL deadlock only appears when the writer holds the…
  ├ #41 assistant  …snapshot copy taken over a read-only connection…
  └ #43 tool_result  ok
1 hit (12 sessions scanned)
```

### `botspy import`

Run the pipeline: discover, extract, and write into the local store.

- Incremental by default — discovery is incremental by identity, and
  extraction resumes at stored byte offsets per session.
- `--full` re-extracts from byte zero.
- `--dry-run` reports what would land plus the skip counters, and
  writes nothing.
- Every run prints the skip summary. Cursor's live store is read
  through a WAL-safe snapshot automatically.

Flags: `--dry-run`, `--full`, `--source`, plus output globals.

```console
$ botspy import --dry-run
SOURCE       NEW  CHANGED  SKIPPED  NOTES
claude_code    3        1       12  2 malformed, 10 unknown
cursor         0        2        0  live WAL: read via snapshot
codex          1        0        4  4 partial (trailing lines)
dry run: nothing written
```

### `botspy sources`

The source registry as a human view: every known source name, its
resolved root (after `--root`/`--home` overrides), the adapter behind
it, the session count from the last discovery, and skip stats. `-v`
adds per-source discovery detail (for example Cursor's IDE store
versus its CLI transcripts).

Flags: `--source` (detail one), `-v, --verbose`.

### `botspy doctor`

Per-source diagnostics: root/store path, discovered counts, skip
counters, and the adapter's issues list. `--parity` adds the golden
round-trip check per agent and verifies with digests that no source
file was mutated. Doctor reports; it does not gate — the exit code
stays 0 unless a source is unreadable.

Flags: `--source`, `--parity`.

### `botspy snapshot <source>`

Take a WAL-safe snapshot of a SQLite source through a read-only
connection, prove the source was not mutated (digests of the database
and its `-wal`/`-shm` sidecars, before and after), and report row
counts. The positional argument is a source name or a path. Snapshots
land under `<home>/.botspy/snapshots/<source>/snapshot.db` by default;
`--out <dir>` overrides. `--list` shows what is cached.

```console
$ botspy snapshot cursor
snapshot: ~/.botspy/snapshots/cursor/snapshot.db
events: 121,004   source unmutated (digest verified before/after)
```

### `botspy stats`

Aggregates over the unified history: sessions, messages, turns, parts
by kind, tokens in and out, cost where agents recorded it, sessions
per source and per project, compaction counts, partial sessions.
Absent usage shows as `—`, never as 0.

Flags: `--source`, `--project`, `--since`/`--until`,
`--by <day|source|project>`.

```console
$ botspy stats --since 2026-09-01
sessions 373   messages 41,208   turns 9,877
tokens in 3.1M   tokens out 0.9M   cost $18.42 (where recorded)
by source: claude_code 214 · cursor 96 · codex 63
top projects: myapp 122 · dataparade-cli 87 · BotSpy 74
```

### `botspy watch`

Follow the agents as they work: re-discovery on an interval
(identity-incremental, so this is cheap), re-extraction at stored
offsets for appended messages. NDJSON emits typed events:
`session_new`, `session_changed`, `message_new` — a building block for
`grep` and `jq`. SIGINT exits with code 130.

Flags: `--source`, `--interval <secs>` (default 2), plus output
globals.

```console
$ botspy watch --source claude_code
15:41:07 session_new    f02ab…  (myapp)
15:42:12 message_new    a3f9c1e2 #129 user "now run the parity check"
15:43:01 message_new    a3f9c1e2 #130 assistant (text, tool_call, tool_result)
^C
```

## Composition

No REPL, no TUI. The fun is piping:

```bash
botspy sessions --source claude_code -o ndjson | jq -r .project_id | sort | uniq -c
botspy show a3f9 -o ndjson | jq -c 'select(.role=="assistant")'
botspy search "WAL deadlock" -o ndjson | head -20
botspy watch --source cursor -o ndjson | grep session_new
```