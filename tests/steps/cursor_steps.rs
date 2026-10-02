//! Steps for the Cursor importer (spec 03_importers/cursor/adapter.feature).

use crate::steps::BotSpyWorld;
use botspy::adapters::cursor::{CursorCliSource, CursorSource};
use botspy::snapshot::digest_files;
use cucumber::{given, then, when};
use rusqlite::OpenFlags;
use serde_json::{json, Value};
use std::path::PathBuf;
use std::sync::atomic::{AtomicUsize, Ordering};

static UNIQUE_DIR: AtomicUsize = AtomicUsize::new(0);

const BLOB_HASH: &str = "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855";

fn new_root(world: &mut BotSpyWorld) -> PathBuf {
    let base = std::env::temp_dir().join("botspy-cursor").join(format!(
        "{}-{}",
        std::process::id(),
        UNIQUE_DIR.fetch_add(1, Ordering::SeqCst)
    ));
    std::fs::create_dir_all(&base).expect("create Cursor fixture dir");
    world.cur_store = None;
    world.cur_source = None;
    world.cur_cli_root = None;
    world.cur_cli_source = None;
    world.cur_composer = None;
    world.cur_discovery = None;
    world.cur_extraction = None;
    base
}

fn open_store(path: &std::path::Path) -> rusqlite::Connection {
    let conn = rusqlite::Connection::open(path).expect("open Cursor KV store");
    conn.pragma_update(None, "journal_mode", "WAL")
        .expect("enable WAL mode");
    conn.execute(
        "CREATE TABLE IF NOT EXISTS cursorDiskKV (key TEXT PRIMARY KEY, value TEXT NOT NULL)",
        [],
    )
    .expect("create cursorDiskKV table");
    conn
}

fn put_value(conn: &rusqlite::Connection, key: &str, value: &Value) {
    conn.execute(
        "INSERT OR REPLACE INTO cursorDiskKV (key, value) VALUES (?1, ?2)",
        rusqlite::params![key, value.to_string()],
    )
    .expect("write KV value");
}

fn composer_entry(id: &str, last_updated_at: i64, name: &str) -> Value {
    json!({"composerId": id, "lastUpdatedAt": last_updated_at, "name": name})
}

fn composer_data(headers: &[&str], name: &str) -> Value {
    json!({
        "name": name,
        "fullConversationHeadersOnly": headers
            .iter()
            .map(|bubble_id| json!({"bubbleId": bubble_id}))
            .collect::<Vec<_>>()
    })
}

fn user_bubble(text: &str) -> Value {
    json!({"type": 1, "text": text})
}

fn assistant_thinking_bubble(text: &str, thinking: &str) -> Value {
    json!({"type": 2, "text": text, "thinking": thinking})
}

fn assistant_tool_bubble(name: &str) -> Value {
    json!({"type": 2, "toolFormerData": {"name": name, "params": "{\"cmd\":[\"cargo\",\"test\"]}"}})
}

fn assistant_blob_bubble() -> Value {
    json!({"type": 2, "toolResultBlob": BLOB_HASH})
}

fn seed_store(
    world: &mut BotSpyWorld,
    composers: &[Value],
    composer_data_rows: &[(&str, Value)],
    bubbles: &[(&str, &str, Value)],
) -> PathBuf {
    let base = new_root(world);
    let store = base.join("state.vscdb");
    let conn = open_store(&store);
    put_value(&conn, "composerHeaders", &Value::Array(composers.to_vec()));
    for (id, data) in composer_data_rows {
        put_value(&conn, &format!("composerData:{id}"), data);
    }
    for (composer_id, bubble_id, bubble) in bubbles {
        put_value(
            &conn,
            &format!("bubbleId:{composer_id}:{bubble_id}"),
            bubble,
        );
    }
    world.cur_store = Some(store.clone());
    world.cur_source = Some(CursorSource::new(&store));
    store
}

#[given(regex = r#"^a Cursor KV store with ([0-9]+) composers in "composerHeaders"$"#)]
fn kv_store_with_composers(world: &mut BotSpyWorld, count: usize) {
    let composers: Vec<Value> = (1..=count)
        .map(|i| composer_entry(&format!("composer-{i}"), 1000 + i as i64, "demo"))
        .collect();
    let data_rows: Vec<(String, Value)> = (1..=count)
        .map(|i| (format!("composer-{i}"), composer_data(&["b1"], "demo")))
        .collect();
    let data_refs: Vec<(&str, Value)> = data_rows
        .iter()
        .map(|(id, data)| (id.as_str(), data.clone()))
        .collect();
    let bubbles: Vec<(String, String, Value)> = (1..=count)
        .map(|i| {
            (
                format!("composer-{i}"),
                "b1".to_string(),
                user_bubble("hello"),
            )
        })
        .collect();
    let bubble_refs: Vec<(&str, &str, Value)> = bubbles
        .iter()
        .map(|(cid, bid, bubble)| (cid.as_str(), bid.as_str(), bubble.clone()))
        .collect();
    seed_store(world, &composers, &data_refs, &bubble_refs);
}

#[given(
    regex = r#"^a Cursor composer "([^"]+)" whose bubbles have keys "([^"]+)", "([^"]+)", and "([^"]+)"$"#
)]
fn composer_with_unordered_bubbles(
    world: &mut BotSpyWorld,
    composer_id: String,
    first: String,
    second: String,
    third: String,
) {
    let bubbles: Vec<Value> = [&first, &second, &third]
        .iter()
        .map(|bubble_id| user_bubble(&format!("text of {bubble_id}")))
        .collect();
    let rows: Vec<(String, String, Value)> = [&first, &second, &third]
        .iter()
        .zip(bubbles)
        .map(|(bubble_id, bubble)| (composer_id.clone(), (*bubble_id).clone(), bubble))
        .collect();
    let rows: Vec<(&str, &str, Value)> = rows
        .iter()
        .map(|(cid, bid, bubble)| (cid.as_str(), bid.as_str(), bubble.clone()))
        .collect();
    seed_store(
        world,
        &[composer_entry(&composer_id, 1000, "demo")],
        &[(composer_id.as_str(), composer_data(&[], "demo"))],
        &rows,
    );
    world.cur_composer = Some(composer_id);
}

#[given(regex = r#"^whose "([^"]+)" lists "([^"]+)", "([^"]+)", and "([^"]+)"$"#)]
fn composer_headers_list(
    world: &mut BotSpyWorld,
    field: String,
    first: String,
    second: String,
    third: String,
) {
    assert_eq!(
        field, "fullConversationHeadersOnly",
        "unexpected header field"
    );
    let composer_id = world.cur_composer.clone().expect("no Cursor composer");
    let store = world.cur_store.clone().expect("no Cursor KV store");
    let conn = open_store(&store);
    put_value(
        &conn,
        &format!("composerData:{composer_id}"),
        &composer_data(&[&first, &second, &third], "demo"),
    );
}

#[given(
    regex = r#"^a Cursor composer "([^"]+)" with a user bubble, an assistant bubble with thinking, and a "toolFormerData" tool call$"#
)]
fn composer_with_part_kinds(world: &mut BotSpyWorld, composer_id: String) {
    seed_store(
        world,
        &[composer_entry(&composer_id, 1000, "demo")],
        &[(
            composer_id.as_str(),
            composer_data(&["b1", "b2", "b3"], "demo"),
        )],
        &[
            (composer_id.as_str(), "b1", user_bubble("run the tests")),
            (
                composer_id.as_str(),
                "b2",
                assistant_thinking_bubble("On it.", "checking the suite"),
            ),
            (
                composer_id.as_str(),
                "b3",
                assistant_tool_bubble("run_terminal"),
            ),
        ],
    );
    world.cur_composer = Some(composer_id);
}

#[given(
    regex = r#"^a Cursor composer "([^"]+)" whose tool result lives only in "agentKv:blob:<sha256>"$"#
)]
fn composer_with_blob_result(world: &mut BotSpyWorld, composer_id: String) {
    seed_store(
        world,
        &[composer_entry(&composer_id, 1000, "demo")],
        &[(composer_id.as_str(), composer_data(&["b1"], "demo"))],
        &[(composer_id.as_str(), "b1", assistant_blob_bubble())],
    );
    world.cur_composer = Some(composer_id);
}

#[given(regex = r#"^a Cursor projects directory with CLI transcripts "([^"]+)" and "([^"]+)"$"#)]
fn projects_dir_with_transcripts(world: &mut BotSpyWorld, a: String, b: String) {
    let base = new_root(world);
    for (project, name) in [("proj-a", a), ("proj-b", b)] {
        let dir = base.join(project).join("agent-transcripts");
        std::fs::create_dir_all(&dir).expect("create agent-transcripts dir");
        std::fs::write(
            dir.join(name),
            "{\"role\":\"user\",\"text\":\"hello from the CLI\"}\n",
        )
        .expect("write CLI transcript");
    }
    world.cur_cli_root = Some(base.clone());
    world.cur_cli_source = Some(CursorCliSource::new(base));
}

#[given(regex = r#"^a Cursor KV store in WAL mode with a live app writing bubbles$"#)]
fn wal_store_with_live_app(world: &mut BotSpyWorld) {
    let base = new_root(world);
    let store = base.join("state.vscdb");
    let writer = open_store(&store);
    put_value(
        &writer,
        "composerHeaders",
        &Value::Array(vec![composer_entry("composer-1", 1000, "demo")]),
    );
    put_value(
        &writer,
        "composerData:composer-1",
        &composer_data(&["b1"], "demo"),
    );
    put_value(
        &writer,
        "bubbleId:composer-1:b1",
        &user_bubble("before the read"),
    );
    world.cur_store = Some(store.clone());
    world.cur_source = Some(CursorSource::new(store));
    world.cur_writer = Some(writer);
}

#[when(regex = r#"^the Cursor importer detects changes(?: again)?$"#)]
fn detect_changes(world: &mut BotSpyWorld) {
    if let Some(source) = world.cur_source.as_ref() {
        let (summaries, discovery) = source.discover();
        world.discovered = summaries;
        world.cur_discovery = Some(discovery);
        return;
    }
    let source = world.cur_cli_source.as_ref().expect("no Cursor source");
    let (summaries, cli) = source.discover();
    world.discovered = summaries;
    world.cur_cli_discovery = Some(cli);
}

#[when(
    regex = r#"^the app updates composer "([^"]+)" with a new bubble and bumps "lastUpdatedAt"$"#
)]
fn app_updates_composer(world: &mut BotSpyWorld, composer_id: String) {
    let store = world.cur_store.clone().expect("no Cursor KV store");
    let writer = open_store(&store);
    let headers: Vec<Value> = ["composer-1", "composer-2", "composer-3"]
        .iter()
        .map(|id| {
            if *id == composer_id {
                composer_entry(id, 9000, "demo")
            } else {
                composer_entry(
                    id,
                    1000 + id
                        .trim_start_matches("composer-")
                        .parse::<i64>()
                        .expect("composer index"),
                    "demo",
                )
            }
        })
        .collect();
    put_value(&writer, "composerHeaders", &Value::Array(headers));
    put_value(
        &writer,
        &format!("bubbleId:{composer_id}:b2"),
        &user_bubble("fresh bubble"),
    );
}

#[when(regex = r#"^the Cursor importer extracts the composer$"#)]
fn extract_composer(world: &mut BotSpyWorld) {
    let source = world.cur_source.as_ref().expect("no Cursor source");
    let composer_id = world.cur_composer.as_deref().expect("no Cursor composer");
    let extraction = source
        .extract(composer_id)
        .expect("no composer data for the id");
    world.cur_extraction = Some(extraction);
}

#[when(regex = r#"^the importer reads the store through a snapshot copy$"#)]
fn read_through_snapshot(world: &mut BotSpyWorld) {
    let store = world.cur_store.clone().expect("no Cursor KV store");
    let wal = wal_path(&store);
    world.cur_digest_before = Some(digest_files(&[&store, &wal]));
    let discovery = world
        .cur_source
        .as_ref()
        .expect("no Cursor source")
        .discover();
    world.cur_discovery = Some(discovery.1);
    world.cur_digest_after = Some(digest_files(&[&store, &wal]));
}

fn wal_path(db: &std::path::Path) -> PathBuf {
    let mut wal = db.as_os_str().to_owned();
    wal.push("-wal");
    PathBuf::from(wal)
}

#[then(regex = r#"^only composer "([^"]+)" is reported as changed$"#)]
fn only_composer_changed(world: &mut BotSpyWorld, composer_id: String) {
    let discovery = world.cur_discovery.as_ref().expect("no Cursor discovery");
    assert_eq!(
        discovery.changed,
        vec![composer_id],
        "the wrong composers were reported as changed"
    );
}

#[then(
    regex = r#"^the normalized messages appear in headers order "([^"]+)", "([^"]+)", and "([^"]+)"$"#
)]
fn messages_in_headers_order(
    world: &mut BotSpyWorld,
    first: String,
    second: String,
    third: String,
) {
    let extraction = world.cur_extraction.as_ref().expect("no Cursor extraction");
    let order: Vec<&str> = extraction
        .session
        .messages
        .iter()
        .map(|message| {
            message
                .provenance
                .as_ref()
                .and_then(|prov| prov.record_id.as_deref())
                .expect("bubble provenance")
        })
        .collect();
    assert_eq!(
        order,
        vec![first.as_str(), second.as_str(), third.as_str()],
        "bubbles did not follow the headers order"
    );
}

#[then(regex = r#"^the normalized messages map the bubble roles, thinking, and tool call kinds$"#)]
fn messages_map_bubble_kinds(world: &mut BotSpyWorld) {
    let extraction = world.cur_extraction.as_ref().expect("no Cursor extraction");
    let messages = &extraction.session.messages;
    assert_eq!(messages.len(), 3, "wrong number of normalized bubbles");
    assert_eq!(messages[0].role, botspy::Role::User);
    assert_eq!(messages[1].role, botspy::Role::Assistant);
    assert!(
        matches!(
            messages[1].parts.get(1),
            Some(botspy::Part::Known(botspy::KnownPart::Thinking { .. }))
        ),
        "the assistant thinking did not map to a thinking part"
    );
    assert!(
        matches!(
            messages[2].parts.first(),
            Some(botspy::Part::Known(botspy::KnownPart::ToolCall { .. }))
        ),
        "the toolFormerData did not map to a tool call part"
    );
}

#[then(regex = r#"^the tool result is a blob part with the blob hash and container "([^"]+)"$"#)]
fn blob_part_with_hash_and_container(world: &mut BotSpyWorld, container: String) {
    let extraction = world.cur_extraction.as_ref().expect("no Cursor extraction");
    let blob = extraction
        .session
        .messages
        .iter()
        .find_map(|message| {
            message.parts.iter().find_map(|part| match part {
                botspy::Part::Known(botspy::KnownPart::Blob {
                    blob_hash,
                    container: found,
                    ..
                }) => Some((blob_hash.clone(), found.clone())),
                _ => None,
            })
        })
        .expect("no blob part was normalized");
    assert_eq!(blob.1, container);
    assert_eq!(blob.0, BLOB_HASH);
}

#[then(regex = r#"^the source store and its WAL were never mutated$"#)]
fn store_unmutated(world: &mut BotSpyWorld) {
    let before = world.cur_digest_before.as_ref().expect("digest before");
    let after = world.cur_digest_after.as_ref().expect("digest after");
    assert_eq!(before, after, "reading the store mutated it");
}

#[then(regex = r#"^the live app kept writing without interference$"#)]
fn live_app_keeps_writing(world: &mut BotSpyWorld) {
    let writer = world.cur_writer.as_ref().expect("no live app writer");
    put_value(
        writer,
        "bubbleId:composer-1:b2",
        &user_bubble("appended during read"),
    );
    let store = world.cur_store.clone().expect("no Cursor KV store");
    let conn = rusqlite::Connection::open_with_flags(&store, OpenFlags::SQLITE_OPEN_READ_ONLY)
        .expect("reopen store");
    let bubbles: u64 = conn
        .query_row(
            "SELECT COUNT(*) FROM cursorDiskKV WHERE key LIKE 'bubbleId:%'",
            [],
            |row| row.get::<_, i64>(0),
        )
        .expect("count bubbles") as u64;
    assert_eq!(bubbles, 2, "the appended bubble is missing from the store");
}
