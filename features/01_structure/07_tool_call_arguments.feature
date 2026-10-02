@wip
Feature: Tool-call arguments that are not JSON objects
  Some agents pass arguments as raw strings — JavaScript source or
  JSON-encoded strings. Arguments keep their raw form; when the raw
  string is valid JSON it is also parsed, and the raw string is kept.
  Evidence: Codex custom_tool_call.input is JavaScript source for the
  exec tool; Cursor toolFormerData.params is a JSON-encoded string;
  Antigravity argument values are strings of JSON.

  Scenario: A custom tool call keeps its raw string arguments
    Given a fixture session "a1" from agent "codex" in project "demo"
    When I record a custom tool call "a1c1" named "exec" with raw input "rm -rf /tmp/build && cargo test"
    Then the last message has 1 part with kinds "tool_call"
    And part 1 has arguments raw "rm -rf /tmp/build && cargo test"

  Scenario: A JSON-encoded string parameter is decoded and kept raw
    Given a fixture session "a2" from agent "cursor" in project "demo"
    When I record a tool call "a2c1" named "edit_file" with params "{\"path\": \"src/main.rs\", \"new_str\": \"fn main() {}\"}"
    Then part 1 has arguments parsed with path "src/main.rs"
    And part 1 has arguments raw "{\"path\": \"src/main.rs\", \"new_str\": \"fn main() {}\"}"

  Scenario: Argument values that are JSON-encoded strings are not double-decoded
    Given a fixture session "a3" from agent "antigravity" in project "demo"
    When I record a tool call "a3c1" named "run_command" with arguments {"CommandLine": "\"git status && cargo test\""}
    Then part 1 has arguments value at "CommandLine" equal to "\"git status && cargo test\""