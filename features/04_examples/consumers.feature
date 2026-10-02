@wip
Feature: Example consumers of the unified history
  The example consumers — a coding-session thanks-vs-F-bombs meter and a
  local sentiment analysis — read BotSpy sessions and score the
  conversation. They are examples, not core: they live in `examples/`
  (or an optional `botspy[metrics]` extra), never in the dependency
  graph, and only consume the public API.
  Evidence: BOTSPY-ab18a2 (example use cases epic).

  Scenario: The meter counts thanks and F-bombs per session
    Given a session with messages "thanks, that fixed it", "what the f*** is this", and "thank you, works now"
    When the meter scores the session
    Then it reports 2 thanks and 1 F-bomb
    And the ratio favors thanks

  Scenario: The meter aggregates across sessions
    Given sessions "m1" with 1 thanks and "m2" with 2 F-bombs
    When the meter scores the unified history
    Then it reports 1 thanks and 2 F-bombs across 2 sessions

  Scenario: Sentiment runs locally over message text
    Given a session with messages "I love this fix", "this is terrible", and "works fine"
    When the sentiment example scores the session
    Then it reports 1 positive, 1 negative, and 1 neutral message
    And no message text leaves the machine

  Scenario: Examples depend only on the public API
    Given the unified history with 2 fixture sessions
    When an example consumer is built against the published crate
    Then it uses only the public API surface
    And it adds no dependency to the core library