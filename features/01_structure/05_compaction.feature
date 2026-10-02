Feature: Compaction and summarization are first-class
  Agents summarize their own context. A compaction event is modeled with
  pre/post token counts and the summary is visible as a message, so the
  history explains its own holes.
  Evidence: Claude compact_boundary with compactMetadata (trigger, preTokens,
  postTokens, logicalParentUuid) plus isCompactSummary user messages; Codex
  compacted records with replacement_history and a window chain; Antigravity
  CHECKPOINT steps and 4.5 MB continuation summaries; Cursor legacy
  contextResiduals; Grok Bot has none locally (server-side only).

  Scenario: A compaction boundary records pre and post token counts
    Given a fixture session "c1" from agent "claude_code" in project "demo"
    When I record a compaction boundary with pre tokens 968965 and post tokens 16445
    Then the session has a compaction event
    And the compaction event has pre_tokens 968965
    And the compaction event has post_tokens 16445
    And the compaction event points at its logical parent message

  Scenario: The compacted summary is visible as a message
    Given a fixture session "c2" from agent "claude_code" in project "demo"
    When I record a compaction summary message with text "The earlier parts of the conversation..."
    Then the last message has role "user"
    And the last message is marked as a compaction summary

  Scenario: Compaction windows form a chain
    Given a fixture session "c3" from agent "codex" in project "demo"
    When I record a compaction window "w2" replacing history after window "w1"
    Then the session has a compaction window "w2"
    And compaction window "w2" has previous window "w1"
    And compaction window "w2" records its retained context

  Scenario: A checkpoint step is a compaction summary
    Given a fixture session "c4" from agent "antigravity" in project "demo"
    When I record a checkpoint step with text "{{ CHECKPOINT 0 }} The earlier parts..."
    Then the last message is marked as a compaction summary

  Scenario: Legacy compaction hints are preserved
    Given a fixture session "c5" from agent "cursor" in project "demo"
    When I record residual context of 24000 tokens
    Then the session records residual context 24000 tokens

  Scenario: Compaction that happened server-side is not invented locally
    Given a fixture session "c6" from agent "grok_bot" in project "demo"
    Then the session has no compaction events