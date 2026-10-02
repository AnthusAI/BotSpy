Feature: botspy show
  Open one session from the unified history and walk its messages. A
  session is addressed by the agent's own stable id, or by any
  unambiguous prefix of it; an ambiguous prefix is an error listing the
  candidates, never a guess. An unknown id is an error, never an empty
  result.

  Background:
    Given a fixture home built from the synthetic corpus

  Scenario: Show renders the session header and its messages
    When I run "botspy show solo-abc123" against the fixture home
    Then the exit code is 0
    And stdout contains "#1 user"
    And stdout contains "#2 user"
    And stdout contains "hello from the fixture"

  Scenario: --message selects one message
    When I run "botspy show solo-abc123 --message 2" against the fixture home
    Then the exit code is 0
    And stdout contains "#2 user"
    And stdout does not contain "#1 user"

  Scenario: An out-of-range message ordinal is an error
    When I run "botspy show solo-abc123 --message 99" against the fixture home
    Then the exit code is 1
    And stderr contains "99"

  Scenario: JSON output is the full session
    When I run "botspy show solo-abc123 -o json" against the fixture home
    Then the exit code is 0
    And stdout parses as a JSON object with "id" "solo-abc123"
    And the JSON "messages" list has 2 entries

  Scenario: NDJSON streams one message per line
    When I run "botspy show solo-abc123 -o ndjson" against the fixture home
    Then the exit code is 0
    And stdout parses as NDJSON with 2 lines

  Scenario: An unknown session id is an error
    When I run "botspy show nosuchsession" against the fixture home
    Then the exit code is 1
    And stderr contains "nosuchsession"

  Scenario: A long id may be abbreviated to an unambiguous prefix
    When I run "botspy show solo --no-truncate" against the fixture home
    Then the exit code is 0
    And stdout contains "solo-abc123"

  Scenario: An ambiguous prefix is an error listing the candidates
    When I run "botspy show 00000000" against the fixture home
    Then the exit code is 1
    And stderr contains "00000000-0000-4000-8000-000000000001" and "00000000-0000-4000-8000-000000000002" and "00000000-0000-4000-8000-000000000003"

  Scenario: Human output truncates the session id in the header
    When I run "botspy show solo-abc123" against the fixture home
    Then the exit code is 0
    And stdout contains "solo-abc"
    And stdout does not contain "solo-abc123"
    When I run "botspy show solo-abc123 --no-truncate" against the fixture home
    Then stdout contains "solo-abc123"