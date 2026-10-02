Feature: Unified session history
  BotSpy piles sessions from every coding agent into one unified history.
  Listing sessions shows sessions across agents sorted by last activity,
  most recent first.

  Scenario: Sessions from two agents are listed sorted by last activity
    Given the fixture adapter for agent "claude_code" with sessions:
      | id | project | started_at           | last_activity_at     |
      | c1 | demo    | 2026-10-01T09:00:00Z | 2026-10-01T10:00:00Z |
      | c2 | demo    | 2026-10-01T08:00:00Z | 2026-10-01T11:00:00Z |
    And the fixture adapter for agent "codex" with sessions:
      | id | project | started_at           | last_activity_at     |
      | x1 | infra   | 2026-10-01T07:00:00Z | 2026-10-01T09:30:00Z |
    When I list sessions
    Then the sessions are listed in order "c2, c1, x1"

  Scenario: Each listed session carries its agent and project
    Given the fixture adapter for agent "claude_code" with sessions:
      | id | project | started_at           | last_activity_at     |
      | c1 | demo    | 2026-10-01T09:00:00Z | 2026-10-01T10:00:00Z |
    And the fixture adapter for agent "grok_bot" with sessions:
      | id | project | started_at           | last_activity_at     |
      | g1 | ops     | 2026-10-01T06:00:00Z | 2026-10-01T06:30:00Z |
    When I list sessions
    Then session "c1" is listed with agent "claude_code" and project "demo"
    And session "g1" is listed with agent "grok_bot" and project "ops"