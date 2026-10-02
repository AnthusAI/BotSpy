@wip
Feature: botspy snapshot
  A WAL-safe snapshot of a SQLite source: a consistent copy taken over a
  read-only connection, with proof the source was not mutated — the same
  mechanism the library uses to read hot WAL databases. A snapshot is
  addressed by source name or by path, and lands where --out points.

  Background:
    Given a fixture home built from the synthetic corpus

  Scenario: Snapshot a SQLite source by name
    When I run "botspy snapshot cursor --out {out_dir}" against the fixture home
    Then the exit code is 0
    And the path after "snapshot:" exists
    And stdout contains "unmutated"
    And the number after "rows:" is more than 0

  Scenario: Snapshot a SQLite file by path
    When I run "botspy snapshot {cursor_store_path} --out {out_dir}" against the fixture corpus
    Then the exit code is 0
    And the path after "snapshot:" exists
    And stdout contains "unmutated"

  Scenario: JSON output reports the snapshot
    When I run "botspy snapshot cursor --out {out_dir} -o json" against the fixture home
    Then the exit code is 0
    And stdout parses as a JSON object
    And the JSON "unmutated" is true
    And the JSON "rows" is more than 0

  Scenario: An unknown source name is rejected
    When I run "botspy snapshot notepad --out {out_dir}"
    Then the exit code is 1
    And stderr contains "notepad"

  Scenario: A source without a SQLite database fails cleanly
    When I run "botspy snapshot claude_code --out {out_dir}" against the fixture home
    Then the exit code is 3
    And stderr contains "claude_code"