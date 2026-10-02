Feature: botspy sessions
  One unified listing across every registered source. Filters select by
  source, project, and activity window; --limit caps the listing. The
  output modes are human (truncated, pipe-tolerant), json, and ndjson —
  machine modes never truncate.

  Background:
    Given a fixture home built from the synthetic corpus

  Scenario: Sessions from every source are listed together
    When I run "botspy sessions" against the fixture home
    Then the exit code is 0
    And stdout reports "13 sessions"

  Scenario: JSON output is the session summaries serialized
    When I run "botspy sessions -o json" against the fixture home
    Then the exit code is 0
    And stdout parses as a JSON array of 13 entries
    And every JSON entry has "id", "agent" and "project_id"

  Scenario: NDJSON streams one summary per line
    When I run "botspy sessions -o ndjson" against the fixture home
    Then the exit code is 0
    And stdout parses as NDJSON with 13 lines

  Scenario: --source filters to one source
    When I run "botspy sessions --source claude_code" against the fixture home
    Then the exit code is 0
    And stdout reports "4 sessions"

  Scenario: --source can be repeated
    When I run "botspy sessions --source claude_code --source codex" against the fixture home
    Then the exit code is 0
    And stdout reports "6 sessions"

  Scenario: --project filters by project id
    When I run "botspy sessions --project extra-demo" against the fixture home
    Then the exit code is 0
    And stdout reports "1 session"
    And stdout contains "solo-abc123"

  Scenario: --limit caps the listing
    When I run "botspy sessions --limit 5" against the fixture home
    Then the exit code is 0
    And stdout reports "5 sessions"

  Scenario: --since excludes sessions with no activity that recent
    When I run "botspy sessions --since 2030-01-01T00:00:00Z" against the fixture home
    Then the exit code is 0
    And stdout reports "0 sessions"

  Scenario: Human output truncates session ids
    When I run "botspy sessions" against the fixture home
    Then the exit code is 0
    And stdout does not contain "00000000-0000-4000-8000-000000000001"
    And stdout contains "00000000"

  Scenario: An unknown source filter is rejected
    When I run "botspy sessions --source notepad"
    Then the exit code is 1
    And stderr contains "notepad"