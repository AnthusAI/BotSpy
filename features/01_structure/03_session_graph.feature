@wip
Feature: Session graph — parents, roots, sub-agents, forks, and battles
  Sessions can be children, forks, or competitors of other sessions. The
  graph (parent_id, root_id, sub-agent kind/name, fork and battle links)
  is part of the normalized schema, so "open the sub-agent session for
  this Agent tool call" is representable.
  Evidence: Claude subagents/agent-<id>.jsonl keyed by parent sessionId;
  Cursor isSubagent composers with subagentTypeName and isBestOfNSubcomposer;
  Codex thread_spawn_edges and forked_from_id/forked_from_ordinal_exclusive;
  Antigravity parent_conversation_id, nesting_depth, battle_id and
  winning_conversation_id; Grok Bot cloudAgentPeerIds (peer ids only, no
  local transcript).

  Scenario: A sub-agent session links to its parent
    Given a fixture session "g1" from agent "claude_code" in project "demo"
    And a fixture sub-agent session "g1-sub1" of kind "general" with parent "g1"
    Then session "g1-sub1" has parent_id "g1"
    And session "g1-sub1" has root_id "g1"

  Scenario: A sub-agent session records its kind or agent name
    Given a fixture session "g2" from agent "cursor" in project "demo"
    And a fixture sub-agent session "g2-explore" of kind "explore" with parent "g2"
    Then session "g2-explore" records sub-agent kind "explore"

  Scenario: A forked session records the fork point
    Given a fixture session "g3" from agent "codex" in project "demo"
    When I fork session "g3" at ordinal 512 into "g3-fork"
    Then session "g3-fork" has parent_id "g3"
    And session "g3-fork" records fork ordinal 512

  Scenario: A spawned sub-agent thread records the spawn edge
    Given a fixture session "g4" from agent "codex" in project "demo"
    When I spawn session "g4-child" from session "g4"
    Then session "g4-child" has parent_id "g4"
    And session "g4-child" has root_id "g4"

  Scenario: A nested conversation records its nesting depth
    Given a fixture session "g5" from agent "antigravity" in project "demo"
    And a fixture sub-agent session "g5-sub" of kind "subagent" with parent "g5" at nesting depth 1
    Then session "g5-sub" has parent_id "g5"
    And session "g5-sub" records nesting depth 1

  Scenario: A best-of-N battle links its competitors and winner
    Given a fixture session "g6-a" from agent "antigravity" in project "demo" in battle "b1"
    And a fixture session "g6-b" from agent "antigravity" in project "demo" in battle "b1"
    When the battle "b1" is won by "g6-a"
    Then session "g6-a" records battle "b1" and winning conversation "g6-a"
    And session "g6-b" records battle "b1" without a win

  Scenario: A cloud agent with a partial peer id still links to its peer
    Given a fixture session "g7" from agent "grok_bot" in project "demo"
    When I record a peer cloud agent "bc-1234-abcd" with status "finished"
    Then session "g7" records peer id "bc-1234-abcd"
    And the peer session has no local transcript