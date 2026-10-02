Feature: Error and status flags on tool results, messages, and turns
  Tool results carry their outcome (ok, error, interrupted), tool calls
  keep their own status, and messages and turns can be flagged as failed
  or aborted, so failure is observable in the normalized history.
  Evidence: Claude tool_result.is_error and toolUseResult.interrupted;
  Cursor toolFormerData.status and CLI turn_ended.status; Codex
  thread_turns.error_json (usageLimitExceeded) and turn_aborted.reason;
  Antigravity error steps (HTTP 429 RESOURCE_EXHAUSTED) and status enums.

  Scenario: A failed tool result carries an error flag
    Given a fixture session "t1" from agent "claude_code" in project "demo"
    When I record a tool result for call "t1" that is an error with text "command not found"
    Then the last message has 1 part with kinds "tool_result"
    And part 1 references call "t1"
    And part 1 has status "error"

  Scenario: An interrupted tool result is distinguished from a failed one
    Given a fixture session "t2" from agent "claude_code" in project "demo"
    When I record a tool result for call "t2" that was interrupted with output "partial stdout"
    Then part 1 has status "interrupted"

  Scenario: A tool call's own status is preserved
    Given a fixture session "t3" from agent "cursor" in project "demo"
    When I record a tool call "t3" named "run_terminal" with status "loading"
    Then the last message has 1 part with kinds "tool_call"
    And part 1 has status "loading"

  Scenario: A turn aborted mid-flight is recorded with its reason
    Given a fixture session "t4" from agent "codex" in project "demo"
    When I record a turn that was aborted with reason "interrupted"
    Then the turn is marked aborted with reason "interrupted"

  Scenario: A usage-limit failure surfaces as a turn error
    Given a fixture session "t5" from agent "codex" in project "demo"
    When I record a turn that failed with error "usageLimitExceeded"
    Then the turn is marked failed with error "usageLimitExceeded"

  Scenario: An agent-level error event is recorded with its status
    Given a fixture session "t6" from agent "antigravity" in project "demo"
    When I record an error event with text "RESOURCE_EXHAUSTED (code 429): HTTP 429"
    Then the last message has role "system"
    And the last message is marked as an error

  Scenario: A CLI turn end marker with a failing status closes the turn
    Given a fixture session "t7" from agent "cursor" in project "demo"
    When I record a turn end marker with status "failed"
    Then the turn is marked failed with status "failed"