@wip
Feature: Optional session metadata
  Title, git coordinates, working directory, archive state, PR links,
  status, app version, and models are named optional Session fields so
  listing across agents can use them.
  Evidence: Claude custom-title/agent-name records, per-record gitBranch,
  pr-link records, version per record, message.model; Codex threads
  (title, git_branch, git_sha, git_origin_url, archived, is_pinned, model
  with mid-session changes via thread_settings_applied, cli_version);
  Cursor composerHeaders name, isArchived and branchMetadata.prUrl keys;
  Grok Bot cloud-agent records (branchName, prUrl, prState, status) and
  roster names; Antigravity titles and CASCADE_RUN_STATUS_*.

  Scenario: A session exposes its title, branch, and PR link
    Given a fixture session "m1" from agent "claude_code" in project "demo"
    When the session records title "Fix the release pipeline", git branch "claude/summarization-bug" and pr_url "https://github.com/AnthusAI/BotSpy/pull/7"
    Then the session summary includes title "Fix the release pipeline"
    And the session summary includes git branch "claude/summarization-bug"
    And the session summary includes pr_url "https://github.com/AnthusAI/BotSpy/pull/7"

  Scenario: Git coordinates include the commit when the agent records it
    Given a fixture session "m2" from agent "codex" in project "demo"
    When the session records git branch "main", git commit "a50a9d8" and origin "git@github.com:AnthusAI/BotSpy.git"
    Then the session summary includes git commit "a50a9d8"

  Scenario: The working directory is recorded
    Given a fixture session "m3" from agent "codex" in project "demo"
    When the session records cwd "/Users/home/Projects/BotSpy"
    Then the session summary includes cwd "/Users/home/Projects/BotSpy"

  Scenario: Archive and status flags are preserved
    Given a fixture session "m4" from agent "codex" in project "demo"
    When the session is archived and pinned
    Then the session summary includes archived true
    And the session summary includes status "archived"

  Scenario: App version and models are recorded
    Given a fixture session "m5" from agent "claude_code" in project "demo"
    When the session records app version "2.1.284" and model "claude-opus-5-5"
    Then the session summary includes app version "2.1.284"
    And the session summary includes model "claude-opus-5-5"

  Scenario: A model change mid-session is visible in the session's models
    Given a fixture session "m6" from agent "codex" in project "demo" that started on "gpt-5.6-terra"
    When the session changes its model mid-flight to "gpt-6.1-sol"
    Then the session summary includes models "gpt-5.6-terra, gpt-6.1-sol"

  Scenario: Metadata the agent does not record stays absent
    Given a fixture session "m7" from agent "cursor" in project "demo"
    Then the session summary has no title