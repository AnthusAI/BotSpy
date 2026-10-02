@wip
Feature: Semantic and hybrid search
  With the store's embedder configured (all-MiniLM-L6-v2, 384-dim), ingest
  embeds every session and semantic search returns the nearest sessions
  for a query text, each with a similarity score and a caller-chosen k.
  Text and vector results merge into one hybrid ranking (reciprocal-rank
  fusion): a session that both words-match and embeds near the query
  ranks first. A store opened without an embedder answers text search
  normally and answers semantic search with a clean empty result. Stores
  never mix embedding models: the first embed records the model id in
  meta, and a different one is refused.

  Scenario: Semantic search returns the nearest sessions
    Given the store was opened with the "all-MiniLM-L6-v2" embedder
    And fixture sessions "v1", "v2", and "v3" from "claude_code" in project "demo"
    And session "v1" has messages "users cannot sign in, the session token keeps expiring"
    And session "v2" has messages "add a dark mode toggle to the settings page"
    And session "v3" has messages "the staging database migration failed overnight"
    When I search the store semantically for "log in not working"
    Then the search yields sessions "v1" only
    And every search result carries a score

  Scenario: The caller chooses k
    Given the store was opened with the "all-MiniLM-L6-v2" embedder
    And fixture sessions "k1", "k2", and "k3" from "claude_code" in project "demo"
    And session "k1" has messages "users cannot sign in, the session token keeps expiring"
    And session "k2" has messages "single sign-on stopped working after the upgrade"
    And session "k3" has messages "add a dark mode toggle to the settings page"
    When I search the store semantically for "log in not working" with k 2
    Then the search yields 2 sessions
    And every search result carries a score

  Scenario: Hybrid ranking merges lexical and semantic matches
    Given the store was opened with the "all-MiniLM-L6-v2" embedder
    And fixture sessions "h1", "h2", and "h3" from "claude_code" in project "demo"
    And session "h1" has messages "the release notes are still missing"
    And session "h2" has messages "the release failed and we rolled it back"
    And session "h3" has messages "we shipped a broken build to production"
    When I run the hybrid search for "release broke"
    Then the search yields sessions "h2", "h3", and "h1" in that order
    And every search result carries a score

  Scenario: A store without an embedder answers text search and returns an empty semantic result
    Given fixture sessions "w1" and "w2" from "claude_code"
    And session "w1" has messages "fix the login bug"
    And a store at "no-embedder.db" with those sessions ingested
    When I search the store semantically for "log in not working"
    Then the search yields no sessions
    And searching the store for "login" still yields sessions "w1" only

  Scenario: Stores never mix embedding models
    Given the store was opened with the "all-MiniLM-L6-v2" embedder
    And a fixture session "x1" from agent "claude_code" in project "demo" is registered
    And I ingest the registered adapters into the store
    When I ingest into the store with an embedder claiming model "other-model-v1"
    Then the ingest fails with a model mismatch error