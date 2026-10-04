//! Synthetic fixture corpus for all five agents (BOTSPY-6acb77).
//!
//! The committed tree under `tests/fixtures/` mirrors each agent's real
//! on-disk layout with fully synthetic content — no real transcripts, names,
//! or secrets. The corpus is the reference layout for every importer.
//!
//! Regenerate the committed corpus with:
//!
//! ```text
//! BOTSPY_FIXTURES_REGEN=1 cargo test --test fixtures_corpus
//! ```
//!
//! then re-run the same test without the env var to verify parity — regen
//! wipes and rewrites the committed tree, so it is a maintainer action, not
//! something CI ever does.
//!
//! Three guards run here:
//! 1. `committed_fixtures_match_generator` — the committed tree is exactly
//!    what the generator produces (text files byte-for-byte, SQLite files
//!    by logical content so page-level rebuilds cannot cause drift).
//! 2. `fixtures_contain_no_secrets` — pattern scan plus home-path and
//!    stray-WAL checks prove the corpus stays synthetic-only.
//! 3. `fixtures_feed_every_adapter` — every importer discovers and extracts
//!    against the committed corpus, so layout drift fails loudly.

use botspy::adapters::antigravity::AntigravitySource;
use botspy::adapters::claude_code::ClaudeCodeSource;
use botspy::adapters::codex::CodexSource;
use botspy::adapters::cursor::{CursorCliSource, CursorSource};
use botspy::adapters::grok_bot::GrokBotSource;
use serde_json::{json, Value};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicUsize, Ordering};

const CLAUDE_SESSION_1: &str = "00000000-0000-4000-8000-000000000001";
const CODEX_THREAD_1: &str = "t-synth-1";
const CURSOR_COMPOSER_1: &str = "composer-synth-1";
const GROK_BLOB_1: &str = "0000000000000000000000000000000000000000000000000000000000000001";
const ANTIGRAVITY_CONVERSATION_1: &str = "conv-synth-1";

/// sha256 of the empty string — publicly known, used as the synthetic Cursor
/// tool-result blob hash.
const CURSOR_BLOB_HASH: &str = "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855";

fn fixtures_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures")
}

fn build_corpus(root: &Path) {
    std::fs::create_dir_all(root).expect("create corpus root");
    build_claude_code(&root.join("claude_code/projects"));
    build_codex(&root.join("codex"));
    build_cursor(&root.join("cursor"));
    build_grok_bot(&root.join("grok_bot/sand-client-persistence"));
    build_antigravity(&root.join("antigravity"));
    prune_sqlite_sidecars(root);
}

// ── Claude Code ─────────────────────────────────────────────────────────────

fn claude_user(uuid: &str, parent: &str, timestamp: &str, text: &str) -> String {
    format!(
        r#"{{"type":"user","uuid":"{uuid}","parentUuid":"{parent}","timestamp":"{timestamp}","message":{{"role":"user","content":"{text}"}}}}"#
    )
}

fn build_claude_code(root: &Path) {
    let alpha = root.join("-Users-synth-workspace-alpha");
    std::fs::create_dir_all(&alpha).expect("create Claude Code project dir");
    let records = [
        claude_user("u1", "", "2026-10-01T09:00:00Z", "kick off the parser work"),
        r#"{"type":"assistant","uuid":"u2","parentUuid":"u1","timestamp":"2026-10-01T09:00:05Z","message":{"role":"assistant","content":[{"type":"thinking","thinking":"reading the transcript stream","signature":"sig"},{"type":"tool_use","id":"call_synth_1","name":"read_file","input":{"path":"main.rs"}}]}}"#.to_string(),
        r#"{"type":"user","uuid":"u3","parentUuid":"u2","timestamp":"2026-10-01T09:00:10Z","message":{"role":"user","content":[{"type":"tool_result","tool_use_id":"call_synth_1","content":"contents"}]}}"#.to_string(),
        r#"{"type":"assistant","uuid":"u4","parentUuid":"u3","timestamp":"2026-10-01T09:00:15Z","message":{"role":"assistant","content":[{"type":"text","text":"the parser work is done"}]}}"#.to_string(),
        r#"{"type":"cost-state","totalCostUsd":0.42,"totalApiDurationMs":1200}"#.to_string(),
        r#"{"type":"pr-link","url":"https://github.com/synth/repo/pull/1"}"#.to_string(),
        r#"{"type":"custom-title","title":"Synthetic parser session"}"#.to_string(),
    ];
    std::fs::write(
        alpha.join(format!("{CLAUDE_SESSION_1}.jsonl")),
        records
            .iter()
            .map(|line| format!("{line}\n"))
            .collect::<String>(),
    )
    .expect("write Claude Code session 1");
    std::fs::write(
        alpha.join("00000000-0000-4000-8000-000000000002.jsonl"),
        format!(
            "{}\n",
            claude_user(
                "u1",
                "",
                "2026-10-01T09:10:00Z",
                "wrap up and commit the corpus"
            )
        ),
    )
    .expect("write Claude Code session 2");
    let beta = root.join("-Users-synth-workspace-beta");
    std::fs::create_dir_all(&beta).expect("create Claude Code beta project dir");
    std::fs::write(
        beta.join("00000000-0000-4000-8000-000000000003.jsonl"),
        format!(
            "{}\n",
            claude_user(
                "u1",
                "",
                "2026-10-01T09:20:00Z",
                "explore the synthetic corpus"
            )
        ),
    )
    .expect("write Claude Code session 3");
}

// ── Codex ───────────────────────────────────────────────────────────────────

fn codex_message(
    ordinal: u64,
    timestamp: &str,
    role: &str,
    content_field: &str,
    text: &str,
) -> String {
    format!(
        r#"{{"ordinal":{ordinal},"timestamp":"{timestamp}","type":"response_item","payload":{{"type":"message","id":"m{ordinal}","role":"{role}","content":[{{"type":"{content_field}","text":"{text}"}}]}}}}"#
    )
}

fn build_codex(root: &Path) {
    std::fs::create_dir_all(root).expect("create Codex home dir");
    let conn =
        rusqlite::Connection::open(root.join("state_5.sqlite")).expect("open Codex threads index");
    conn.pragma_update(None, "journal_mode", "WAL")
        .expect("enable WAL mode");
    // The real threads table: no parent column — spawn edges live in
    // thread_spawn_edges — plus the metadata columns the adapter maps.
    conn.execute_batch(
        "CREATE TABLE threads (
    id TEXT PRIMARY KEY,
    rollout_path TEXT NOT NULL,
    created_at INTEGER NOT NULL,
    updated_at INTEGER NOT NULL,
    source TEXT NOT NULL,
    model_provider TEXT NOT NULL,
    cwd TEXT NOT NULL,
    title TEXT NOT NULL,
    sandbox_policy TEXT NOT NULL,
    approval_mode TEXT NOT NULL,
    tokens_used INTEGER NOT NULL DEFAULT 0,
    has_user_event INTEGER NOT NULL DEFAULT 0,
    archived INTEGER NOT NULL DEFAULT 0,
    archived_at INTEGER,
    git_sha TEXT,
    git_branch TEXT,
    git_origin_url TEXT
, cli_version TEXT NOT NULL DEFAULT '', first_user_message TEXT NOT NULL DEFAULT '', agent_nickname TEXT, agent_role TEXT, memory_mode TEXT NOT NULL DEFAULT 'enabled', model TEXT, reasoning_effort TEXT, agent_path TEXT, created_at_ms INTEGER, updated_at_ms INTEGER, thread_source TEXT, preview TEXT NOT NULL DEFAULT '', recency_at INTEGER NOT NULL DEFAULT 0, recency_at_ms INTEGER NOT NULL DEFAULT 0, history_mode TEXT NOT NULL DEFAULT 'legacy', name TEXT, is_pinned INTEGER NOT NULL DEFAULT 0, thread_section_id TEXT, section_position INTEGER, section_entered_at_ms INTEGER, project_id TEXT, originator TEXT, daybreak_enabled BOOLEAN, creator_user_id TEXT, creator_account_id TEXT);
CREATE TABLE thread_spawn_edges (
    parent_thread_id TEXT NOT NULL,
    child_thread_id TEXT NOT NULL PRIMARY KEY,
    status TEXT NOT NULL
);",
    )
    .expect("create Codex index tables");
    for (id, rollout_path, title, created_s, updated_s) in [
        (
            CODEX_THREAD_1,
            "sessions/2026/10/01/rollout-synth-1.jsonl",
            "Alpha build fix",
            1790812800,
            1790845220,
        ),
        (
            "t-synth-2",
            "sessions/2026/10/01/rollout-synth-2.jsonl",
            "Alpha subagent run",
            1790845200,
            1790845500,
        ),
    ] {
        conn.execute(
            "INSERT INTO threads (id, rollout_path, created_at, updated_at, source, \
             model_provider, cwd, title, sandbox_policy, approval_mode, git_sha, git_branch, \
             git_origin_url, model, created_at_ms, updated_at_ms) \
             VALUES (?1, ?2, ?3, ?4, 'cli', 'synthetic', '/workspace/alpha', ?5, 'off', 'never', \
             'abcdef1234567890', 'feat/alpha', 'https://git.example.com/acme/alpha.git', \
             'synthetic-model', ?3 * 1000, ?4 * 1000)",
            rusqlite::params![id, rollout_path, created_s, updated_s, title],
        )
        .expect("insert thread row");
    }
    conn.execute(
        "INSERT INTO thread_spawn_edges (parent_thread_id, child_thread_id, status) \
         VALUES (?1, 't-synth-2', 'open')",
        rusqlite::params![CODEX_THREAD_1],
    )
    .expect("insert spawn edge");
    drop(conn);

    let rollout_dir = root.join("sessions/2026/10/01");
    std::fs::create_dir_all(&rollout_dir).expect("create rollout dirs");
    let records = [
        codex_message(1, "2026-10-01T09:00:00Z", "user", "input_text", "run the tests"),
        codex_message(2, "2026-10-01T09:00:05Z", "assistant", "output_text", "On it."),
        codex_message(3, "2026-10-01T09:00:06Z", "developer", "input_text", "Be concise."),
        r#"{"ordinal":4,"timestamp":"2026-10-01T09:00:07Z","type":"response_item","payload":{"type":"reasoning","summary":[{"type":"summary_text","text":"thinking it through"}]}}"#.to_string(),
        r#"{"ordinal":5,"timestamp":"2026-10-01T09:00:08Z","type":"response_item","payload":{"type":"function_call","call_id":"call_synth_1","name":"shell","arguments":"{\"cmd\":[\"cargo\",\"test\"]}"}}"#.to_string(),
        r#"{"ordinal":6,"timestamp":"2026-10-01T09:00:20Z","type":"response_item","payload":{"type":"function_call_output","call_id":"call_synth_1","output":"all tests pass"}}"#.to_string(),
        r#"{"ordinal":7,"type":"token_usage_record","payload":{"input_tokens":1200,"cached_input_tokens":300,"output_tokens":800,"reasoning_tokens":150,"model":"synthetic-model"}}"#.to_string(),
        r#"{"ordinal":8,"type":"compacted","payload":{"window_id":"w2","previous_window":"w1","retained_context":"synthetic summary so far"}}"#.to_string(),
        r#"{"ordinal":9,"type":"task_complete","payload":{"turn_id":"turn-synth-1","duration_ms":42000,"time_to_first_token_ms":900}}"#.to_string(),
    ];
    std::fs::write(
        rollout_dir.join("rollout-synth-1.jsonl"),
        records
            .iter()
            .map(|line| format!("{line}\n"))
            .collect::<String>(),
    )
    .expect("write Codex rollout 1");
    std::fs::write(
        rollout_dir.join("rollout-synth-2.jsonl"),
        format!(
            "{}\n",
            codex_message(
                1,
                "2026-10-01T09:05:00Z",
                "user",
                "input_text",
                "child thread work"
            )
        ),
    )
    .expect("write Codex rollout 2");
}

// ── Cursor ──────────────────────────────────────────────────────────────────

fn open_kv_store(path: &Path) -> rusqlite::Connection {
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

fn build_cursor(root: &Path) {
    std::fs::create_dir_all(root.join("store")).expect("create Cursor store dir");
    let conn = open_kv_store(&root.join("store/state.vscdb"));
    put_value(
        &conn,
        "composerHeaders",
        &json!([
            {"composerId": CURSOR_COMPOSER_1, "lastUpdatedAt": 1769850000000i64, "name": "parser work"},
            {"composerId": "composer-synth-2", "lastUpdatedAt": 1769853000000i64, "name": "corpus sweep"}
        ]),
    );
    put_value(
        &conn,
        &format!("composerData:{CURSOR_COMPOSER_1}"),
        &json!({
            "name": "parser work",
            "fullConversationHeadersOnly": [{"bubbleId": "b1"}, {"bubbleId": "b2"}, {"bubbleId": "b3"}]
        }),
    );
    put_value(
        &conn,
        "composerData:composer-synth-2",
        &json!({
            "name": "corpus sweep",
            "fullConversationHeadersOnly": [{"bubbleId": "b1"}]
        }),
    );
    put_value(
        &conn,
        &format!("bubbleId:{CURSOR_COMPOSER_1}:b1"),
        &json!({"type": 1, "text": "run the tests"}),
    );
    put_value(
        &conn,
        &format!("bubbleId:{CURSOR_COMPOSER_1}:b2"),
        &json!({"type": 2, "text": "On it.", "thinking": "checking the suite"}),
    );
    put_value(
        &conn,
        &format!("bubbleId:{CURSOR_COMPOSER_1}:b3"),
        &json!({"type": 2, "toolFormerData": {"name": "run_terminal", "params": "{\"cmd\":[\"cargo\",\"test\"]}"}}),
    );
    put_value(
        &conn,
        "bubbleId:composer-synth-2:b1",
        &json!({"type": 2, "toolResultBlob": CURSOR_BLOB_HASH}),
    );
    put_value(
        &conn,
        &format!("agentKv:blob:{CURSOR_BLOB_HASH}"),
        &json!({"toolResult": "all tests pass"}),
    );
    drop(conn);

    let cli_dir = root.join("cli/projects/alpha/agent-transcripts");
    std::fs::create_dir_all(&cli_dir).expect("create Cursor CLI transcript dir");
    std::fs::write(
        cli_dir.join("cli-synth-1.jsonl"),
        "{\"role\":\"user\",\"text\":\"hello from the CLI\"}\n{\"role\":\"assistant\",\"text\":\"hello back\"}\n",
    )
    .expect("write Cursor CLI transcript");
}

// ── Grok Bot ────────────────────────────────────────────────────────────────

fn grok_message(seq: u64, role: &str, text: &str, author: Option<&str>) -> Value {
    let mut entry = json!({"seq": seq, "type": "message", "role": role, "text": text});
    if let Some(author) = author {
        entry["author"] = json!(author);
    }
    entry
}

fn build_grok_bot(root: &Path) {
    std::fs::create_dir_all(root).expect("create Grok Bot persistence dir");
    let blob_2 = format!("{:064x}", 2);
    let blob_1 = json!({
        "entries": [
            grok_message(1, "user", "hello from the synth blob", None),
            grok_message(2, "assistant", "persona hello", Some("rocket-synth")),
            json!({"seq": 3, "type": "voice_call", "callId": "vc-synth-1", "payload": "opaque-bytes"})
        ]
    });
    std::fs::write(root.join(format!("{GROK_BLOB_1}.json")), blob_1.to_string())
        .expect("write Grok Bot blob 1");
    std::fs::write(
        root.join(format!("{blob_2}.json")),
        json!({
            "capped": true,
            "continuedServerSide": true,
            "entries": [
                grok_message(1, "user", "capped entry one", None),
                grok_message(2, "user", "capped entry two", None)
            ]
        })
        .to_string(),
    )
    .expect("write Grok Bot blob 2");
    std::fs::write(
        root.join("roster.json"),
        json!({GROK_BLOB_1: "rocket-synth"}).to_string(),
    )
    .expect("write Grok Bot roster");
    std::fs::write(
        root.join("cloud-agents.json"),
        json!([{
            "agentId": "peer-synth-1",
            "status": "running",
            "prUrl": "https://github.com/synth/repo/pull/1"
        }])
        .to_string(),
    )
    .expect("write Grok Bot cloud agents");
}

// ── Antigravity ─────────────────────────────────────────────────────────────

fn build_antigravity(root: &Path) {
    std::fs::create_dir_all(root).expect("create Antigravity data dir");
    let conn = rusqlite::Connection::open(root.join("conversation_summaries.db"))
        .expect("open Antigravity summaries index");
    conn.pragma_update(None, "journal_mode", "WAL")
        .expect("enable WAL mode");
    conn.execute(
        "CREATE TABLE IF NOT EXISTS conversations (id TEXT PRIMARY KEY, name TEXT, updated_at TEXT)",
        [],
    )
    .expect("create conversations table");
    for (id, name, updated_at) in [
        (
            ANTIGRAVITY_CONVERSATION_1,
            "parser work",
            "2026-10-01T09:00:00Z",
        ),
        ("conv-synth-2", "corpus sweep", "2026-10-01T09:05:00Z"),
    ] {
        conn.execute(
            "INSERT OR REPLACE INTO conversations (id, name, updated_at) VALUES (?1, ?2, ?3)",
            rusqlite::params![id, name, updated_at],
        )
        .expect("insert conversation row");
    }
    drop(conn);

    for (id, records, with_payload) in [
        (
            ANTIGRAVITY_CONVERSATION_1,
            &[
                r#"{"type":"USER_INPUT","step_index":1,"created_at":"2026-10-01T09:00:00Z","content":"run the tests"}"#,
                r#"{"type":"PLANNER_RESPONSE","step_index":2,"created_at":"2026-10-01T09:00:05Z","content":"On it."}"#,
                r#"{"type":"TOOL_CALL","step_index":3,"created_at":"2026-10-01T09:00:10Z","toolName":"run_terminal","arguments":"{\"cmd\":[\"cargo\",\"test\"]}"}"#,
                r#"{"type":"CHECKPOINT","step_index":4,"created_at":"2026-10-01T09:00:15Z","summary":"checkpoint state"}"#,
                r#"{"type":"ERROR_MESSAGE","step_index":5,"created_at":"2026-10-01T09:00:20Z","content":"synthetic failure"}"#,
            ][..],
            true,
        ),
        (
            "conv-synth-2",
            &[
                r#"{"type":"USER_INPUT","step_index":1,"created_at":"2026-10-01T09:05:00Z","content":"explore the corpus"}"#,
                r#"{"type":"PLANNER_RESPONSE","step_index":2,"created_at":"2026-10-01T09:05:05Z","content":"On it."}"#,
            ][..],
            false,
        ),
    ] {
        let logs = root
            .join("brain")
            .join(id)
            .join(".system_generated")
            .join("logs");
        std::fs::create_dir_all(&logs).expect("create Antigravity logs dir");
        std::fs::write(
            logs.join("transcript.jsonl"),
            records
                .iter()
                .map(|line| format!("{line}\n"))
                .collect::<String>(),
        )
        .expect("write Antigravity transcript");
        if with_payload {
            let conn = rusqlite::Connection::open(root.join("brain").join(id).join("payload.db"))
                .expect("open Antigravity payload DB");
            conn.pragma_update(None, "journal_mode", "WAL")
                .expect("enable WAL mode");
            conn.execute(
                "CREATE TABLE IF NOT EXISTS tool_links (step_index INTEGER PRIMARY KEY, call_id TEXT NOT NULL, result TEXT)",
                [],
            )
            .expect("create tool_links table");
            conn.execute(
                "INSERT OR REPLACE INTO tool_links (step_index, call_id, result) VALUES (?1, ?2, ?3)",
                rusqlite::params![3, "call_synth_3", "all tests pass"],
            )
            .expect("insert tool link");
            drop(conn);
        }
    }
}

fn prune_sqlite_sidecars(root: &Path) {
    for entry in walk_files(root) {
        let name = entry
            .file_name()
            .and_then(|name| name.to_str())
            .unwrap_or("");
        for suffix in ["-wal", "-shm", "-journal"] {
            if name.ends_with(suffix) {
                std::fs::remove_file(&entry).expect("remove sqlite sidecar");
            }
        }
    }
}

// ── Corpus guards ───────────────────────────────────────────────────────────

fn walk_files(root: &Path) -> Vec<PathBuf> {
    let mut files = Vec::new();
    let Ok(entries) = std::fs::read_dir(root) else {
        return files;
    };
    for entry in entries.flatten() {
        let path = entry.path();
        if path.is_dir() {
            files.extend(walk_files(&path));
        } else {
            files.push(path);
        }
    }
    files
}

fn list_relative(root: &Path) -> Vec<String> {
    let mut files: Vec<String> = walk_files(root)
        .into_iter()
        .filter(|path| !is_sqlite_sidecar(path))
        .map(|path| {
            path.strip_prefix(root)
                .expect("path under root")
                .to_string_lossy()
                .into_owned()
        })
        .collect();
    files.sort();
    files
}

/// SQLite runtime sidecars: created when adapters open the WAL-mode stores in
/// place, removed by SQLite on clean close, and never corpus content.
fn is_sqlite_sidecar(path: &Path) -> bool {
    path.file_name()
        .and_then(|name| name.to_str())
        .is_some_and(|name| {
            ["-wal", "-shm", "-journal"]
                .iter()
                .any(|suffix| name.ends_with(suffix))
        })
}

fn is_sqlite(rel: &str) -> bool {
    rel.ends_with(".sqlite") || rel.ends_with(".vscdb") || rel.ends_with(".db")
}

/// Logical content of a SQLite file: schema plus every row, so regeneration
/// with a different page layout or SQLite build cannot cause false drift.
/// Dumped through a scratch copy — even a read-only open of a WAL-mode
/// database can create -shm sidecars next to the original.
fn sqlite_dump(path: &Path) -> String {
    static DUMP_ID: AtomicUsize = AtomicUsize::new(0);
    let scratch = std::env::temp_dir()
        .join("botspy-fixtures-dump")
        .join(format!(
            "{}-{}",
            std::process::id(),
            DUMP_ID.fetch_add(1, Ordering::SeqCst)
        ));
    std::fs::create_dir_all(&scratch).expect("create sqlite dump scratch dir");
    let copy = scratch.join("dump.sqlite");
    std::fs::copy(path, &copy).expect("copy sqlite file for dumping");
    let dump = sqlite_dump_read_only(&copy);
    let _ = std::fs::remove_dir_all(&scratch);
    dump
}

fn sqlite_dump_read_only(path: &Path) -> String {
    let conn =
        rusqlite::Connection::open_with_flags(path, rusqlite::OpenFlags::SQLITE_OPEN_READ_ONLY)
            .unwrap_or_else(|error| panic!("open {path:?} read-only: {error}"));
    let mut out = String::new();
    let mut stmt = conn
        .prepare(
            "SELECT name, sql FROM sqlite_master WHERE type = 'table' AND name NOT LIKE 'sqlite_%' ORDER BY name",
        )
        .expect("list tables");
    let tables: Vec<(String, String)> = stmt
        .query_map([], |row| Ok((row.get(0)?, row.get(1)?)))
        .expect("query tables")
        .map(Result::unwrap)
        .collect();
    drop(stmt);
    for (name, sql) in tables {
        out.push_str(&sql);
        out.push('\n');
        let mut stmt = conn
            .prepare(&format!("SELECT * FROM {name} ORDER BY rowid"))
            .expect("prepare table scan");
        let names: Vec<String> = stmt
            .column_names()
            .iter()
            .map(|column| (*column).to_string())
            .collect();
        let mut rows = stmt.query([]).expect("scan table");
        while let Some(row) = rows.next().expect("read row") {
            let values: Vec<String> = (0..names.len())
                .map(|index| fmt_value(row.get_ref(index).expect("column value")))
                .collect();
            out.push_str(&values.join("|"));
            out.push('\n');
        }
    }
    out
}

fn fmt_value(value: rusqlite::types::ValueRef<'_>) -> String {
    match value {
        rusqlite::types::ValueRef::Null => "null".to_string(),
        rusqlite::types::ValueRef::Integer(value) => value.to_string(),
        rusqlite::types::ValueRef::Real(value) => value.to_string(),
        rusqlite::types::ValueRef::Text(bytes) => String::from_utf8_lossy(bytes).into_owned(),
        rusqlite::types::ValueRef::Blob(bytes) => {
            bytes.iter().map(|byte| format!("{byte:02x}")).collect()
        }
    }
}

fn copy_tree(from: &Path, to: &Path) {
    for rel in list_relative(from) {
        let target = to.join(&rel);
        std::fs::create_dir_all(target.parent().expect("target parent"))
            .expect("create target dir");
        std::fs::copy(from.join(&rel), &target).expect("copy fixture file");
    }
}

#[test]
fn committed_fixtures_match_generator() {
    let committed = fixtures_root();
    let temp = std::env::temp_dir()
        .join("botspy-fixtures-regen")
        .join(std::process::id().to_string());
    let _ = std::fs::remove_dir_all(&temp);
    std::fs::create_dir_all(&temp).expect("create regen dir");
    build_corpus(&temp);

    if std::env::var("BOTSPY_FIXTURES_REGEN").is_ok() {
        let _ = std::fs::remove_dir_all(&committed);
        std::fs::create_dir_all(&committed).expect("recreate committed corpus");
        copy_tree(&temp, &committed);
        let _ = std::fs::remove_dir_all(&temp);
        return;
    }

    let expected = list_relative(&temp);
    let actual = list_relative(&committed);
    assert_eq!(
        actual, expected,
        "the committed fixture tree differs from the generator output"
    );
    for rel in expected {
        let generated = temp.join(&rel);
        let checked_in = committed.join(&rel);
        if is_sqlite(&rel) {
            assert_eq!(
                sqlite_dump(&generated),
                sqlite_dump(&checked_in),
                "logical SQLite content drift in {rel}"
            );
        } else {
            assert_eq!(
                std::fs::read(&generated).expect("read generated"),
                std::fs::read(&checked_in).expect("read committed"),
                "byte drift in {rel}"
            );
        }
    }
    let _ = std::fs::remove_dir_all(&temp);
}

const SECRET_PATTERNS: &[&str] = &[
    "sk-",
    "ghp_",
    "gho_",
    "ghu_",
    "ghs_",
    "ghr_",
    "github_pat_",
    "glpat_",
    "xoxa-",
    "xoxb-",
    "xoxp-",
    "xoxs-",
    "akia",
    "-----begin rsa private key-----",
    "-----begin openssh private key-----",
    "-----begin private key-----",
    "-----begin ec private key-----",
    "-----begin dsa private key-----",
];

const SECRET_ASSIGNMENT_PATTERNS: &[&str] = &[
    "\"apikey\":\"",
    "\"api_key\":\"",
    "\"secret\":\"",
    "\"password\":\"",
    "\"authorization\":\"",
    "api_key=",
    "password=",
    "passwd=",
    "secret=",
];

#[test]
fn fixtures_contain_no_secrets() {
    let committed = fixtures_root();
    // Adapters opening the WAL-mode stores in place leave -wal/-shm sidecars
    // behind on an unclean exit; they are runtime artifacts (gitignored), so
    // prune them before scanning content.
    for path in walk_files(&committed) {
        if is_sqlite_sidecar(&path) {
            let _ = std::fs::remove_file(&path);
        }
    }
    let files = list_relative(&committed);
    assert!(
        !files.is_empty(),
        "the fixture corpus is empty; run BOTSPY_FIXTURES_REGEN=1 cargo test --test fixtures_corpus"
    );
    for rel in files {
        let bytes = std::fs::read(committed.join(&rel)).expect("read fixture file");
        let text = String::from_utf8_lossy(&bytes).to_lowercase();
        for pattern in SECRET_PATTERNS.iter().chain(SECRET_ASSIGNMENT_PATTERNS) {
            assert!(
                !text.contains(pattern),
                "possible secret pattern {pattern:?} in fixture {rel}"
            );
        }
    }
    if let Ok(home) = std::env::var("HOME") {
        // An empty or relative HOME would make contains() trivially true or
        // meaningless (musl cross containers set HOME=""), so only check a
        // plausible absolute home path.
        if home.len() > 1 && home.starts_with('/') {
            for rel in list_relative(&committed) {
                let bytes = std::fs::read(committed.join(&rel)).expect("read fixture file");
                let text = String::from_utf8_lossy(&bytes).into_owned();
                assert!(
                    !text.contains(&home),
                    "fixture {rel} leaks the real home directory path"
                );
            }
        }
    }
}

#[test]
fn fixtures_feed_every_adapter() {
    let fixtures = fixtures_root();
    // Adapters open SQLite sources in place and WAL-mode opens create -wal/-shm
    // sidecars, so exercise them against a scratch copy and keep the committed
    // corpus pristine.
    let scratch = std::env::temp_dir()
        .join("botspy-fixtures-smoke")
        .join(std::process::id().to_string());
    let _ = std::fs::remove_dir_all(&scratch);
    std::fs::create_dir_all(&scratch).expect("create smoke scratch dir");
    copy_tree(&fixtures, &scratch);
    let fixtures = scratch.clone();

    let claude = ClaudeCodeSource::new(fixtures.join("claude_code/projects"));
    let (sessions, _) = claude.discover();
    assert_eq!(sessions.len(), 3, "Claude Code discovery count");
    let extraction = claude
        .extract(CLAUDE_SESSION_1)
        .expect("no Claude Code session for the synthetic transcript");
    assert!(extraction.good > 0, "Claude Code extraction mapped nothing");

    let codex = CodexSource::new(fixtures.join("codex"));
    let (sessions, discovery) = codex.discover();
    assert_eq!(sessions.len(), 2, "Codex discovery count");
    assert_eq!(
        discovery.parent_links.get("t-synth-2").map(String::as_str),
        Some(CODEX_THREAD_1),
        "Codex spawn edge missing"
    );
    let extraction = codex
        .extract(CODEX_THREAD_1)
        .expect("no Codex rollout for the synthetic thread");
    assert!(extraction.good >= 6, "Codex extraction mapped too little");

    let cursor = CursorSource::new(fixtures.join("cursor/store/state.vscdb"));
    let (sessions, _) = cursor.discover();
    assert_eq!(sessions.len(), 2, "Cursor KV discovery count");
    let extraction = cursor
        .extract(CURSOR_COMPOSER_1)
        .expect("no Cursor composer data for the synthetic composer");
    assert!(extraction.good >= 3, "Cursor extraction mapped too little");

    let cli = CursorCliSource::new(fixtures.join("cursor/cli/projects"));
    let (sessions, _) = cli.discover();
    assert_eq!(sessions.len(), 1, "Cursor CLI discovery count");
    let extraction = cli
        .extract("cli-synth-1")
        .expect("no Cursor CLI session for the synthetic transcript");
    assert!(extraction.good > 0, "Cursor CLI extraction mapped nothing");

    let grok = GrokBotSource::new(fixtures.join("grok_bot/sand-client-persistence"));
    let (sessions, discovery) = grok.discover();
    assert_eq!(sessions.len(), 3, "Grok Bot discovery count");
    assert_eq!(discovery.cloud_agents, 1, "Grok Bot cloud-agent stub count");
    let extraction = grok
        .extract(GROK_BLOB_1)
        .expect("no Grok Bot entry log for the synthetic blob");
    assert!(
        extraction.good >= 2,
        "Grok Bot extraction mapped too little"
    );

    let antigravity = AntigravitySource::new(fixtures.join("antigravity"));
    let (sessions, _) = antigravity.discover();
    assert_eq!(sessions.len(), 2, "Antigravity discovery count");
    let extraction = antigravity
        .extract(ANTIGRAVITY_CONVERSATION_1)
        .expect("no Antigravity transcript for the synthetic conversation");
    assert!(
        extraction.good >= 4,
        "Antigravity extraction mapped too little"
    );
    let _ = std::fs::remove_dir_all(&scratch);
}
