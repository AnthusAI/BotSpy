@wip
Feature: The local store
  The unified history is backed by one local store: a single SQLite file
  the caller can place wherever they like. Opening it is explicit,
  reopening it keeps everything previously ingested, and readers keep
  querying while another connection writes — an out-of-process scanner can
  ingest while an app reads the same file. A file that is not a store is a
  clean typed error, never a panic. The default store path derives from
  BOTSPY_HOME, falling back to $HOME, as ~/.botspy/store.db — the same
  home the CLI's snapshots live under.
  Evidence: BOTSPY-c7795b Local Store Initiative (SQLite + sqlite-vec);
  the CLI's ~/.botspy/snapshots convention.

  Scenario: A store opens at a caller-given path, creating parent directories
    Given no store exists at "stores/nested/store.db"
    When I open a store at "stores/nested/store.db"
    Then the store file exists at "stores/nested/store.db"

  Scenario: Reopening a store persists previously ingested data
    Given fixture sessions "st1" from "claude_code" and "st2" from "cursor"
    And a store at "persist/store.db" with those sessions ingested
    When I reopen a store at "persist/store.db"
    And I iterate the store's sessions
    Then the store iteration yields 2 sessions
    And the store iteration yields sessions "st1" and "st2"

  Scenario: A reader on a second connection keeps querying while an ingest commits
    Given fixture sessions "rw1", "rw2", and "rw3" from "claude_code"
    And a store at "shared/store.db" with those sessions ingested
    And a second reader is open on the store at "shared/store.db" and has taken its first session
    When a new fixture session "rw4" is ingested through the writer
    And the reader continues its iteration
    Then the reader finishes its iteration without error

  Scenario: A file that is not a store is a clean typed error, never a panic
    Given a file "junk/store.db" containing "this is not a sqlite database"
    When I open a store at "junk/store.db"
    Then opening fails with a store error, not a panic

  Scenario: The default store path follows BOTSPY_HOME
    Given BOTSPY_HOME is "homes/agent"
    When I open the default store
    Then the store file exists at "homes/agent/.botspy/store.db"