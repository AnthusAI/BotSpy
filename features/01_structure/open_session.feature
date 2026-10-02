Feature: Opening and walking a session
  A session from any agent opens by id, and its messages walk in order.
  An id no adapter knows is an error, never a silent empty result.

  Scenario: Opening a session and walking its messages in order
    Given the fixture adapter for agent "claude_code" with sessions:
      | id | project | started_at           | last_activity_at     |
      | c1 | demo    | 2026-10-01T09:00:00Z | 2026-10-01T10:00:00Z |
    And the session "c1" has messages:
      | role      | timestamp            | text                    |
      | user      | 2026-10-01T09:01:00Z | please fix the bug      |
      | assistant | 2026-10-01T09:02:00Z | the bug is in parser.rs |
      | assistant | 2026-10-01T09:03:00Z | fixed, tests pass       |
    When I open session "c1"
    And I walk the messages of session "c1"
    Then opening session "c1" succeeds with agent "claude_code", project "demo" and 3 messages
    And walking its messages yields roles "user, assistant, assistant" in order
    And message 1 has a text part "please fix the bug"

  Scenario Outline: Opening an unknown session id reports an error
    Given the fixture adapter for agent "claude_code" with sessions:
      | id | project | started_at           | last_activity_at     |
      | c1 | demo    | 2026-10-01T09:00:00Z | 2026-10-01T10:00:00Z |
    When I open session "<id>"
    Then an unknown-session error is reported for "<id>"

    Examples:
      | id         |
      | nope       |
      | missing-42 |