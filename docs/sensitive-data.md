# Sensitive data

Your coding-agent sessions hold sensitive content: prompts, source
code, file contents, tool output, thinking traces, and sometimes
credentials and secrets that you or a tool put into a prompt.

Sensitive-data visibility is a central purpose of BotSpy, not a
side note. BotSpy reads this content, copies it, and indexes it so you
can search it. That only helps you if you can also see exactly where
the copies live. This page is that map.

## The marker

Every diagram in this documentation marks a sensitive place with the
same marker: an amber border, 3 px wide, with a lock icon 🔒. The
marker means: **this place reads, copies, or stores session content.**
The legend on every diagram states the same rule, and the shared theme
file `diagrams/_shared.d2` defines the marker once for all diagrams.

Places that only move or transform data — adapters, the CLI process,
the model cache — keep the theme's default look.

## The inventory

Every path BotSpy reads, copies, or writes. `<home>` is the
`--home` flag, else `BOTSPY_HOME`, else `$HOME`.

### 1. Read (the agents' own files — never written by BotSpy)

| Source | Exact paths | Content |
| --- | --- | --- |
| Claude Code | `~/.claude/projects/<encoded-cwd>/<session>.jsonl`; `~/.claude/projects/<encoded-cwd>/subagents/agent-*.jsonl` (skipped, counted) | Prompts, replies, thinking, tool calls and results, costs, titles |
| Cursor | `~/.cursor/state.vscdb` (plus `-wal`, `-shm`); `~/.cursor/projects/<encoded-cwd>/agent-transcripts/*.jsonl` | Composer conversations, bubbles, tool blobs |
| Codex | `~/.codex/state_5.sqlite`; `~/.codex/sessions/YYYY/MM/DD/rollout-*.jsonl` | Threads, rollouts, messages, reasoning, tool calls, token usage |
| Grok Bot | `~/.grok/sand-client-persistence/*.json` — entry logs, plus the reserved `roster.json` and `cloud-agents.json` | Messages, voice calls (raw passthrough), agent names, agent status |
| Antigravity | `~/.gemini/antigravity/conversation_summaries.db`; `~/.gemini/antigravity/brain/<id>/payload.db`; `~/.gemini/antigravity/brain/<id>/.system_generated/logs/transcript.jsonl` | Conversations, planner responses, checkpoints, tool calls and results |

### 2. Copied (BotSpy-created snapshot copies — temporary or cached)

| Copy | Exact path | When |
| --- | --- | --- |
| Cursor snapshot | `$TMPDIR/botspy-cursor-snaps/<pid>-<seq>/snapshot.db` | Automatic: the live Cursor store is read through this copy |
| Antigravity snapshots | `$TMPDIR/botspy-ag-snaps/<pid>-<seq>/snapshot.db` (one per database) | Automatic: the live Antigravity databases are read through these copies |
| Snapshot cache | `<home>/.botspy/snapshots/<source>/snapshot.db` | Written by the `snapshot` verb, on demand |

The snapshot module proves with file digests (`digest_files`, before
and after) that the source files were not mutated. The copies hold the
same content as the sources.

### 3. Stored (BotSpy's own store — the long-term copy)

| Place | Exact path | Content |
| --- | --- | --- |
| Store database | `<home>/.botspy/store.db` | Full session JSON (`sessions`, `messages`), plaintext part text (`parts`) |
| WAL sidecars | `<home>/.botspy/store.db-wal`, `<home>/.botspy/store.db-shm` | Recent page writes — same content |
| FTS5 index | the `message_fts` virtual table inside `store.db` | Plaintext index over `parts.text` |
| Vector index | the `message_vec_rows` + `message_vec` tables inside `store.db` | Embeddings — derived data, not plaintext |

Provenance on every stored part carries the absolute source path, so
the store always knows where each piece of content came from.

### 4. Output (leaves BotSpy through your terminal)

| Place | Content |
| --- | --- |
| stdout / JSON / NDJSON of `sessions`, `show`, `search`, `stats` | Session summaries, full messages, part text, search hits — whatever you asked to see |

Search query text also transits the local embedder at search time
(semantic and hybrid modes). The embedder runs on your CPU and sends
nothing over the network.

### 5. Not sensitive

| Place | Exact path | Why |
| --- | --- | --- |
| Model cache | `<home>/.botspy/models/all-MiniLM-L6-v2/` (`model_quantized.onnx`, `tokenizer.json`) | Model files only. No session data. Downloaded once, digest-verified. |

## Properties that hold everywhere

- BotSpy never writes to a source file. Every database connection is
  read-only; the snapshot module uses the SQLite online backup API
  over a read-only connection.
- Today only Cursor and Antigravity read their live databases through
  snapshot copies. Codex opens `~/.codex/state_5.sqlite` read-only,
  directly: no snapshot copy. A read-only connection still touches the
  `-shm` sidecar of a database that another process keeps open. The
  text sources (Claude Code, Grok Bot, the Codex rollouts) read plain
  files.
- The only writes are BotSpy's own: the store, the snapshot copies,
  and the model cache.
- No network at runtime. The one-time model download is the only
  network step, and it is digest-verified.

## Removing the data

- To remove all BotSpy copies: delete `<home>/.botspy/` (the store, the
  snapshot cache, and the model cache).
- The temporary snapshot copies in `$TMPDIR` go away with the temp
  directory.
- The agents' own files stay; BotSpy cannot and does not change them.