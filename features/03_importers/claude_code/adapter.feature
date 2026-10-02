Feature: Claude Code importer
  The Claude Code importer watches append-only JSONL transcripts under
  `~/.claude/projects/<encoded-cwd>/` — one file per session — and emits
  normalized records as they appear. Sidechain files under `subagents/`
  are skipped and counted. The source tree is only ever read.
  Evidence: session files carry `uuid`/`parentUuid`, `cost-state`,
  `pr-link`, `custom-title` records; files are appended live while
  sessions run.

  Scenario: Discovery finds one session per transcript
    Given a Claude Code source root with project transcripts "a.jsonl" and "b.jsonl"
    When the Claude Code importer detects changes
    Then discovery yields 2 sessions

  Scenario: Sub-agent sidechains are skipped and counted
    Given a Claude Code session transcript "a.jsonl" with sidechain "subagents/agent-1.jsonl"
    When the Claude Code importer detects changes
    Then discovery yields 1 session
    And the doctor report counts 1 skipped subagent

  Scenario: Appended records appear on the next read
    Given a Claude Code transcript "a.jsonl" with 3 records
    When the Claude Code importer extracts the transcript
    Then extraction yields 3 messages
    When 2 more records are appended to the transcript
    And the Claude Code importer extracts the transcript again
    Then extraction yields 2 more messages

  Scenario: Native ids become provenance, not identity
    Given a Claude Code transcript "a.jsonl" holding a user record "uuid-1" with parent "uuid-0"
    When the Claude Code importer extracts the transcript
    Then the first message's provenance has record_id "uuid-1" and parent record "uuid-0"

  Scenario: Auxiliary records park in session state
    Given a Claude Code transcript "a.jsonl" holding "cost-state", "pr-link", and "custom-title" records
    When the Claude Code importer extracts the transcript
    Then the session carries the cost, the PR link, and the title

  Scenario: Partial trailing lines are held back, malformed lines skipped
    Given a Claude Code transcript "a.jsonl" with 2 records ending in a partial trailing line
    When the Claude Code importer extracts the transcript
    Then extraction yields 2 messages
    And 1 partial trailing line is held back
    When the file is completed with the rest of the record
    And the Claude Code importer extracts the transcript again
    Then extraction yields 1 more message