Feature: Normalized message parts
  A message is made of ordered typed parts: text, thinking, tool_call,
  tool_result, attachment, system.

  Scenario: A message with text, thinking, and a tool call
    Given a fixture session "s1" from agent "claude_code" in project "demo"
    When I record a message with role "assistant" at "2026-10-01T09:02:00Z" and parts:
      | kind      | text                  | id | name      |
      | text      | The fix is ready      |    |           |
      | thinking  | Inspecting parser.rs  |    |           |
      | tool_call |                       | t1 | read_file |
    Then the last message has 3 parts with kinds "text, thinking, tool_call"
    And part 3 is a tool_call with id "t1" and name "read_file"

  Scenario: A tool result references its tool call by id
    Given a fixture session "s2" from agent "cursor" in project "demo"
    When I record a message with role "assistant" at "2026-10-01T09:04:00Z" and parts:
      | kind        | call_id | id | name      |
      | tool_call   |         | t1 | read_file |
      | tool_result | t1      |    |           |
    Then the last message has 2 parts with kinds "tool_call, tool_result"
    And part 2 references call "t1"

  Scenario: An attachment carries path, mime, and size
    Given a fixture session "s3" from agent "codex" in project "demo"
    When I record a message with role "assistant" at "2026-10-01T09:05:00Z" and parts:
      | kind       | path         | mime          | size |
      | attachment | docs/spec.md | text/markdown | 512  |
    Then the last message has 1 part with kinds "attachment"
    And part 1 is an attachment with path "docs/spec.md", mime "text/markdown" and size 512