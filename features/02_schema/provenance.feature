Feature: Provenance
  Every message records where it came from on disk: the source file plus a
  line number for line-oriented sources, or a row number for SQLite-backed
  sources.

  Scenario: A message records its source file and line
    Given a fixture session "p1" from agent "claude_code" in project "demo"
    When I record a message with role "user" at "2026-10-01T09:01:00Z" with text "please fix the bug" and provenance "transcript.jsonl" line 42
    Then the last message provenance is file "transcript.jsonl" line 42

  Scenario: A SQLite-backed message records its source file and row
    Given a fixture session "p2" from agent "cursor" in project "demo"
    When I record a message with role "user" at "2026-10-01T09:01:00Z" with text "hello" and provenance "state.vscdb" row 17
    Then the last message provenance is file "state.vscdb" row 17