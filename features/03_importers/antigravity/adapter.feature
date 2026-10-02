Feature: Antigravity importer
  The Antigravity importer watches `conversation_summaries.db` for new
  and updated conversations, prefers each conversation's readable
  `brain/<id>/.system_generated/logs/transcript.jsonl`, and falls back to
  the per-conversation SQLite DB (protobuf payload blobs) for linkage the
  JSONL lacks. Per-conversation DBs are hot WAL databases: they are read
  through a snapshot copy, never in place.
  Evidence: transcript.jsonl records carry `USER_INPUT`,
  `PLANNER_RESPONSE`, `CHECKPOINT`, `ERROR_MESSAGE`, `step_index`,
  `created_at`.

  Scenario: Conversations are discovered from the summaries index
    Given an Antigravity summaries index with 3 conversations
    When the Antigravity importer detects changes
    Then discovery yields 3 sessions

  Scenario: The transcript is the preferred source
    Given an Antigravity conversation with both a "transcript.jsonl" and a SQLite payload DB
    When the Antigravity importer extracts the conversation
    Then the messages come from the transcript records

  Scenario: Transcript types map to chapter-1 kinds
    Given an Antigravity transcript holding "USER_INPUT", "PLANNER_RESPONSE", "CHECKPOINT", and "ERROR_MESSAGE" records
    When the Antigravity importer extracts the transcript
    Then "USER_INPUT" and "PLANNER_RESPONSE" become user and assistant messages
    And "CHECKPOINT" becomes a compaction summary
    And "ERROR_MESSAGE" becomes an error message

  Scenario: Messages appear in step_index order
    Given an Antigravity transcript with steps "step_index 3", "step_index 1", and "step_index 2"
    When the Antigravity importer extracts the transcript
    Then the normalized messages appear in step order 1, 2, 3

  Scenario: The SQLite DB supplies linkage the transcript lacks
    Given an Antigravity conversation whose transcript lacks tool-call ids
    And whose payload DB carries the tool call and its result
    When the Antigravity importer extracts the conversation
    Then the tool call and result are linked by the DB-supplied call id

  Scenario: Hot per-conversation DBs are read through a snapshot copy
    Given an Antigravity conversation DB in WAL mode with a live app writing
    When the importer reads the DB through a snapshot copy
    Then the source DB and its WAL were never mutated
    And the live app kept writing rows without interference

  Scenario: Malformed steps are skipped and counted
    Given an Antigravity transcript holding 3 good records and 2 malformed ones
    When the Antigravity importer extracts the transcript
    Then extraction maps 3 good records
    And extraction reports 2 skipped records