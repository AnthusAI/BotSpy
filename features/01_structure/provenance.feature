Feature: Provenance
  Every message records where it came from on disk: the source file plus a
  line number for line-oriented sources, or a row number for SQLite-backed
  sources. The native record identity — the agent's own record id, record
  type, ordinal/sort key, and parent pointer — is preserved too, so the
  raw record behind any normalized message can always be found and
  tree-structured histories (resumes, edits, compaction) keep their shape.

  Scenario: A message records its source file and line
    Given a fixture session "p1" from agent "claude_code" in project "demo"
    When I record a message with role "user" at "2026-10-01T09:01:00Z" with text "please fix the bug" and provenance "transcript.jsonl" line 42
    Then the last message provenance is file "transcript.jsonl" line 42

  Scenario: A SQLite-backed message records its source file and row
    Given a fixture session "p2" from agent "cursor" in project "demo"
    When I record a message with role "user" at "2026-10-01T09:01:00Z" with text "hello" and provenance "state.vscdb" row 17
    Then the last message provenance is file "state.vscdb" row 17

  Scenario: A message records its native record id and type
    Given a fixture session "p3" from agent "claude_code" in project "demo"
    When I record a message with role "assistant" at "2026-10-01T09:02:00Z" and provenance record id "bfc63cd8-uuid" of type "assistant"
    Then the last message provenance has record id "bfc63cd8-uuid"
    And the last message provenance has record type "assistant"

  Scenario: A message records its ordinal sort key
    Given a fixture session "p4" from agent "codex" in project "demo"
    When I record a message with role "assistant" at "2026-10-01T09:03:00Z" and provenance "rollout.jsonl" line 512 with ordinal 511
    Then the last message provenance has ordinal 511
    And the messages are ordered by their ordinal, not their file position

  Scenario: A message records its native parent pointer
    Given a fixture session "p5" from agent "claude_code" in project "demo"
    When I record a message with role "assistant" at "2026-10-01T09:04:00Z" and provenance record id "child-uuid" with parent record "parent-uuid"
    Then the last message provenance has parent record "parent-uuid"