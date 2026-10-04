Feature: Codex importer
  The Codex importer watches rollout JSONL event logs under
  `~/.codex/sessions/YYYY/MM/DD/`, indexed by `state_5.sqlite`. The
  `threads` table carries the thread metadata (id, rollout_path, cwd,
  title, model, git branch/origin/sha, created/updated instants) and has
  no parent column: spawn edges live in the `thread_spawn_edges` table
  (parent_thread_id, child_thread_id, status). The index is a live
  database, so it is read through a safe snapshot copy, never in place.
  `rollout_path` may be absolute or relative to the home root; both are
  resolved explicitly. `thread_history_1.sqlite` stores turn and item
  projections with rollout byte offsets. Rollouts are appended live and
  paginated; partial trailing lines are retried, malformed records
  skipped and counted.
  Evidence: every rollout record carries an explicit `ordinal` — the
  authoritative order; `response_item`, `token_usage_record`, and
  `compacted` records; `task_complete` carries duration_ms and
  time_to_first_token_ms.

  Scenario: Rollouts are discovered via the threads index
    Given a Codex threads index with 3 threads pointing at rollout files
    When the importer detects changes
    Then discovery yields 3 sessions

  Scenario: Spawn edges link sub-agent threads to their parents
    Given a Codex threads index where thread "t2" is spawned by thread "t1"
    When the importer detects changes
    Then session "t2" links to parent session "t1"

  Scenario: Thread metadata lands in the discovery summary
    Given a Codex threads index where thread "t1" carries title "Alpha build fix", model "synthetic-model", git branch "feat/alpha", git origin "https://git.example.com/acme/alpha.git", and cwd "/workspace/alpha"
    When the importer detects changes
    Then session "t1" carries the title, model, git branch, git origin, and cwd

  Scenario: Thread instants land in the discovery summary
    Given a Codex threads index where thread "t1" was created at 1767225600000 and last updated at 1767229200000
    When the importer detects changes
    Then session "t1" starts at "2026-01-01T00:00:00Z" and was last active at "2026-01-01T01:00:00Z"

  Scenario: An absolute rollout path resolves to itself
    Given a Codex threads index where thread "t1" points at an absolute rollout path
    When the importer detects changes
    Then discovery yields 1 session

  Scenario: The threads index is read through a snapshot copy
    Given a Codex threads index with 2 threads pointing at rollout files
    When the importer reads the threads index through a snapshot copy
    Then the threads index was never mutated

  Scenario: Schema drift in the threads index is reported, not swallowed
    Given a Codex threads index with a legacy schema carrying a parent column
    When the importer detects changes
    Then discovery yields 0 sessions
    And discovery reports an issue about the threads index

  Scenario: A missing spawn edge table is reported, not swallowed
    Given a Codex threads index whose spawn edge table is missing
    When the importer detects changes
    Then discovery still yields the indexed sessions
    And discovery reports an issue about the spawn edges

  Scenario: Extraction maps thread metadata into the session
    Given a Codex threads index where thread "t1" carries title "Alpha build fix", model "synthetic-model", git branch "feat/alpha", git origin "https://git.example.com/acme/alpha.git", and cwd "/workspace/alpha"
    When the importer extracts the rollout
    Then the session carries the title, model, git branch, git origin, and cwd as project

  Scenario: Record order follows the explicit ordinal
    Given a Codex rollout with records "ordinal 3", "ordinal 1", and "ordinal 2"
    When the importer extracts the rollout
    Then the normalized messages appear in ordinal order 1, 2, 3

  Scenario: Extraction resumes from the stored byte offset
    Given a Codex rollout with 4 records and a stored offset after record 2
    When the importer extracts the rollout from the stored offset
    Then extraction yields 2 more good records

  Scenario: Response items become messages with roles, reasoning, and tool calls
    Given a Codex rollout holding "response_item" records with roles "user", "assistant", "developer", a reasoning summary, and a tool call with output
    When the importer extracts the rollout
    Then the normalized messages map the roles, reasoning kinds, and tool call/result kinds

  Scenario: Usage and compaction records map to their chapter-1 types
    Given a Codex rollout holding a "token_usage_record" and a "compacted" record
    When the importer extracts the rollout
    Then the session usage carries the token counts
    And the compaction window carries the previous window and the retained context

  Scenario: Task completion maps to turn timing
    Given a Codex rollout holding a "task_complete" record with duration_ms 14300 and time_to_first_token_ms 850
    When the importer extracts the rollout
    Then the turn has duration_ms 14300 and time_to_first_token_ms 850

  Scenario: Partial trailing lines are retried, malformed records skipped
    Given a Codex rollout holding a malformed line and ending in a partial trailing line
    When the importer extracts the rollout
    Then extraction reports 2 skipped records
    And no partial or malformed record is normalized

  Scenario: Doctor reports schema drift as an issue
    Given a Codex threads index with a legacy schema carrying a parent column
    When the doctor checks the Codex source
    Then the doctor reports the schema drift issue