Feature: Inline data and content-addressed attachments
  Images and other inline bytes are a part kind of their own, identified
  by a content hash; file-backed attachments keep path, mime, and size.
  Evidence: Claude Code inline base64 PNGs duplicated across two fields
  per record; Cursor bubble images and screenshots inside protobuf blobs
  (content-addressed by sha256); Codex attachments dir; Antigravity
  .user_uploaded and brain scratch files.

  Scenario: An inline image is a part with its media type and bytes
    Given a fixture session "i1" from agent "claude_code" in project "demo"
    When I record an inline image part with media type "image/png" and base64 data "iVBORw0KGgo..."
    Then the last message has 1 part with kinds "inline_data"
    And part 1 has mime "image/png"
    And part 1 has inline data

  Scenario: Inline data is identified by its content hash
    Given a fixture session "i2" from agent "claude_code" in project "demo"
    When I record the same inline image twice in one message
    Then both parts have the same content hash
    And each part has data_ref equal to that hash

  Scenario: A blob-backed image is referenced by its content hash
    Given a fixture session "i3" from agent "cursor" in project "demo"
    When I record an image part whose data lives in blob "agentKv:blob:3f1a..."
    Then part 1 has data_ref "agentKv:blob:3f1a..."
    And part 1 has no inline data

  Scenario: File-backed attachments keep path, mime, and size
    Given a fixture session "i4" from agent "codex" in project "demo"
    When I record an attachment part with path "attachments/pasted-text.txt", mime "text/plain" and size 512
    Then the last message has 1 part with kinds "attachment"