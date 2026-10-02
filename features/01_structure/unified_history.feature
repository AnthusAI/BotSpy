Feature: Unified history across all five agents
  Sessions from all five adapters pile into one unified agent-session
  history: consistent ordering, dedup, and provenance survive the merge.
  Evidence: BOTSPY-e4765d (synthetic fixtures for all five agents) —
  Claude Code JSONL, Cursor KV + CLI transcripts, Codex rollouts, Grok
  Bot entry logs, Antigravity conversations.

  Scenario: Sessions from all five adapters list together
    Given fixture sessions "u1" from "claude_code", "u2" from "cursor", "u3" from "codex", "u4" from "grok_bot", and "u5" from "antigravity"
    When the unified history is listed
    Then it yields 5 sessions
    And it yields sessions "u1", "u2", "u3", "u4", and "u5"

  Scenario: Ordering is by last activity across agents
    Given fixture sessions "u6" last active at "2026-09-30T18:00:00Z", "u7" last active at "2026-10-01T12:00:00Z", and "u8" last active at "2026-10-01T09:00:00Z" from "codex"
    When the unified history is listed
    Then the order is "u7", "u8", "u6"

  Scenario: The same session seen twice is deduplicated
    Given fixture session "u9" from "cursor" reported by two discovery passes
    When the unified history is listed
    Then it yields session "u9" once

  Scenario: Provenance survives the merge
    Given fixture session "u10" from "codex" whose message has provenance "rollout.jsonl" line 42
    When the unified history is opened for session "u10"
    Then that message's provenance still names "rollout.jsonl" line 42

  Scenario: Each session keeps its own agent and project
    Given fixture sessions "u11" from "claude_code" in project "web" and "u12" from "antigravity" in project "cli"
    When the unified history is listed
    Then session "u11" carries agent "claude_code" and project "web"
    And session "u12" carries agent "antigravity" and project "cli"