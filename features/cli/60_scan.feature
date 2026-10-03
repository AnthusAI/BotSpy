Feature: botspy scan
  botspy scan mines the adapters into the local store — the store's
  ingest/CDC pass, on the command line. By default every registered
  source is scanned into the one store at $BOTSPY_HOME/.botspy/store.db.
  The pass adds new sessions and updates changed ones; it never prunes:
  an adapter that reports nothing must never delete stored history, so a
  source yielding zero sessions (the Codex zero-sessions bug BOTSPY-81143b)
  stays visible without being destructive. Exit codes: 0 success
  (including adapters that report nothing), 1 unknown source, 2 usage
  error, 3 store failure.

  Background:
    Given a fixture home built from the synthetic corpus

  Scenario: A default scan fills the store from every source
    When I run "botspy scan" with BOTSPY_HOME at the fixture home
    Then the exit code is 0
    And the fixture store holds 12 sessions
    And stdout contains "claude_code", "cursor", "codex", "grok_bot" and "antigravity"
    And stdout reports "12 added"

  Scenario: The scan report names the store and its elapsed time
    When I run "botspy scan -o json --store {out_dir}/scan-meta.db" against the fixture home
    Then the exit code is 0
    And the JSON "store" is "{out_dir}/scan-meta.db"
    And the JSON "elapsed_ms" is more than 0
    And the JSON totals report "new" 12

  Scenario: A dry-run scan reports what would change and writes nothing
    When I run "botspy scan --dry-run" with BOTSPY_HOME at the fixture home
    Then the exit code is 0
    And stdout reports "12 added"
    And stdout reports "dry run"
    And the fixture store holds 0 sessions

  Scenario: JSON output carries the per-adapter report
    When I run "botspy scan -o json" with BOTSPY_HOME at the fixture home
    Then the exit code is 0
    And stdout parses as a JSON object
    And the JSON "adapters" list has 5 entries
    And the JSON scan source "claude_code" has "discovered" 4 and "added_sessions" 4
    And the JSON scan source "cursor" has "discovered" 2 and "added_sessions" 2
    And the JSON scan source "codex" has "discovered" 2 and "added_sessions" 2
    And the JSON scan source "grok_bot" has "discovered" 3 and "added_sessions" 2
    And the JSON scan source "grok_bot" has "errors" 1
    And the JSON scan source "antigravity" has "discovered" 2 and "added_sessions" 2

  Scenario: NDJSON streams one entry per source
    When I run "botspy scan -o ndjson" with BOTSPY_HOME at the fixture home
    Then the exit code is 0
    And stdout parses as NDJSON with 5 lines

  Scenario: JSON output counts the messages the scan brought in
    When I run "botspy scan --source claude_code -o json --store {out_dir}/scan-msgs.db" against the fixture home
    Then the exit code is 0
    And the JSON scan source "claude_code" has "added_sessions" 4 and "added_messages" 8

  Scenario: --source scans a single adapter
    When I run "botspy scan --source claude_code --store {out_dir}/scan-single.db" against the fixture home
    Then the exit code is 0
    And the store at "{out_dir}/scan-single.db" holds 4 sessions

  Scenario: --source can be repeated
    When I run "botspy scan --source claude_code --source codex --store {out_dir}/scan-list.db" against the fixture home
    Then the exit code is 0
    And the store at "{out_dir}/scan-list.db" holds 6 sessions

  Scenario: --since keeps sessions last active before the cutoff out of the store
    When I run "botspy scan --source claude_code --since 2026-09-20T00:00:00Z --store {out_dir}/scan-since.db -o json" against the fixture home
    Then the exit code is 0
    And the store at "{out_dir}/scan-since.db" holds 3 sessions
    And the JSON scan source "claude_code" has "skipped_sessions" 1

  Scenario: --since accepts an ISO date
    When I run "botspy scan --source claude_code --since 2026-09-20 --store {out_dir}/scan-date.db" against the fixture home
    Then the exit code is 0
    And the store at "{out_dir}/scan-date.db" holds 3 sessions

  Scenario: --since accepts a relative duration
    When I run "botspy scan --source claude_code --since 1h --store {out_dir}/scan-recent.db" against the fixture home
    Then the exit code is 0
    And the store at "{out_dir}/scan-recent.db" holds 0 sessions

  Scenario: --since accepts a duration that reaches back past every session
    When I run "botspy scan --since 10000w --store {out_dir}/scan-old.db" against the fixture home
    Then the exit code is 0
    And the store at "{out_dir}/scan-old.db" holds 12 sessions

  Scenario: A cutoff scan never deletes the stored sessions it skipped
    When I run "botspy scan --store {out_dir}/scan-keep.db" against the fixture home
    Then the exit code is 0
    And the store at "{out_dir}/scan-keep.db" holds 12 sessions
    When I run "botspy scan --since 2999-01-01T00:00:00Z --store {out_dir}/scan-keep.db -o json" against the fixture home
    Then the exit code is 0
    And the store at "{out_dir}/scan-keep.db" holds 12 sessions
    And the JSON scan source "claude_code" has "skipped_sessions" 4

  Scenario: A source with no sessions is reported with its zero row
    Given a fixture home with only the sources "claude_code"
    When I run "botspy scan --store {out_dir}/scan-zeros.db -o json" against the fixture home
    Then the exit code is 0
    And the JSON scan source "codex" has "discovered" 0 and "added_sessions" 0
    And the JSON scan source "codex" has "errors" 0
    And the JSON totals report "new" 4

  Scenario: An unknown source is rejected
    When I run "botspy scan --source notepad"
    Then the exit code is 1
    And stderr contains "notepad", "claude_code" and "antigravity"

  Scenario: An invalid --since value is a usage error
    When I run "botspy scan --since bogus"
    Then the exit code is 2
    And stderr contains "--since" "bogus"

  Scenario: A file that is not a store fails cleanly
    Given a non-store file at "{out_dir}/bogus.db"
    When I run "botspy scan --store {out_dir}/bogus.db" against the fixture home
    Then the exit code is 3
    And stderr mentions "not a BotSpy store"