Feature: Ingesting the adapters into the store
  Ingestion mines every registered adapter into the store: discovery
  first, then opening each session and writing its normalized rows in one
  transaction. The sources stay read-only — ingestion never writes the
  data it mines. A session reported by two adapters is stored once (first
  report wins, the chapter-1 rule), and what comes back out is the session
  the adapter produced, exactly.
  Evidence: BOTSPY-c7795b Local Store Initiative; the chapter-1
  first-report-wins rule and the importer contract's digest proof.

  Scenario: Ingesting registered adapters fills the store
    Given fixture sessions "g1" from "claude_code" and "g2" from "cursor"
    And a store at "ingest/store.db"
    When I ingest the registered adapters into the store
    Then the ingest reports 2 new sessions
    When I iterate the store's sessions
    Then the store iteration yields 2 sessions

  Scenario: A session reported by two adapters is stored once
    Given fixture session "g3" from "claude_code" and "g3" from "cursor"
    And a store at "dedup/store.db"
    When I ingest the registered adapters into the store
    Then the ingest reports 1 new session
    When I iterate the store's sessions
    Then the store iteration yields sessions "g3"

  Scenario: An ingested session round-trips losslessly
    Given a fixture session "g4" from agent "claude_code" in project "demo"
    And the session has messages "hello", "hi there", "now fix it"
    And a store at "roundtrip/store.db"
    When I ingest the registered adapters into the store
    And I open the stored session "g4"
    Then it equals the session the adapter returns

  Scenario: Ingestion never writes the sources
    Given fixture sessions "n1" from "claude_code" and "n2" from "cursor"
    And a store at "readonly/store.db"
    When I snapshot the adapters' sessions digest
    And I ingest the registered adapters into the store
    And I snapshot the adapters' sessions digest again
    Then the sources are unchanged by the ingest

  Scenario: An empty registry is a clean no-op
    Given a store at "empty/store.db"
    When I ingest the registered adapters into the store
    Then the ingest reports 0 new sessions
    When I iterate the store's sessions
    Then the store iteration yields 0 sessions