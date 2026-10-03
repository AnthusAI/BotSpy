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

  Scenario: A cutoff keeps sessions last active before it out of the ingest
    Given fixture sessions "c1" last active at "2026-10-01T09:00:00Z" and "c2" last active at "2026-09-01T09:00:00Z" and "c3" last active at "2026-10-01T09:30:00Z"
    And a store at "cutoff/store.db"
    When I ingest the registered adapters into the store with a cutoff "2026-10-01T09:00:00Z"
    Then the ingest reports 2 new sessions
    And the ingest reports 1 skipped session
    When I iterate the store's sessions
    Then the store iteration yields sessions "c3" and "c1"

  Scenario: A cutoff never drops sessions with no recorded activity
    Given fixture sessions "c4" last active at "" and "c5" last active at "2026-10-01T09:00:00Z" and "c6" last active at "2026-09-01T09:00:00Z"
    And a store at "cutoff-blank/store.db"
    When I ingest the registered adapters into the store with a cutoff "2026-10-01T00:00:00Z"
    Then the ingest reports 2 new sessions
    And the ingest reports 1 skipped session
    When I iterate the store's sessions
    Then the store iteration yields sessions "c5" and "c4"

  Scenario: A dry-run ingest reports what would change and writes nothing
    Given fixture sessions "d1" from "claude_code" and "d2" from "cursor"
    And a store at "dryrun/store.db"
    When I ingest the registered adapters into the store without writing
    Then the ingest reports 2 new sessions
    When I iterate the store's sessions
    Then the store iteration yields 0 sessions
    When I ingest the registered adapters into the store
    Then the ingest reports 2 new sessions
    When I iterate the store's sessions
    Then the store iteration yields 2 sessions

  Scenario: The ingest reports what each adapter contributed
    Given fixture sessions "p1" from "claude_code" and "p2" from "cursor" and "p3" from "cursor"
    And an empty fixture adapter for "codex"
    And a store at "per-adapter/store.db"
    When I ingest the registered adapters into the store
    Then the ingest reports 3 new sessions
    And the ingest reports 1 added session for "claude_code"
    And the ingest reports 2 added sessions for "cursor"
    And the ingest reports 1 message added by "claude_code"
    And the ingest reports 2 messages added by "cursor"
    And the ingest reports 0 added sessions for "codex"

  Scenario: A session the adapter cannot open is reported, not silently dropped
    Given fixture sessions "u1" from "claude_code" and "u2" from "cursor"
    And session "u1" cannot be opened from its adapter
    And a store at "unopenable/store.db"
    When I ingest the registered adapters into the store
    Then the ingest reports 1 new session
    And the ingest reports 1 session that failed to open