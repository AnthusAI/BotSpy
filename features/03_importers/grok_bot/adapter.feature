Feature: Grok Bot importer
  The Grok Bot importer watches the plain-JSON `sand-client-persistence`
  blobs — chat entry logs keyed by opaque hash names, an agent roster, and
  cloud-agent records — plus `~/.grokbot/` settings, and emits normalized
  records as they appear.
  Evidence: entry logs are capped at 200 entries locally with the full
  history server-side, so sessions are partial local views by design;
  entry-log records carry `seq`; voice-call entries are opaque.

  Scenario: Entry logs are discovered from the persistence directory
    Given a Grok Bot persistence directory with 2 entry-log blobs
    When the Grok Bot importer detects changes
    Then discovery yields 2 sessions

  Scenario: Blob names map to agents via the roster
    Given a Grok Bot roster pointing blob "abc123" at agent "rocket"
    When the Grok Bot importer detects changes
    Then the session from blob "abc123" is attributed to agent "rocket"

  Scenario: Messages appear in seq order
    Given a Grok Bot entry log with entries "seq 3", "seq 1", and "seq 2"
    When the Grok Bot importer extracts the entry log
    Then the normalized messages appear in seq order 1, 2, 3

  Scenario: Personas carry an author, voice calls stay raw
    Given a Grok Bot entry log holding a persona message with "author" and a voice-call entry
    When the Grok Bot importer extracts the entry log
    Then the persona message records its author
    And the voice-call entry is preserved as a raw part

  Scenario: Cloud-agent records become session stubs
    Given a Grok Bot cloud-agent record with status "running" and a PR link
    When the Grok Bot importer detects changes
    Then discovery yields a session stub carrying the status and PR metadata

  Scenario: Capped entry logs are flagged as partial local views
    Given a Grok Bot entry log capped at 200 entries with a server-side continuation
    When the Grok Bot importer extracts the entry log
    Then the extracted session is flagged as a partial local view