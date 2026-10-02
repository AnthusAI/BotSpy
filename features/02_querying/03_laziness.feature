Feature: Query laziness
  Iteration is lazy: the engine reads and normalizes only what the caller
  actually consumes. Taking the first session must not open, read, or
  normalize the rest — the same streaming discipline the importers follow.
  Evidence: transcripts exceed 100 MB (some trees hold 1.4 GB and 6 GB
  stores); eager materialization does not scale.

  Scenario: Taking the first session does not open the others
    Given fixture sessions "l1", "l2", and "l3" from "claude_code"
    When I take the first 1 session from the iteration
    Then the iteration yields sessions "l1" only
    And only session "l1" was opened

  Scenario: Dropping an iterator stops the work
    Given fixture sessions "l4", "l5", and "l6" from "codex"
    When I iterate 1 session and drop the iterator
    Then the store performed only 1 session open

  Scenario: Message iteration streams without loading the whole session
    Given a fixture session "l7" from "cursor" with 500 messages
    When I iterate the first 5 messages of session "l7"
    Then the iteration yields 5 messages
    And the session was not fully materialized