Feature: Query filters
  The query interface filters sessions and messages by source, project,
  part kind, and time window. The engine does the filtering — callers
  never pull every record into memory to compare properties themselves.
  Evidence: BOTSPY-c7795b Local Store Initiative; the storage survey's
  1.4 GB Claude trees and 6 GB Cursor stores.

  Scenario: Sessions filter by source
    Given fixture sessions "f1" from "claude_code" and "f2" from "cursor" and "f3" from "claude_code"
    When I iterate the sessions filtered by source "claude_code"
    Then the iteration yields 2 sessions
    And the iteration yields sessions "f1" and "f3"

  Scenario: Sessions filter by project
    Given fixture sessions "f4" from "claude_code" in project "web" and "f5" from "cursor" in project "web" and "f6" from "codex" in project "cli"
    When I iterate the sessions filtered by project "web"
    Then the iteration yields 2 sessions
    And the iteration yields sessions "f4" and "f5"

  Scenario: Sessions filter by part kind
    Given fixture sessions "f7" and "f8" from "claude_code" in project "demo"
    And session "f7" has a message with a part of kind "tool_call"
    And session "f8" has a message with a part of kind "text"
    When I iterate the sessions filtered by part kind "tool_call"
    Then the iteration yields sessions "f7" only

  Scenario: Messages filter by part kind
    Given a fixture session "f9" from agent "claude_code" in project "demo"
    And the session has a message with parts "text", "thinking", "tool_call", "text"
    When I iterate the messages of session "f9" filtered by part kind "tool_call"
    Then the iteration yields 1 message
    And the iteration yields the kinds "tool_call"

  Scenario: Sessions filter by time window
    Given fixture sessions "f10" last active at "2026-09-30T18:00:00Z" and "f11" last active at "2026-10-01T09:00:00Z" and "f12" last active at "2026-10-01T12:00:00Z"
    When I iterate the sessions active after "2026-10-01T00:00:00Z" and before "2026-10-01T10:00:00Z"
    Then the iteration yields sessions "f11" only