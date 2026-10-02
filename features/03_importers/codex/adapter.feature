Feature: Codex importer
  The Codex importer watches rollout JSONL event logs under
  `~/.codex/sessions/YYYY/MM/DD/`, indexed by `state_5.sqlite` (threads,
  projects, spawn edges) and `thread_history_1.sqlite` (turn and item
  projections with rollout byte offsets), and emits normalized records as
  they appear. Rollouts are appended live and paginated; partial trailing
  lines are retried, malformed records skipped and counted.
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