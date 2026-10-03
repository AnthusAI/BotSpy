Feature: Incremental refresh
  Refresh keeps the store in step with its sources without re-mining
  everything: a pass opens only the sessions whose summaries changed,
  leaves unchanged sessions untouched, and prunes sessions the source no
  longer reports. Running it again changes nothing.
  Evidence: BOTSPY-c7795b Local Store Initiative; transcripts exceed
  100 MB (some trees hold 1.4 GB and 6 GB stores), so re-mining every
  session on every pass does not scale.

  Scenario: A second pass ingests only what is new
    Given fixture sessions "f1" from "claude_code" and "f2" from "cursor"
    And a store at "refresh-new/store.db"
    When I ingest the registered adapters into the store
    And a fixture session "f3" from "codex" is registered
    And I refresh the store
    Then the refresh reports 1 new session, 0 updated, and 0 pruned
    When I iterate the sessions
    Then the iteration yields sessions "f1", "f2", and "f3"

  Scenario: A changed session is re-ingested and queries return the new content
    Given fixture sessions "f4" from "claude_code" and "f5" from "cursor"
    And a store at "refresh-changed/store.db"
    When I ingest the registered adapters into the store
    And session "f4" gets another message at "2026-10-01T10:00:00Z"
    And I refresh the store
    Then the refresh reports 0 new sessions, 1 updated, and 0 pruned
    When I iterate the messages of session "f4"
    Then the iteration yields the texts "hello from f4", "and one more" in that order

  Scenario: A session gone from the source is pruned
    Given fixture sessions "f6" from "claude_code" and "f7" from "cursor"
    And a store at "refresh-prune/store.db"
    When I ingest the registered adapters into the store
    And session "f7" is removed from its adapter
    And I refresh the store
    Then the refresh reports 0 new sessions, 0 updated, and 1 pruned
    When I iterate the sessions
    Then the iteration yields sessions "f6"

  Scenario: Refresh is idempotent
    Given fixture sessions "f8" from "claude_code" and "f9" from "cursor" and "g0" from "codex"
    And a store at "refresh-idempotent/store.db"
    When I ingest the registered adapters into the store
    And I refresh the store
    Then the refresh reports 0 new sessions, 0 updated, and 0 pruned
    When I iterate the sessions
    Then the iteration yields sessions "f8", "f9", and "g0"

  Scenario: Unchanged sessions are not re-opened from their adapters
    Given fixture sessions "f11" from "claude_code" and "f12" from "cursor"
    And a store at "refresh-open-count/store.db"
    When I ingest the registered adapters into the store
    And I snapshot the adapters' open count
    And I refresh the store
    And I snapshot the adapters' open count again
    Then the refresh reports 0 new sessions, 0 updated, and 0 pruned
    And the adapters served no session opens during the refresh