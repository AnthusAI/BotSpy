@wip
Feature: Text search
  The store answers "which sessions mention this?" over every source: a
  full-text match (FTS5) over the message text projections written at
  ingest, ranked and case-insensitive, returning the matched sessions
  with their matching messages. A query that matches nothing is an empty
  result, never an error. Search composes with the session filters.
  Evidence: transcripts exceed 100 MB (some trees hold 1.4 GB and 6 GB
  stores); recall must not require loading sessions into memory.

  Scenario: Sessions are found by a word in their messages
    Given fixture sessions "t1" and "t2" from "claude_code" in project "demo"
    And session "t1" has messages "fix the login bug" and "retry the login flow"
    And session "t2" has messages "write the release notes"
    When I search the store for "login"
    Then the search yields sessions "t1" only

  Scenario: More matching messages rank a session higher
    Given fixture sessions "r1" and "r2" from "claude_code" in project "demo"
    And session "r1" has messages "login failed" and "login retry worked" and "login metrics look fine"
    And session "r2" has messages "login failed"
    When I search the store for "login"
    Then the search yields sessions "r1" and "r2" in that order

  Scenario: Search is case-insensitive
    Given a fixture session "c1" from agent "claude_code" in project "demo"
    And session "c1" has messages "Fix the LOGIN bug"
    When I search the store for "login"
    Then the search yields sessions "c1" only

  Scenario: No match is an empty result, not an error
    Given a fixture session "n1" from agent "claude_code" in project "demo"
    And session "n1" has messages "write the release notes"
    When I search the store for "floccinaucinihilipilification"
    Then the search yields no sessions

  Scenario: Punctuation in the query never breaks the search
    Given a fixture session "p1" from agent "claude_code" in project "demo"
    And session "p1" has messages "fix the login bug"
    When I search the store for "login!"
    Then the search yields sessions "p1" only

  Scenario: Search composes with the session filters
    Given fixture sessions "f1" from "claude_code" in project "web" and "f2" from "cursor" in project "web"
    And session "f1" has messages "fix the login bug"
    And session "f2" has messages "fix the login bug"
    When I search the store for "login" filtered by source "cursor"
    Then the search yields sessions "f2" only

  Scenario: The search reports the matched messages
    Given a fixture session "m1" from agent "claude_code" in project "demo"
    And session "m1" has messages "fix the login bug" and "ship the release"
    When I search the store for "release"
    Then the search yields sessions "m1" only
    And the search yields the matched texts "ship the release"