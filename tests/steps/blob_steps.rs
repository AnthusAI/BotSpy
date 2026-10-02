//! Steps for the raw blob escape hatch and partial-cache provenance (spec
//! 12_raw_blob_escape_hatch).

use crate::steps::{current_session, last_message, BotSpyWorld};
use botspy::{KnownPart, Message, Part, PartialHistory, PartialReason, Role};
use cucumber::{given, then, when};

#[when(regex = r#"I record a tool result whose payload lives in protobuf blob "([^"]+)""#)]
fn record_blob_tool_result(world: &mut BotSpyWorld, blob_ref: String) {
    let mut segments = blob_ref.splitn(3, ':');
    let head = segments
        .next()
        .expect("blob reference must be <container>:<kind>:<hash>");
    let kind = segments
        .next()
        .expect("blob reference must be <container>:<kind>:<hash>");
    let blob_hash = segments
        .next()
        .expect("blob reference must be <container>:<kind>:<hash>")
        .to_string();
    let container = format!("{head}:{kind}");
    crate::steps::record_message_on_current(
        world,
        Message {
            role: Role::Tool,
            parts: vec![Part::Known(KnownPart::Blob {
                blob_hash,
                container,
                extra: None,
            })],
            ..Message::default()
        },
    );
}

#[when(regex = r#"I record a thinking part with encrypted content "([^"]+)""#)]
fn record_encrypted_thinking(world: &mut BotSpyWorld, encrypted: String) {
    crate::steps::record_message_on_current(
        world,
        Message {
            role: Role::Assistant,
            parts: vec![Part::Known(KnownPart::Thinking {
                text: None,
                signature: None,
                encrypted: Some(encrypted),
                extra: None,
            })],
            ..Message::default()
        },
    );
}

#[when(regex = r#"I record a thinking part with an empty text and signature "([^"]+)""#)]
fn record_signed_thinking(world: &mut BotSpyWorld, signature: String) {
    crate::steps::record_message_on_current(
        world,
        Message {
            role: Role::Assistant,
            parts: vec![Part::Known(KnownPart::Thinking {
                text: Some(String::new()),
                signature: Some(signature),
                encrypted: None,
                extra: None,
            })],
            ..Message::default()
        },
    );
}

#[given(
    regex = r#"a fixture session "([^"]+)" from agent "([^"]+)" in project "([^"]+)" cached from the cloud"#
)]
fn cloud_cached_session(world: &mut BotSpyWorld, id: String, agent: String, project: String) {
    let adapter = crate::steps::adapter_for(world, &agent);
    adapter.add_session(botspy::Session {
        id: id.clone(),
        agent: crate::steps::parse_agent(&agent),
        project_id: project,
        started_at: "2026-10-01T09:00:00Z".to_string(),
        last_activity_at: "2026-10-01T09:00:00Z".to_string(),
        partial: Some(PartialHistory {
            reason: PartialReason::CloudCache,
            detail: Some("cloud-cache".to_string()),
        }),
        ..botspy::Session::default()
    });
    world.current_session = Some(id);
}

#[given(
    regex = r#"a fixture session "([^"]+)" from agent "([^"]+)" in project "([^"]+)" capped at ([0-9]+) entries"#
)]
fn capped_session(world: &mut BotSpyWorld, id: String, agent: String, project: String, cap: u64) {
    let adapter = crate::steps::adapter_for(world, &agent);
    adapter.add_session(botspy::Session {
        id: id.clone(),
        agent: crate::steps::parse_agent(&agent),
        project_id: project,
        started_at: "2026-10-01T09:00:00Z".to_string(),
        last_activity_at: "2026-10-01T09:00:00Z".to_string(),
        partial: Some(PartialHistory {
            reason: PartialReason::LocalCap,
            detail: Some(format!("{cap} entries")),
        }),
        ..botspy::Session::default()
    });
    world.current_session = Some(id);
}

fn thinking_part(world: &mut BotSpyWorld) -> KnownPart {
    let message = last_message(world);
    message
        .parts
        .iter()
        .find_map(|part| match part {
            Part::Known(known @ KnownPart::Thinking { .. }) => Some(known.clone()),
            _ => None,
        })
        .expect("the last message has no thinking part")
}

#[then(regex = r#"part ([0-9]+) has blob_hash "([^"]+)""#)]
fn part_blob_hash(world: &mut BotSpyWorld, index: usize, blob_hash: String) {
    let part = &last_message(world).parts[index - 1];
    match part {
        Part::Known(KnownPart::Blob {
            blob_hash: actual, ..
        }) => assert_eq!(actual, &blob_hash),
        other => panic!("part {index} is not a blob: {other:?}"),
    }
}

#[then(regex = r#"part ([0-9]+) has container "([^"]+)""#)]
fn part_container(world: &mut BotSpyWorld, index: usize, container: String) {
    let part = &last_message(world).parts[index - 1];
    match part {
        Part::Known(KnownPart::Blob {
            container: actual, ..
        }) => assert_eq!(actual, &container),
        other => panic!("part {index} is not a blob: {other:?}"),
    }
}

#[then(regex = r#"the last message has a thinking part with no plain text"#)]
fn thinking_no_plain_text(world: &mut BotSpyWorld) {
    match thinking_part(world) {
        KnownPart::Thinking { text: None, .. } => {}
        other => panic!("thinking part carries plain text: {other:?}"),
    }
}

#[then(regex = r#"the thinking part preserves its encrypted blob"#)]
fn thinking_preserves_encrypted(world: &mut BotSpyWorld) {
    match thinking_part(world) {
        KnownPart::Thinking {
            encrypted: Some(_), ..
        } => {}
        other => panic!("thinking part loses its encrypted blob: {other:?}"),
    }
}

#[then(regex = r#"the thinking part preserves its signature blob"#)]
fn thinking_preserves_signature(world: &mut BotSpyWorld) {
    match thinking_part(world) {
        KnownPart::Thinking {
            signature: Some(_), ..
        } => {}
        other => panic!("thinking part loses its signature blob: {other:?}"),
    }
}

#[then(regex = r#"the session is flagged as a partial cloud cache"#)]
fn session_partial_cloud_cache(world: &mut BotSpyWorld) {
    let session = current_session(world);
    match &session.partial {
        Some(PartialHistory {
            reason: PartialReason::CloudCache,
            ..
        }) => {}
        other => panic!("session is not flagged as a partial cloud cache: {other:?}"),
    }
}

#[then(regex = r#"the session is flagged as a partial local view"#)]
fn session_partial_local_view(world: &mut BotSpyWorld) {
    let session = current_session(world);
    match &session.partial {
        Some(PartialHistory {
            reason: PartialReason::LocalCap,
            ..
        }) => {}
        other => panic!("session is not flagged as a partial local view: {other:?}"),
    }
}

#[then(regex = r#"listing the session marks its provenance as partial"#)]
fn listing_marks_partial(world: &mut BotSpyWorld) {
    let id = current_session(world).id;
    let summary = world
        .store
        .list_sessions()
        .into_iter()
        .find(|s| s.id == id)
        .unwrap_or_else(|| panic!("session {id} not listed"));
    assert!(
        summary.partial.is_some(),
        "listing does not mark session {id} as partial"
    );
}
