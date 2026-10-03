# Source adapters

This document uses Simplified Technical English. It gives the rules
that all five adapters follow. One document per source goes deeper:
[claude-code.md](claude-code.md), [cursor.md](cursor.md),
[codex.md](codex.md), [grok-bot.md](grok-bot.md),
[antigravity.md](antigravity.md).

## The registry

1. The registry makes a source by name: `create_source(name, options)`.
2. The names are the five registry names: `claude_code`, `cursor`,
   `codex`, `grok_bot`, `antigravity`. Kebab-case names are accepted.
3. The options give the root directory and the home directory.

The roots:

| Source | Root |
| --- | --- |
| `claude_code` | `~/.claude/projects` |
| `cursor` | `~/.cursor` (the IDE store is `~/.cursor/state.vscdb`; the CLI transcripts are under `~/.cursor/projects`) |
| `codex` | `~/.codex` |
| `grok_bot` | `~/.grok/sand-client-persistence` |
| `antigravity` | `~/.gemini/antigravity` |

The home directory comes from `--home`, or from `BOTSPY_HOME`, or from
`$HOME`. The `--root` flag replaces the root for one source.

## The adapter contract

Each adapter does two things:

1. **Discovery.** The adapter lists the sessions it can see. It gives
   a summary per session: id, title, started time, last activity,
   message count. Discovery uses a throwaway scanner; it does not
   consume the extraction state.
2. **Extraction.** `open(id)` gives one full session. The adapter reads
   the session from its files and converts it to the unified schema.

Discovery is incremental by identity: a second run reports only the
new and the changed sessions. Extraction resumes at a byte offset per
session where the source is a JSONL file. A partial trailing line
(an agent still writing) is held back until it is complete.

## The unified schema

Each adapter emits the same shapes:

- `Session` — id, agent, project, times, title, the session graph
  (parent, root, subagent, fork, battle, peers), compaction windows,
  partial-history notes, usage, cost, and metadata.
- `Message` — role (user, assistant, system, tool), timestamp, turn.
- `Part` — one of: text, thinking, tool_call, tool_result, attachment,
  inline_data, blob, system. Unknown payload shapes pass through
  verbatim (`Part::Extra`).

Each part keeps provenance: the source file, the line or row, the
record id and type, and the ordinal.

Two rules hold everywhere:

- **Timestamps are never made up.** A missing time stays missing.
  Absent usage shows as none, not as zero.
- **The order is authoritative, not the file order.** Each source has
  one field that defines the order. The adapter sorts by that field.

## Skips

Skips are data, not errors. A malformed line, an unknown record type,
or a missing file is skipped and counted. The counts surface in
`doctor`, in `sources -v`, in `import`, and in the reports. Skips
never change the exit code.

## Snapshots

Some sources keep their data in SQLite databases that the agent still
writes to. The adapters never open such a database for writing. They
copy it first:

1. The snapshot module opens the source database read-only.
2. It makes a consistent copy with the SQLite online backup API.
3. The copy lands in `$TMPDIR` (`botspy-cursor-snaps/<pid>-<seq>/`,
   `botspy-ag-snaps/<pid>-<seq>/`) or, for the `snapshot` verb, in
   `<home>/.botspy/snapshots/<source>/`.
4. `digest_files` hashes the source database and its `-wal` and `-shm`
   sidecars before and after. The digests match: the source was not
   mutated.

The `snapshot` verb wraps the same module so you can make and inspect
a copy by hand.

## Reports and diagnostics

Each adapter gives a report type with its root, its counts, its skip
counters, and its issues. `botspy doctor` prints one report per source.
Doctor reports; it does not gate.

## Sensitive data

Every adapter reads session content: prompts, code, tool output,
thinking traces. The content is sensitive. Every per-source document
below lists the exact paths, and
[`../sensitive-data.md`](../sensitive-data.md) holds the full
inventory.