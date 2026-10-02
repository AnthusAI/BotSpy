//! Steps for the Grok Bot importer (spec 03_importers/grok_bot/adapter.feature).

use crate::steps::BotSpyWorld;
use botspy::adapters::grok_bot::{GrokBotSource, GrokDiscovery};
use botspy::schema::{PartialReason, SessionMetadata};
use cucumber::{given, then, when};
use serde_json::{json, Value};
use std::path::PathBuf;
use std::sync::atomic::{AtomicUsize, Ordering};

static UNIQUE_DIR: AtomicUsize = AtomicUsize::new(0);

fn new_root(world: &mut BotSpyWorld) -> PathBuf {
    let base = std::env::temp_dir().join("botspy-gb").join(format!(
        "{}-{}",
        std::process::id(),
        UNIQUE_DIR.fetch_add(1, Ordering::SeqCst)
    ));
    std::fs::create_dir_all(&base).expect("create Grok Bot persistence dir");
    world.gb_root = Some(base.clone());
    world.gb_source = None;
    world.gb_blob = None;
    world.gb_discovery = None;
    world.gb_extraction = None;
    base
}

fn write_blob(root: &std::path::Path, name: &str, value: &Value) {
    std::fs::write(root.join(format!("{name}.json")), value.to_string())
        .expect("write Grok Bot blob");
}

fn message_entry(seq: u64, role: &str, text: &str, author: Option<&str>) -> Value {
    let mut entry = json!({"seq": seq, "type": "message", "role": role, "text": text});
    if let Some(author) = author {
        entry["author"] = json!(author);
    }
    entry
}

fn finish_setup(world: &mut BotSpyWorld, blob: String) {
    let root = world.gb_root.clone().expect("no Grok Bot persistence dir");
    world.gb_blob = Some(blob);
    world.gb_source = Some(GrokBotSource::new(root));
}

#[given(regex = r#"^a Grok Bot persistence directory with ([0-9]+) entry-log blobs$"#)]
fn persistence_dir_with_blobs(world: &mut BotSpyWorld, count: usize) {
    let root = new_root(world);
    for i in 1..=count {
        write_blob(
            &root,
            &format!("blob-hash-{i}"),
            &json!({"entries": [message_entry(1, "user", &format!("hello from blob {i}"), None)]}),
        );
    }
    world.gb_source = Some(GrokBotSource::new(root));
}

#[given(regex = r#"^a Grok Bot roster pointing blob "([^"]+)" at agent "([^"]+)"$"#)]
fn roster_points_blob_at_agent(world: &mut BotSpyWorld, blob: String, agent: String) {
    let root = new_root(world);
    write_blob(
        &root,
        &blob,
        &json!({"entries": [message_entry(1, "assistant", "persona hello", Some(agent.as_str()))]}),
    );
    std::fs::write(
        root.join("roster.json"),
        json!({blob.clone(): agent}).to_string(),
    )
    .expect("write roster");
    world.gb_source = Some(GrokBotSource::new(root));
}

#[given(
    regex = r#"^a Grok Bot entry log with entries "seq ([0-9]+)", "seq ([0-9]+)", and "seq ([0-9]+)"$"#
)]
fn entry_log_with_shuffled_seqs(world: &mut BotSpyWorld, first: u64, second: u64, third: u64) {
    let root = new_root(world);
    write_blob(
        &root,
        "blob-hash-1",
        &json!({"entries": [
            message_entry(first, "user", &format!("entry seq {first}"), None),
            message_entry(second, "assistant", &format!("entry seq {second}"), None),
            message_entry(third, "user", &format!("entry seq {third}"), None)
        ]}),
    );
    finish_setup(world, "blob-hash-1".to_string());
}

#[given(
    regex = r#"^a Grok Bot entry log holding a persona message with "([^"]+)" and a voice-call entry$"#
)]
fn entry_log_with_persona_and_voice_call(world: &mut BotSpyWorld, field: String) {
    assert_eq!(field, "author", "unexpected persona field");
    let root = new_root(world);
    write_blob(
        &root,
        "blob-hash-1",
        &json!({"entries": [
            message_entry(1, "assistant", "persona hello", Some("rocket")),
            json!({"seq": 2, "type": "voice_call", "callId": "vc-1", "payload": "opaque-bytes"})
        ]}),
    );
    finish_setup(world, "blob-hash-1".to_string());
}

#[given(regex = r#"^a Grok Bot cloud-agent record with status "([^"]+)" and a PR link$"#)]
fn cloud_agent_record(world: &mut BotSpyWorld, status: String) {
    let root = new_root(world);
    std::fs::write(
        root.join("cloud-agents.json"),
        json!([{
            "agentId": "peer-1",
            "status": status,
            "prUrl": "https://github.com/x/y/pull/9"
        }])
        .to_string(),
    )
    .expect("write cloud-agent records");
    world.gb_source = Some(GrokBotSource::new(root));
}

#[given(
    regex = r#"^a Grok Bot entry log capped at ([0-9]+) entries with a server-side continuation$"#
)]
fn capped_entry_log(world: &mut BotSpyWorld, count: u64) {
    let root = new_root(world);
    let entries: Vec<Value> = (1..=count)
        .map(|seq| message_entry(seq, "user", &format!("entry seq {seq}"), None))
        .collect();
    write_blob(
        &root,
        "blob-hash-1",
        &json!({"capped": true, "continuedServerSide": true, "entries": entries}),
    );
    finish_setup(world, "blob-hash-1".to_string());
}

#[when(regex = r#"^the Grok Bot importer detects changes$"#)]
fn detect_changes(world: &mut BotSpyWorld) {
    let source = world.gb_source.as_ref().expect("no Grok Bot source");
    let (summaries, discovery) = source.discover();
    world.discovered = summaries;
    world.gb_discovery = Some(discovery);
}

#[when(regex = r#"^the Grok Bot importer extracts the entry log$"#)]
fn extract_entry_log(world: &mut BotSpyWorld) {
    let source = world.gb_source.as_ref().expect("no Grok Bot source");
    let blob = world.gb_blob.as_deref().expect("no Grok Bot blob");
    let extraction = source
        .extract(blob)
        .expect("no entry log for the blob name");
    world.gb_extraction = Some(extraction);
}

#[then(regex = r#"^the session from blob "([^"]+)" is attributed to agent "([^"]+)"$"#)]
fn blob_attributed_to_agent(world: &mut BotSpyWorld, blob: String, agent: String) {
    let discovery: &GrokDiscovery = world
        .gb_discovery
        .as_ref()
        .expect("no Grok Bot discovery result");
    assert_eq!(
        discovery.attributions.get(&blob).map(String::as_str),
        Some(agent.as_str()),
        "roster attribution {blob} -> {agent} missing"
    );
}

#[then(regex = r#"^the normalized messages appear in seq order ([0-9]+(?:, [0-9]+)*)$"#)]
fn messages_in_seq_order(world: &mut BotSpyWorld, expected: String) {
    let expected_seqs: Vec<u64> = expected
        .split(", ")
        .map(|part| part.parse().expect("seq number"))
        .collect();
    let extraction = world
        .gb_extraction
        .as_ref()
        .expect("no Grok Bot extraction");
    let seqs: Vec<u64> = extraction
        .session
        .messages
        .iter()
        .map(|message| {
            message
                .provenance
                .as_ref()
                .and_then(|prov| prov.ordinal)
                .expect("entry provenance seq")
        })
        .collect();
    assert_eq!(
        seqs, expected_seqs,
        "messages did not follow the explicit seq order"
    );
}

#[then(regex = r#"^the persona message records its author$"#)]
fn persona_message_records_author(world: &mut BotSpyWorld) {
    let extraction = world
        .gb_extraction
        .as_ref()
        .expect("no Grok Bot extraction");
    let author = extraction
        .session
        .messages
        .iter()
        .find_map(|message| message.author.clone())
        .expect("no persona author recorded");
    assert_eq!(author, "rocket");
}

#[then(regex = r#"^the voice-call entry is preserved as a raw part$"#)]
fn voice_call_preserved_raw(world: &mut BotSpyWorld) {
    let extraction = world
        .gb_extraction
        .as_ref()
        .expect("no Grok Bot extraction");
    let raw = extraction
        .session
        .messages
        .iter()
        .find_map(|message| {
            message.parts.iter().find_map(|part| match part {
                botspy::Part::Extra(value) => Some(value.clone()),
                _ => None,
            })
        })
        .expect("no raw voice-call part preserved");
    assert_eq!(
        raw.get("type").and_then(Value::as_str),
        Some("voice_call"),
        "the voice-call entry was not preserved verbatim"
    );
    assert_eq!(raw.get("callId").and_then(Value::as_str), Some("vc-1"));
}

#[then(regex = r#"^discovery yields a session stub carrying the status and PR metadata$"#)]
fn discovery_yields_cloud_stub(world: &mut BotSpyWorld) {
    let discovery = world.gb_discovery.as_ref().expect("no Grok Bot discovery");
    assert_eq!(discovery.cloud_agents, 1);
    assert_eq!(discovery.cloud_agent_ids, vec!["peer-1".to_string()]);
    let stub = world
        .discovered
        .iter()
        .find(|summary| summary.id == "peer-1")
        .expect("no cloud-agent session stub");
    let expected: SessionMetadata = SessionMetadata {
        status: Some("running".to_string()),
        pr_url: Some("https://github.com/x/y/pull/9".to_string()),
        ..SessionMetadata::default()
    };
    assert_eq!(stub.metadata.status, expected.status);
    assert_eq!(stub.metadata.pr_url, expected.pr_url);
}

#[then(regex = r#"^the extracted session is flagged as a partial local view$"#)]
fn session_flagged_partial(world: &mut BotSpyWorld) {
    let extraction = world
        .gb_extraction
        .as_ref()
        .expect("no Grok Bot extraction");
    let partial = extraction
        .session
        .partial
        .as_ref()
        .expect("the capped entry log did not flag the session as partial");
    assert_eq!(partial.reason, PartialReason::LocalCap);
}
