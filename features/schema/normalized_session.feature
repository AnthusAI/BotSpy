Feature: Normalized schema
  A session is made of ordered messages whose parts each have a kind
  (text, thinking, tool_call, tool_result, attachment, system). Agent-specific
  content outside the fixed kinds is preserved in extra.

  Scenario: A session holds ordered messages with typed parts
    Given a session from "claude-code" in project "demo"
    When I append a message with parts:
      | kind       | text                    |
      | system     | context injected        |
      | text       | please fix the bug      |
      | thinking   | the bug is in parser.py |
      | tool_call  | read_file parser.py     |
      | tool_result| def parse(): ...        |
      | attachment | patch.diff              |
      | text       | done, tests pass        |
    Then the session has 1 message with 7 parts in the given order
    And every part kind is one of text, thinking, tool_call, tool_result, attachment, system

  Scenario: Unknown agent-specific content is preserved in extra
    Given a session from "grok-bot" in project "demo"
    When I append a message with a part of unknown kind "mood"
    Then the part is normalized to kind "text"
    And the original kind "mood" is preserved in extra