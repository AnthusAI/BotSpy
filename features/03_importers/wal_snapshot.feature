@wip
Feature: Read-only sources and safe SQLite/WAL snapshotting
  Adapters never write to the sources they tap. SQLite sources running in
  WAL mode — a live app writing while we read — are read through a safe
  snapshot copy, never in place, and never interfere with the writer.
  Evidence: BOTSPY-296d2c; Cursor's store is a hot WAL database.

  Scenario: A SQLite/WAL source is read through a safe snapshot copy
    Given a SQLite source in WAL mode at "roots/demo/store.db" with 5 rows and a live writer appending rows
    When I read the source through a snapshot copy
    Then the snapshot yields a consistent read of the rows present at snapshot time
    And the source database file and its WAL file were never mutated
    And the live writer kept appending without interference