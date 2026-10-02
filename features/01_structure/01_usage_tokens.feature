Feature: Usage and cost metrics on messages and sessions
  Token usage (input, output, cache read/write, reasoning) is first-class
  data on messages where the agent persists it, cost is session-level, and
  rate-limit/plan state is session-level too. Metrics an agent does not
  persist are absent, never zero.
  Evidence: Claude Code message.usage (incl. cache 1h/5m and thinking
  tokens) and cost-state records; Codex token_usage_record (five counters,
  turn and thread scopes) and rate_limits; Antigravity per-step usage;
  Grok Bot persists nothing locally.

  Scenario: An assistant message carries its token usage
    Given a fixture session "u1" from agent "claude_code" in project "demo"
    When I record an assistant message with usage:
      | field              | value           |
      | input_tokens       | 2               |
      | output_tokens      | 243             |
      | cache_read_tokens  | 40530           |
      | cache_write_tokens | 28233           |
      | reasoning_tokens   | 110             |
      | model              | claude-opus-5-5 |
    Then the last message usage has input_tokens 2
    And the last message usage has output_tokens 243
    And the last message usage has cache_read_tokens 40530
    And the last message usage has cache_write_tokens 28233
    And the last message usage has reasoning_tokens 110
    And the last message usage has model "claude-opus-5-5"

  Scenario: A session cost in USD comes from Claude Code's cost-state record
    Given a fixture session "u2" from agent "claude_code" in project "demo"
    When the session carries a cost record:
      | field                  | value   |
      | total_cost_usd         | 12.40   |
      | total_api_duration_ms  | 194000  |
      | total_tool_duration_ms | 86000   |
      | total_lines_added      | 412     |
      | total_lines_removed    | 33      |
    Then the session cost is 12.40 USD
    And the session cost per model "claude-opus-5-5" is present

  Scenario: Codex token usage is recorded per API response and accumulates per turn and per thread
    Given a fixture session "u3" from agent "codex" in project "demo"
    When I record an assistant message with usage:
      | field               | value |
      | input_tokens        | 5120  |
      | cached_input_tokens | 3072  |
      | cache_write_tokens  | 2048  |
      | output_tokens       | 1229  |
      | reasoning_tokens    | 940   |
    Then the last message usage has reasoning_tokens 940
    And the turn usage accumulates the message usages of its turn
    And the thread usage accumulates the turn usages of the session

  Scenario: Rate-limit and plan state lives at session level
    Given a fixture session "u4" from agent "codex" in project "demo"
    When the session carries rate-limit state:
      | field          | value                |
      | plan_type      | prolite              |
      | used_percent   | 61.4                 |
      | window_minutes | 300                  |
      | resets_at      | 2026-10-02T18:00:00Z |
    Then the session rate limit is 61.4 percent used in a 300 minute window
    And the session plan type is "prolite"

  Scenario: Antigravity records usage per step
    Given a fixture session "u6" from agent "antigravity" in project "demo"
    When I record an assistant message with usage:
      | field            | value |
      | input_tokens     | 1318  |
      | output_tokens    | 24282 |
      | reasoning_tokens | 612   |
      | model            | gemini-3.8-flash |
    Then the last message usage has reasoning_tokens 612
    And the last message usage has model "gemini-3.8-flash"

  Scenario: Usage an agent does not persist is absent, not zero
    Given a fixture session "u5" from agent "grok_bot" in project "demo"
    When I record a message with role "assistant" at "2026-10-01T09:07:00Z" and parts:
      | kind | text                  |
      | text | Done, the tests pass. |
    Then the last message has no usage
    And the session has no usage
    And the session has no cost