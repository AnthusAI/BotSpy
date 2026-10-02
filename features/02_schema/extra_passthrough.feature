Feature: Raw passthrough for agent-specific content
  Content outside the fixed kinds is preserved verbatim in the raw
  passthrough, including its original kind and fields, so nothing is lost
  when an agent has content BotSpy does not model yet.

  Scenario: An unknown part kind is preserved in raw form
    Given a fixture session "e1" from agent "grok_bot" in project "demo"
    When I record a message with role "assistant" at "2026-10-01T09:06:00Z" and parts:
      | kind | raw                            |
      | mood | {"score": 0.8, "emoji": "🙂"} |
    Then the last message has 1 part with kinds "mood"
    And part 1 is not one of the fixed kinds
    And part 1 preserves the raw kind "mood"
    And part 1 preserves raw field "score" equal to "0.8"