Feature: Adapter contract
  Every source adapter obeys the same contract: discovery finds the
  sessions it can tap, extraction streams records without loading whole
  files, malformed input is skipped and counted (never fatal), and the
  source itself is only ever read — adapters never write to the
  directories or databases they watch.
  Evidence: transcripts exceed 100 MB; malformed lines and unknown
  record types appear in real app data.

  Scenario: Discovery finds sessions under the given roots
    Given a fixture source root "roots/demo/claude" with transcripts "a.jsonl" and "b.jsonl"
    When I discover sessions from the source "claude_code" at that root
    Then discovery yields 2 sessions

  Scenario: Discovery is incremental by identity, not by content
    Given a fixture source root "roots/demo/claude" with transcripts "a.jsonl" and "b.jsonl"
    When I discover sessions from the source "claude_code" at that root
    And I discover sessions from the source "claude_code" at that root again
    Then the second discovery reports no new sessions

  Scenario: Extraction streams records without loading whole files
    Given a fixture source root "roots/demo/claude" with a transcript of 10000 records
    When I extract the records of that transcript
    Then the records stream one at a time
    And the extractor never holds more than 100 records in memory

  Scenario: Malformed records are skipped and counted
    Given a fixture source root "roots/demo/claude" with a transcript holding 3 good records and 2 malformed ones
    When I extract the records of that transcript
    Then extraction yields 3 good records
    And extraction reports 2 skipped records

  Scenario: Unknown records are skipped and counted
    Given a fixture source root "roots/demo/claude" with a transcript holding 2 known records and 1 of an unknown type
    When I extract the records of that transcript
    Then extraction yields 2 good records
    And extraction reports 1 skipped record

  Scenario: Extraction never writes to the source
    Given a fixture source root "roots/demo/claude" with transcripts "a.jsonl" and "b.jsonl"
    When I extract every record from the source "claude_code" at that root
    Then no file under the source root changed