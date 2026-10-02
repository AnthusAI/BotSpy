@wip
Feature: Raw blob escape hatch and partial-cache provenance
  Content an adapter cannot or should not decode — protobuf blobs,
  encrypted reasoning, signed payloads — is preserved as a blob reference
  (blob_hash plus container) instead of being dropped. Sessions that are
  only partial local caches of server history are flagged.
  Evidence: Cursor agentKv:blob:<sha256> protobuf blobs (tool results and
  full prompts, content-hash keyed, no public schema); Codex reasoning
  encrypted_content; Claude thinking signature blocks; Cursor
  conversation-search.db source "cloud-cache"; Grok Bot entry logs capped
  at 200 entries with full history server-side.

  Scenario: A protobuf blob is preserved as a blob reference
    Given a fixture session "b1" from agent "cursor" in project "demo"
    When I record a tool result whose payload lives in protobuf blob "agentKv:blob:9d2c..."
    Then the last message has 1 part with kinds "blob"
    And part 1 has blob_hash "9d2c..."
    And part 1 has container "agentKv:blob"

  Scenario: Encrypted reasoning is preserved verbatim
    Given a fixture session "b2" from agent "codex" in project "demo"
    When I record a thinking part with encrypted content "opaque"
    Then the last message has a thinking part with no plain text
    And the thinking part preserves its encrypted blob

  Scenario: A signed reasoning block keeps its signature
    Given a fixture session "b3" from agent "claude_code" in project "demo"
    When I record a thinking part with an empty text and signature "sig-opaque"
    Then the thinking part preserves its signature blob

  Scenario: A cloud-cached session is flagged as partial
    Given a fixture session "b4" from agent "cursor" in project "demo" cached from the cloud
    Then the session is flagged as a partial cloud cache
    And listing the session marks its provenance as partial

  Scenario: A server-truncated log is flagged as partial
    Given a fixture session "b5" from agent "grok_bot" in project "demo" capped at 200 entries
    Then the session is flagged as a partial local view