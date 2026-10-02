Feature: Role mapping conventions for native message shades
  Native roles beyond user/assistant/system/tool map by convention:
  developer messages become System with a developer origin, inter-agent
  mail keeps author and recipient, injected content is split out of user
  text, and simulated user messages are marked as injected.
  Evidence: Codex role developer; Codex agent_message (author, recipient);
  Antigravity step 101 inter-agent messages with senders; Grok Bot
  multi-agent rooms (fromAgent/toAgent, author personas); Claude Code
  <system-reminder> blocks inside user records and attachment.rendered
  environment snapshots; Cursor isSimulatedMsg + simulatedMsgReason.

  Scenario: A developer message maps to System with a developer origin
    Given a fixture session "r1" from agent "codex" in project "demo"
    When I record a developer message with text "Always answer in Rust."
    Then the last message has role "system"
    And the last message has origin "developer"

  Scenario: Inter-agent mail keeps its author and recipient
    Given a fixture session "r2" from agent "codex" in project "demo"
    When I record an inter-agent message from "/root/rust_wip_capacity" to "/root" with text "build is green"
    Then the last message has role "assistant"
    And the last message has origin "agent_message"
    And the last message has author "/root/rust_wip_capacity"
    And the last message has recipient "/root"

  Scenario: A multi-agent room keeps the persona that spoke
    Given a fixture session "r3" from agent "grok_bot" in project "demo"
    When I record an assistant message authored by "Charlie Munger"
    Then the last message has role "assistant"
    And the last message has author "Charlie Munger"

  Scenario: Injected system content is split out of user text
    Given a fixture session "r4" from agent "claude_code" in project "demo"
    When I record a user message whose text contains a "<system-reminder>" block
    Then the last message has a text part with the user's own words
    And the last message has a system part with the injected content

  Scenario: A simulated user message is marked as injected
    Given a fixture session "r5" from agent "cursor" in project "demo"
    When I record a simulated user message with reason "plan-execution"
    Then the last message has role "user"
    And the last message has origin "simulated"