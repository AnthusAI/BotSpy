@wip
Feature: Timestamps are optional and never fabricated
  Some agents write no timestamps at all. A message without a native time
  has no timestamp, and ordering never depends on it.
  Evidence: Cursor CLI transcripts carry no timestamps — only a
  <timestamp> snippet injected into the user prompt by the harness and
  the transcript file's own metadata; several agents write some records
  without timestamps (Cursor queue/mode rows, Grok persistedAt only).

  Scenario: A transcript without timestamps has messages without timestamps
    Given a fixture session "z1" from agent "cursor" in project "demo"
    When I record 3 messages in file order without timestamps
    Then all 3 messages have no timestamp

  Scenario: Order still follows the source order
    Given a fixture session "z2" from agent "cursor" in project "demo"
    When I record 3 messages in file order without timestamps
    Then the messages are ordered by their source order, not by time

  Scenario: File metadata is not fabricated into timestamps
    Given a fixture session "z3" from agent "cursor" in project "demo"
    When the transcript file was modified at "2026-10-01T10:00:00Z"
    Then no message has timestamp "2026-10-01T10:00:00Z"
    And the file modification time is recorded outside message timestamps