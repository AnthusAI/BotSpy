@wip
Feature: Query iteration over the unified history
  One query interface fronts the whole history. Callers iterate sessions,
  their messages, and those messages' parts in order — without knowing or
  caring which engine (SQLite, sqlite-vec) executes the query. No SQL
  string, no SQLite type, and no vector index leaks through the API.
  Evidence: BOTSPY-c7795b Local Store Initiative (SQLite + sqlite-vec);
  the retired Python sessions(source=None) API.

  Scenario: Sessions iterate across every registered source
    Given fixture sessions "q1" from "claude_code" and "q2" from "cursor" and "q3" from "codex"
    When I iterate the sessions
    Then the iteration yields 3 sessions
    And the iteration yields sessions "q1", "q2", and "q3"

  Scenario: A session's messages iterate in source order
    Given a fixture session "q4" from agent "claude_code" in project "demo"
    And the session has messages "hello", "hi there", "now fix it"
    When I iterate the messages of session "q4"
    Then the iteration yields the texts "hello", "hi there", "now fix it" in that order

  Scenario: A message's parts iterate in order
    Given a fixture session "q5" from agent "claude_code" in project "demo"
    And the session has a message with parts "text", "thinking", "tool_call"
    When I iterate the parts of the last message of session "q5"
    Then the iteration yields the kinds "text", "thinking", "tool_call" in that order