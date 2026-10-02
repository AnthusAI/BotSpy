@wip
Feature: Cursor importer
  The Cursor importer watches two sources: the IDE's SQLite KV store
  (`state.vscdb` — `composerHeaders`, `composerData:<id>`,
  `bubbleId:<composerId>:<bubbleId>` values, content-addressed protobuf
  `agentKv:blob:<sha256>` blobs) and the CLI agent transcripts under
  `~/.cursor/projects/<encoded-cwd>/agent-transcripts/`. The store is a
  hot 6 GB WAL database: it is read through a snapshot copy, never in
  place, never written.
  Evidence: bubble keys are unordered; order comes from
  `composerData.fullConversationHeadersOnly`.

  Scenario: Composers are discovered from the headers table
    Given a Cursor KV store with 3 composers in "composerHeaders"
    When the importer detects changes
    Then discovery yields 3 sessions

  Scenario: Changed composers are re-detected by lastUpdatedAt
    Given a Cursor KV store with 3 composers in "composerHeaders"
    When the importer detects changes
    And the app updates composer "composer-2" with a new bubble and bumps "lastUpdatedAt"
    And the importer detects changes again
    Then only composer "composer-2" is reported as changed

  Scenario: Bubble order comes from the headers list, not the keys
    Given a Cursor composer "composer-1" whose bubbles have keys "b3", "b1", "b2"
    And whose "fullConversationHeadersOnly" lists "b1", "b2", "b3"
    When the importer extracts the composer
    Then the normalized messages appear in headers order "b1", "b2", "b3"

  Scenario: Bubbles become messages with thinking and tool calls
    Given a Cursor composer "composer-2" with a user bubble, an assistant bubble with thinking, and a "toolFormerData" tool call
    When the importer extracts the composer
    Then the normalized messages map the bubble roles, thinking, and tool call kinds

  Scenario: Blob-only tool results stay blob references
    Given a Cursor composer "composer-3" whose tool result lives only in "agentKv:blob:<sha256>"
    When the importer extracts the composer
    Then the tool result is a blob part with the blob hash and container "agentKv:blob"

  Scenario: The CLI transcripts are discovered per project directory
    Given a Cursor projects directory with CLI transcripts "a.jsonl" and "b.jsonl"
    When the importer detects changes
    Then discovery yields 2 sessions

  Scenario: The hot WAL store is read through a snapshot copy
    Given a Cursor KV store in WAL mode with a live app writing bubbles
    When the importer reads the store through a snapshot copy
    Then the source store and its WAL were never mutated
    And the live app kept writing without interference