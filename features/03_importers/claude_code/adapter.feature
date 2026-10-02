@wip
Feature: Claude Code importer
  The Claude Code importer watches append-only JSONL transcripts under
  `~/.claude/projects/<encoded-cwd>/` — one file per session — plus
  sibling `subagents/agent-*.jsonl` sidechain files, and emits normalized
  records as they appear. It never writes to the source tree.
  Evidence: session files carry `uuid`/`parentUuid`, `cost-state`,
  `pr-link`, `custom-title`, `file-history-*` records; files are appended
  live while sessions run.

  Scenario: Discovery finds one session per transcript
    Given a Claude Code source root with project transcripts "a.jsonl" and "b.jsonl"
    When the importer detects changes
    Then discovery yields 2 sessions

  Scenario: Sub-agent sidechains link to their parent session
    Given a Claude Code session transcript "a.jsonl" with sidechain "subagents/agent-1.jsonl"
    When the importer detects changes
    Then discovery yields 2 sessions
    And the sidechain session's parent_id is the main session's id

  Scenario: Appended records appear on the next read
    Given a Claude Code transcript "a.jsonl" with 3 records
    When the importer extracts the transcript
    Then extraction yields 3 good records
    When 2 more records are appended to the transcript
    And the importer extracts the transcript again from the stored offset
    Then extraction yields 2 more good records

  Scenario: Native ids become provenance, not identity
    Given a Claude Code transcript "a.jsonl" holding a user record "uuid-1" with parent "uuid-0"
    When the importer extracts the transcript
    Then the normalized message's provenance has record_id "uuid-1" and parent record "uuid-0"

  Scenario: Auxiliary records park in session metadata
    Given a Claude Code transcript "a.jsonl" holding "cost-state", "pr-link", and "custom-title" records
    When the importer detects changes
    Then the session's metadata carries the cost, PR link, and title

  Scenario: Partial trailing lines are retried, malformed records skipped
    Given a Claude Code transcript "a.jsonl" ending in a partial trailing line
    When the importer extracts the transcript
    Then extraction reports 1 skipped record
    And no partial record is normalized
    When the file is completed with the rest of the record
    And the importer extracts the transcript again from the stored offset
    Then extraction yields 1 more good record