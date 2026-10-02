Feature: Turn grouping with turn-level timing and errors
  A turn is one user request plus everything the agent did to answer it.
  Messages carry turn_id and parent_turn_id; turns carry timing and
  errors, so latency and failure metrics are computable per turn.
  Evidence: Codex turn_id/root_turn_id with task_complete duration_ms and
  time_to_first_token_ms; Claude promptId and turnPosition {promptIndex,
  turnIndex}; Cursor conversationTurnIndex, requestId, stepDurationMs and
  turnDurationMs; Cursor CLI turn_ended status markers.

  Scenario: A user request and its responses share one turn
    Given a fixture session "n1" from agent "claude_code" in project "demo"
    When I record a user message in turn "turn-1"
    And I record an assistant reply in turn "turn-1"
    Then the last 2 messages share turn_id "turn-1"

  Scenario: A turn position groups prompts and their turns
    Given a fixture session "n2" from agent "claude_code" in project "demo"
    When I record a user message in turn "turn-2" at prompt index 0 turn index 1
    Then the last message records turn position prompt 0 turn 1

  Scenario: Sub-turns point at their root turn
    Given a fixture session "n3" from agent "codex" in project "demo"
    When I record a sub-turn "turn-3b" with parent turn "turn-3"
    Then turn "turn-3b" has parent_turn_id "turn-3"

  Scenario: A completed turn records its duration and time to first token
    Given a fixture session "n4" from agent "codex" in project "demo"
    When I record a completed turn "turn-4" with duration 14300 ms and time to first token 850 ms
    Then turn "turn-4" has duration_ms 14300
    And turn "turn-4" has time_to_first_token_ms 850

  Scenario: A turn's step and bubble timings are preserved
    Given a fixture session "n5" from agent "cursor" in project "demo"
    When I record a message in turn "turn-5" with step duration 420 ms and turn duration 3900 ms
    Then turn "turn-5" has duration_ms 3900
    And the last message records step duration_ms 420

  Scenario: A CLI turn is closed by its end marker
    Given a fixture session "n6" from agent "cursor" in project "demo"
    When I record a turn end marker with status "success"
    Then the turn is marked completed with status "success"