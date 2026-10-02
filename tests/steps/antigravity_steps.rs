//! Steps for the Antigravity importer (spec 03_importers/antigravity/adapter.feature).

use crate::steps::BotSpyWorld;
use botspy::adapters::antigravity::AntigravitySource;
use botspy::snapshot::digest_files;
use cucumber::{given, then, when};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicUsize, Ordering};

static UNIQUE_DIR: AtomicUsize = AtomicUsize::new(0);

const CONVERSATION: &str = "conv-1";

fn new_root(world: &mut BotSpyWorld) -> PathBuf {
    let base = std::env::temp_dir().join("botspy-ag").join(format!(
        "{}-{}",
        std::process::id(),
        UNIQUE_DIR.fetch_add(1, Ordering::SeqCst)
    ));
    std::fs::create_dir_all(&base).expect("create Antigravity data root");
    world.ag_root = Some(base.clone());
    world.ag_source = None;
    world.ag_conversation = None;
    world.ag_discovery = None;
    world.ag_extraction = None;
    world.ag_writer = None;
    base
}

fn open_store(path: &Path) -> rusqlite::Connection {
    let conn = rusqlite::Connection::open(path).expect("open Antigravity db");
    conn.pragma_update(None, "journal_mode", "WAL")
        .expect("enable WAL mode");
    conn
}

fn write_summaries_index(root: &Path, ids: &[(&str, &str)]) {
    let conn = open_store(&root.join("conversation_summaries.db"));
    conn.execute(
        "CREATE TABLE IF NOT EXISTS conversations (id TEXT PRIMARY KEY, name TEXT, updated_at TEXT)",
        [],
    )
    .expect("create conversations table");
    for (id, name) in ids {
        conn.execute(
            "INSERT OR REPLACE INTO conversations (id, name, updated_at) VALUES (?1, ?2, ?3)",
            rusqlite::params![id, name, format!("2026-10-01T09:0{count}:00Z", count = 1)],
        )
        .expect("insert conversation row");
    }
}

fn write_transcript(root: &Path, conversation_id: &str, records: &[String]) {
    let path = root
        .join("brain")
        .join(conversation_id)
        .join(".system_generated")
        .join("logs")
        .join("transcript.jsonl");
    std::fs::create_dir_all(path.parent().expect("transcript parent dir"))
        .expect("create transcript dirs");
    std::fs::write(
        path,
        records
            .iter()
            .map(|line| format!("{line}\n"))
            .collect::<String>(),
    )
    .expect("write transcript.jsonl");
}

fn write_payload_db(root: &Path, conversation_id: &str, links: &[(u64, &str, &str)]) {
    let db = root.join("brain").join(conversation_id).join("payload.db");
    let conn = open_store(&db);
    conn.execute(
        "CREATE TABLE IF NOT EXISTS tool_links (step_index INTEGER PRIMARY KEY, call_id TEXT NOT NULL, result TEXT)",
        [],
    )
    .expect("create tool_links table");
    for (step_index, call_id, result) in links {
        conn.execute(
            "INSERT OR REPLACE INTO tool_links (step_index, call_id, result) VALUES (?1, ?2, ?3)",
            rusqlite::params![*step_index as i64, call_id, result],
        )
        .expect("insert tool link");
    }
}

fn user_input(step_index: u64, content: &str) -> String {
    format!(
        r#"{{"type":"USER_INPUT","step_index":{step_index},"created_at":"2026-10-01T09:00:00Z","content":"{content}"}}"#
    )
}

fn planner_response(step_index: u64, content: &str) -> String {
    format!(
        r#"{{"type":"PLANNER_RESPONSE","step_index":{step_index},"created_at":"2026-10-01T09:00:05Z","content":"{content}"}}"#
    )
}

fn finish_setup(world: &mut BotSpyWorld) {
    let root = world.ag_root.clone().expect("no Antigravity data root");
    world.ag_conversation = Some(CONVERSATION.to_string());
    world.ag_source = Some(AntigravitySource::new(root));
}

#[given(regex = r#"^an Antigravity summaries index with ([0-9]+) conversations$"#)]
fn summaries_index(world: &mut BotSpyWorld, count: usize) {
    let root = new_root(world);
    let ids: Vec<(String, String)> = (1..=count)
        .map(|i| (format!("conv-{i}"), format!("demo-{i}")))
        .collect();
    let refs: Vec<(&str, &str)> = ids
        .iter()
        .map(|(id, name)| (id.as_str(), name.as_str()))
        .collect();
    write_summaries_index(&root, &refs);
    world.ag_source = Some(AntigravitySource::new(root));
}

#[given(
    regex = r#"^an Antigravity conversation with both a "transcript.jsonl" and a SQLite payload DB$"#
)]
fn conversation_with_transcript_and_db(world: &mut BotSpyWorld) {
    let root = new_root(world);
    write_summaries_index(&root, &[(CONVERSATION, "demo")]);
    write_transcript(
        &root,
        CONVERSATION,
        &[
            user_input(1, "run the tests"),
            planner_response(2, "On it."),
        ],
    );
    write_payload_db(&root, CONVERSATION, &[(2, "call_2", "ok")]);
    finish_setup(world);
}

#[given(
    regex = r#"^an Antigravity transcript holding "USER_INPUT", "PLANNER_RESPONSE", "CHECKPOINT", and "ERROR_MESSAGE" records$"#
)]
fn transcript_with_all_types(world: &mut BotSpyWorld) {
    let root = new_root(world);
    write_summaries_index(&root, &[(CONVERSATION, "demo")]);
    write_transcript(
        &root,
        CONVERSATION,
        &[
            user_input(1, "run the tests"),
            planner_response(2, "On it."),
            r#"{"type":"CHECKPOINT","step_index":3,"created_at":"2026-10-01T09:00:10Z","summary":"checkpoint state"}"#.to_string(),
            r#"{"type":"ERROR_MESSAGE","step_index":4,"created_at":"2026-10-01T09:00:15Z","content":"boom"}"#.to_string(),
        ],
    );
    finish_setup(world);
}

#[given(
    regex = r#"^an Antigravity transcript with steps "step_index ([0-9]+)", "step_index ([0-9]+)", and "step_index ([0-9]+)"$"#
)]
fn transcript_with_shuffled_steps(world: &mut BotSpyWorld, first: u64, second: u64, third: u64) {
    let root = new_root(world);
    write_summaries_index(&root, &[(CONVERSATION, "demo")]);
    write_transcript(
        &root,
        CONVERSATION,
        &[
            user_input(first, &format!("step {first}")),
            user_input(second, &format!("step {second}")),
            user_input(third, &format!("step {third}")),
        ],
    );
    finish_setup(world);
}

#[given(regex = r#"^an Antigravity conversation whose transcript lacks tool-call ids$"#)]
fn conversation_with_idless_tool_call(world: &mut BotSpyWorld) {
    let root = new_root(world);
    write_summaries_index(&root, &[(CONVERSATION, "demo")]);
    write_transcript(
        &root,
        CONVERSATION,
        &[
            user_input(1, "run the tests"),
            r#"{"type":"TOOL_CALL","step_index":2,"created_at":"2026-10-01T09:00:05Z","toolName":"run_terminal","arguments":"{\"cmd\":[\"cargo\",\"test\"]}"}"#.to_string(),
        ],
    );
    finish_setup(world);
}

#[given(regex = r#"^whose payload DB carries the tool call and its result$"#)]
fn payload_db_carries_linkage(world: &mut BotSpyWorld) {
    let root = world.ag_root.clone().expect("no Antigravity data root");
    write_payload_db(&root, CONVERSATION, &[(2, "call_5", "all tests pass")]);
}

#[given(regex = r#"^an Antigravity conversation DB in WAL mode with a live app writing$"#)]
fn conversation_db_with_live_app(world: &mut BotSpyWorld) {
    let root = new_root(world);
    write_summaries_index(&root, &[(CONVERSATION, "demo")]);
    write_transcript(&root, CONVERSATION, &[user_input(1, "hello")]);
    let db = root.join("brain").join(CONVERSATION).join("payload.db");
    let writer = open_store(&db);
    writer
        .execute(
            "CREATE TABLE IF NOT EXISTS tool_links (step_index INTEGER PRIMARY KEY, call_id TEXT NOT NULL, result TEXT)",
            [],
        )
        .expect("create tool_links table");
    writer
        .execute(
            "INSERT OR REPLACE INTO tool_links (step_index, call_id, result) VALUES (1, 'call_1', 'ok')",
            [],
        )
        .expect("insert tool link");
    world.ag_writer = Some(writer);
    finish_setup(world);
}

#[given(
    regex = r#"^an Antigravity transcript holding ([0-9]+) good records and ([0-9]+) malformed ones$"#
)]
fn transcript_with_malformed(world: &mut BotSpyWorld, good: usize, malformed: usize) {
    let root = new_root(world);
    write_summaries_index(&root, &[(CONVERSATION, "demo")]);
    let mut records = Vec::new();
    for i in 0..good {
        records.push(user_input(i as u64 + 1, &format!("good {i}")));
        if i < malformed {
            records.push("this line is { not json".to_string());
        }
    }
    write_transcript(&root, CONVERSATION, &records);
    finish_setup(world);
}

#[when(regex = r#"^the Antigravity importer detects changes$"#)]
fn detect_changes(world: &mut BotSpyWorld) {
    let source = world.ag_source.as_ref().expect("no Antigravity source");
    let (summaries, discovery) = source.discover();
    world.discovered = summaries;
    world.ag_discovery = Some(discovery);
}

#[when(regex = r#"^the Antigravity importer extracts the (conversation|transcript)$"#)]
fn extract_conversation(world: &mut BotSpyWorld, _kind: String) {
    let source = world.ag_source.as_ref().expect("no Antigravity source");
    let conversation = world
        .ag_conversation
        .as_deref()
        .expect("no Antigravity conversation");
    let extraction = source
        .extract(conversation)
        .expect("no transcript for the conversation");
    world.skipped = extraction.skipped.clone();
    world.ag_extraction = Some(extraction);
}

#[when(regex = r#"^the importer reads the DB through a snapshot copy$"#)]
fn read_through_snapshot(world: &mut BotSpyWorld) {
    let root = world.ag_root.clone().expect("no Antigravity data root");
    let db = root.join("brain").join(CONVERSATION).join("payload.db");
    let wal = wal_path(&db);
    world.ag_digest_before = Some(digest_files(&[&db, &wal]));
    let source = world.ag_source.as_ref().expect("no Antigravity source");
    let extraction = source
        .extract(CONVERSATION)
        .expect("no transcript for the conversation");
    world.ag_extraction = Some(extraction);
    world.ag_digest_after = Some(digest_files(&[&db, &wal]));
}

fn wal_path(db: &Path) -> PathBuf {
    let mut wal = db.as_os_str().to_owned();
    wal.push("-wal");
    PathBuf::from(wal)
}

#[then(regex = r#"^the messages come from the transcript records$"#)]
fn messages_come_from_transcript(world: &mut BotSpyWorld) {
    let extraction = world
        .ag_extraction
        .as_ref()
        .expect("no Antigravity extraction");
    assert!(!extraction.session.messages.is_empty());
    for message in &extraction.session.messages {
        let source_file = message
            .provenance
            .as_ref()
            .map(|prov| prov.source_file.as_str())
            .unwrap_or_default();
        assert!(
            source_file.contains("transcript.jsonl"),
            "message did not come from the transcript: {source_file}"
        );
    }
}

#[then(regex = r#"^"USER_INPUT" and "PLANNER_RESPONSE" become user and assistant messages$"#)]
fn user_and_assistant_roles(world: &mut BotSpyWorld) {
    let extraction = world
        .ag_extraction
        .as_ref()
        .expect("no Antigravity extraction");
    let messages = &extraction.session.messages;
    assert_eq!(messages[0].role, botspy::Role::User);
    assert_eq!(messages[1].role, botspy::Role::Assistant);
}

#[then(regex = r#"^"CHECKPOINT" becomes a compaction summary$"#)]
fn checkpoint_becomes_compaction_summary(world: &mut BotSpyWorld) {
    let extraction = world
        .ag_extraction
        .as_ref()
        .expect("no Antigravity extraction");
    let checkpoint = extraction
        .session
        .messages
        .iter()
        .find(|message| {
            message
                .provenance
                .as_ref()
                .and_then(|prov| prov.record_type.as_deref())
                == Some("CHECKPOINT")
        })
        .expect("no CHECKPOINT message");
    assert!(checkpoint.is_compaction_summary);
}

#[then(regex = r#"^"ERROR_MESSAGE" becomes an error message$"#)]
fn error_message_becomes_error(world: &mut BotSpyWorld) {
    let extraction = world
        .ag_extraction
        .as_ref()
        .expect("no Antigravity extraction");
    let error = extraction
        .session
        .messages
        .iter()
        .find(|message| {
            message
                .provenance
                .as_ref()
                .and_then(|prov| prov.record_type.as_deref())
                == Some("ERROR_MESSAGE")
        })
        .expect("no ERROR_MESSAGE message");
    assert!(error.is_error);
    assert_eq!(error.role, botspy::Role::System);
}

#[then(regex = r#"^the normalized messages appear in step order ([0-9]+(?:, [0-9]+)*)$"#)]
fn messages_in_step_order(world: &mut BotSpyWorld, expected: String) {
    let expected_steps: Vec<u64> = expected
        .split(", ")
        .map(|part| part.parse().expect("step index"))
        .collect();
    let extraction = world
        .ag_extraction
        .as_ref()
        .expect("no Antigravity extraction");
    let steps: Vec<u64> = extraction
        .session
        .messages
        .iter()
        .map(|message| {
            message
                .provenance
                .as_ref()
                .and_then(|prov| prov.ordinal)
                .expect("step provenance index")
        })
        .collect();
    assert_eq!(
        steps, expected_steps,
        "messages did not follow the explicit step_index order"
    );
}

#[then(regex = r#"^the tool call and result are linked by the DB-supplied call id$"#)]
fn tool_call_and_result_linked(world: &mut BotSpyWorld) {
    let extraction = world
        .ag_extraction
        .as_ref()
        .expect("no Antigravity extraction");
    let call_id = extraction
        .session
        .messages
        .iter()
        .find_map(|message| {
            message.parts.iter().find_map(|part| match part {
                botspy::Part::Known(botspy::KnownPart::ToolCall { id, .. }) => Some(id.clone()),
                _ => None,
            })
        })
        .expect("no tool call part");
    let result_call_id = extraction
        .session
        .messages
        .iter()
        .find_map(|message| {
            message.parts.iter().find_map(|part| match part {
                botspy::Part::Known(botspy::KnownPart::ToolResult { call_id, .. }) => {
                    Some(call_id.clone())
                }
                _ => None,
            })
        })
        .expect("no tool result part");
    assert_eq!(call_id, "call_5");
    assert_eq!(call_id, result_call_id);
}

#[then(regex = r#"^the source DB and its WAL were never mutated$"#)]
fn db_unmutated(world: &mut BotSpyWorld) {
    let before = world.ag_digest_before.as_ref().expect("digest before");
    let after = world.ag_digest_after.as_ref().expect("digest after");
    assert_eq!(before, after, "reading the DB mutated it");
}

#[then(regex = r#"^the live app kept writing rows without interference$"#)]
fn live_app_keeps_writing(world: &mut BotSpyWorld) {
    let writer = world.ag_writer.as_ref().expect("no live app writer");
    writer
        .execute(
            "INSERT OR REPLACE INTO tool_links (step_index, call_id, result) VALUES (9, 'call_9', 'appended during read')",
            [],
        )
        .expect("the live app could not write while we read");
}

#[then(regex = r#"^extraction maps ([0-9]+) good records?$"#)]
fn extraction_maps_good(world: &mut BotSpyWorld, count: usize) {
    let extraction = world
        .ag_extraction
        .as_ref()
        .expect("no Antigravity extraction");
    assert_eq!(
        extraction.good, count,
        "extraction mapped the wrong records"
    );
}
