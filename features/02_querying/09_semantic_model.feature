Feature: The semantic model
  The store's semantic half is the shipped embedding model:
  all-MiniLM-L6-v2, 384 dimensions, loaded from the models cache the
  setup step fetched (BOTSPY-c3cf41) and run through tract on CPU
  (BOTSPY-e95fe9). Ungated per the BOTSPY-bb60bb decision: these specs
  run the real model in every CI job. Ingest records the model id in
  meta — the store's identity for its vectors — and search matches
  sessions by meaning: a paraphrase with no word in common still finds
  its session, and k composes with the relevance floor across a mixed
  fixture.

  Scenario: The store records the embedding model it embeds with
    Given the store was opened with the "all-MiniLM-L6-v2" embedder
    And a fixture session "m1" from agent "claude_code" in project "demo" is registered
    And I ingest the registered adapters into the store
    Then the store's embedding model is "all-MiniLM-L6-v2"

  Scenario: A store without an embedder records no embedding model
    Given fixture sessions "w1" and "w2" from "claude_code"
    And session "w1" has messages "fix the login bug"
    And a store at "modelless.db" with those sessions ingested
    Then the store records no embedding model

  Scenario: A paraphrase finds its session without sharing a word
    Given the store was opened with the "all-MiniLM-L6-v2" embedder
    And fixture sessions "s1", "s2", and "s3" from "claude_code" in project "demo"
    And session "s1" has messages "authentication is broken for every account since the morning deploy"
    And session "s2" has messages "the quarterly report spreadsheet needs one more column"
    And session "s3" has messages "the office coffee machine is out of beans"
    When I search the store semantically for "nobody can log into their workspace anymore"
    Then the search yields sessions "s1" only
    And every search result carries a score

  Scenario: k composes with the relevance floor
    Given the store was opened with the "all-MiniLM-L6-v2" embedder
    And fixture sessions "p1", "p2", "p3", "p4", and "p5" from "claude_code" in project "demo"
    And session "p1" has messages "users cannot sign in, the session token keeps expiring"
    And session "p2" has messages "single sign-on stopped working after the upgrade"
    And session "p3" has messages "add a dark mode toggle to the settings page"
    And session "p4" has messages "the export to CSV includes duplicate rows"
    And session "p5" has messages "the staging database migration failed overnight"
    When I search the store semantically for "log in not working" with k 2
    Then the search yields sessions "p2" and "p1" in that order
    And every search result carries a score