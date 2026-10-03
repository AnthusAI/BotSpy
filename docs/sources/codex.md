# Codex source adapter

This document uses Simplified Technical English.

The Codex adapter reads the thread index and the rollout files that
Codex keeps under `~/.codex`.

## Where the data lives

| Path | Content |
| --- | --- |
| `~/.codex/state_5.sqlite` | The threads index. The table `threads` gives: `id`, `rollout_path`, `parent_id`, `cwd`. The database opens read-only. |
| `~/.codex/thread_history_1.sqlite` | The history index. It gives turn and item projections and rollout byte offsets. |
| `~/.codex/sessions/YYYY/MM/DD/rollout-*.jsonl` | The rollouts. One file holds one session. |

## Discovery

1. The adapter reads the `threads` table.
2. Each thread gives its rollout path.
3. A thread with a missing rollout file is skipped and counted.
4. `parent_id` links become `parent_links` in the session graph.
5. `CodexDiscovery` gives: the thread count, the missing rollouts, and
   the parent links.

## Extraction and ordering

1. `CodexSource` reads the rollout record by record.
2. Every record carries an explicit `ordinal` field. The ordinal, not
   the file order, is the authoritative order. The adapter sorts by
   ordinal.
3. The adapter keeps a byte offset per session. The next run resumes
   at the offset.
4. A partial trailing line is held back until it is complete.

## Record mapping

| Record type | Unified result |
| --- | --- |
| `response_item`, payload message user / assistant | A `Text` part |
| `response_item`, payload message developer | A `System` part with origin developer |
| `response_item`, reasoning summary | A `Thinking` part |
| `function_call` | A `ToolCall` part (call id, name, parsed arguments) |
| `function_call_output` | A `ToolResult` part on a tool-role message |
| `token_usage_record` | Session `Usage`: input, output, cached input, reasoning tokens, and the model |
| `compacted` | A `CompactionWindow`: window id, previous window, retained context |
| `task_complete` | A `Turn`: turn id, duration, time to first token |

## Report

`CodexReport` gives: the root, the thread count, the missing rollouts,
and the issues.

## The diagram

<picture>
  <source media="(prefers-color-scheme: dark)" srcset="../diagrams/codex.dark.png">
  <img src="../diagrams/codex.light.png" alt="Codex adapter: threads index, history index, rollouts, record mapping">
</picture>

## Sensitive data

The adapter reads session content: prompts, replies, reasoning
summaries, tool calls and output, and token usage. Exact paths:

- `~/.codex/state_5.sqlite` — read
- `~/.codex/thread_history_1.sqlite` — read
- `~/.codex/sessions/YYYY/MM/DD/rollout-*.jsonl` — read

See [`../sensitive-data.md`](../sensitive-data.md).