//! Steps for the Codex importer (spec 03_importers/codex/adapter.feature).

use crate::steps::BotSpyWorld;
use botspy::adapters::codex::{CodexDiscovery, CodexSource};
use cucumber::{given, then, when};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicUsize, Ordering};

static UNIQUE_DIR: AtomicUsize = AtomicUsize::new(0);

fn new_root(world: &mut BotSpyWorld) -> PathBuf {
    let base = std::env::temp_dir().join("botspy-cx").join(format!(
        "{}-{}",
        std::process::id(),
        UNIQUE_DIR.fetch_add(1, Ordering::SeqCst)
    ));
    std::fs::create_dir_all(&base).expect("create Codex home root");
    world.cx_root = Some(base.clone());
    base
}

fn write_threads_index(root: &Path, rows: &[(&str, &str, Option<&str>, &str)]) {
    let conn =
        rusqlite::Connection::open(root.join("state_5.sqlite")).expect("open Codex threads index");
    conn.execute(
        "CREATE TABLE threads (id TEXT PRIMARY KEY, rollout_path TEXT NOT NULL, parent_id TEXT, cwd TEXT)",
        [],
    )
    .expect("create threads table");
    for (id, rollout_path, parent_id, cwd) in rows {
        conn.execute(
            "INSERT INTO threads (id, rollout_path, parent_id, cwd) VALUES (?1, ?2, ?3, ?4)",
            rusqlite::params![id, rollout_path, parent_id, cwd],
        )
        .expect("insert thread row");
    }
}

fn write_rollout(root: &Path, path: &str, lines: &[String], partial: Option<&str>) {
    let full = root.join(path);
    std::fs::create_dir_all(full.parent().expect("rollout parent dir"))
        .expect("create rollout dirs");
    let mut body: String = lines.iter().map(|line| format!("{line}\n")).collect();
    if let Some(partial_line) = partial {
        body.push_str(partial_line);
    }
    std::fs::write(full, body).expect("write Codex rollout");
}

fn response_message(ordinal: u64, timestamp: &str, text: &str) -> String {
    format!(
        r#"{{"ordinal":{ordinal},"timestamp":"{timestamp}","type":"response_item","payload":{{"type":"message","id":"m{ordinal}","role":"user","content":[{{"type":"input_text","text":"{text}"}}]}}}}"#
    )
}

fn finish_setup(world: &mut BotSpyWorld, thread: String) {
    let root = world.cx_root.clone().expect("no Codex home root");
    world.cx_thread = Some(thread);
    world.cx_source = Some(CodexSource::new(root));
}

const ROLLOUT: &str = "sessions/2026/10/01/rollout-t1.jsonl";

#[given(regex = r#"^a Codex threads index with ([0-9]+) threads pointing at rollout files$"#)]
fn threads_index(world: &mut BotSpyWorld, count: usize) {
    let root = new_root(world);
    let rows: Vec<(String, String, Option<&str>, &str)> = (1..=count)
        .map(|i| {
            (
                format!("t{i}"),
                format!("sessions/2026/10/01/rollout-t{i}.jsonl"),
                None,
                "/repo/demo",
            )
        })
        .collect();
    let row_refs: Vec<(&str, &str, Option<&str>, &str)> = rows
        .iter()
        .map(|(id, path, parent, cwd)| (id.as_str(), path.as_str(), *parent, *cwd))
        .collect();
    write_threads_index(&root, &row_refs);
    for (id, path, _, _) in &rows {
        write_rollout(
            &root,
            path,
            &[response_message(
                1,
                "2026-10-01T09:00:00Z",
                &format!("thread {id}"),
            )],
            None,
        );
    }
    world.cx_source = Some(CodexSource::new(root));
}

#[given(regex = r#"^a Codex threads index where thread "([^"]+)" is spawned by thread "([^"]+)"$"#)]
fn threads_with_spawn_edge(world: &mut BotSpyWorld, child: String, parent: String) {
    let root = new_root(world);
    write_threads_index(
        &root,
        &[
            (
                parent.as_str(),
                "sessions/2026/10/01/rollout-t1.jsonl",
                None,
                "/repo/demo",
            ),
            (
                child.as_str(),
                "sessions/2026/10/01/rollout-t2.jsonl",
                Some(parent.as_str()),
                "/repo/demo",
            ),
        ],
    );
    write_rollout(
        &root,
        "sessions/2026/10/01/rollout-t1.jsonl",
        &[response_message(1, "2026-10-01T09:00:00Z", "parent work")],
        None,
    );
    write_rollout(
        &root,
        "sessions/2026/10/01/rollout-t2.jsonl",
        &[response_message(1, "2026-10-01T09:00:05Z", "child work")],
        None,
    );
    world.cx_source = Some(CodexSource::new(root));
}

#[given(
    regex = r#"^a Codex rollout with records "ordinal ([0-9]+)", "ordinal ([0-9]+)", and "ordinal ([0-9]+)"$"#
)]
fn rollout_with_shuffled_ordinals(world: &mut BotSpyWorld, first: u64, second: u64, third: u64) {
    let root = new_root(world);
    write_threads_index(&root, &[("t1", ROLLOUT, None, "/repo/demo")]);
    write_rollout(
        &root,
        ROLLOUT,
        &[
            response_message(first, "2026-10-01T09:00:15Z", "written first"),
            response_message(second, "2026-10-01T09:00:05Z", "written second"),
            response_message(third, "2026-10-01T09:00:10Z", "written third"),
        ],
        None,
    );
    finish_setup(world, "t1".to_string());
}

#[given(
    regex = r#"^a Codex rollout with ([0-9]+) records and a stored offset after record ([0-9]+)$"#
)]
fn rollout_with_stored_offset(world: &mut BotSpyWorld, count: usize, after: usize) {
    let root = new_root(world);
    write_threads_index(&root, &[("t1", ROLLOUT, None, "/repo/demo")]);
    let lines: Vec<String> = (1..=count)
        .map(|i| response_message(i as u64, "2026-10-01T09:00:00Z", &format!("record {i}")))
        .collect();
    write_rollout(&root, ROLLOUT, &lines, None);
    let offset: usize = lines.iter().take(after).map(|line| line.len() + 1).sum();
    finish_setup(world, "t1".to_string());
    world
        .cx_source
        .as_ref()
        .expect("Codex source")
        .set_offset("t1", offset as u64);
}

#[given(
    regex = r#"^a Codex rollout holding "response_item" records with roles "user", "assistant", "developer", a reasoning summary, and a tool call with output$"#
)]
fn rollout_with_response_items(world: &mut BotSpyWorld) {
    let root = new_root(world);
    write_threads_index(&root, &[("t1", ROLLOUT, None, "/repo/demo")]);
    write_rollout(
        &root,
        ROLLOUT,
        &[
            response_message(1, "2026-10-01T09:00:00Z", "run the tests"),
            r#"{"ordinal":2,"timestamp":"2026-10-01T09:00:05Z","type":"response_item","payload":{"type":"message","role":"assistant","content":[{"type":"output_text","text":"On it."}]}}"#.to_string(),
            r#"{"ordinal":3,"timestamp":"2026-10-01T09:00:06Z","type":"response_item","payload":{"type":"message","role":"developer","content":[{"type":"input_text","text":"Be concise."}]}}"#.to_string(),
            r#"{"ordinal":4,"timestamp":"2026-10-01T09:00:07Z","type":"response_item","payload":{"type":"reasoning","summary":[{"type":"summary_text","text":"thinking it through"}]}}"#.to_string(),
            r#"{"ordinal":5,"timestamp":"2026-10-01T09:00:08Z","type":"response_item","payload":{"type":"function_call","call_id":"call_1","name":"shell","arguments":"{\"cmd\":[\"cargo\",\"test\"]}"}}"#.to_string(),
            r#"{"ordinal":6,"timestamp":"2026-10-01T09:00:20Z","type":"response_item","payload":{"type":"function_call_output","call_id":"call_1","output":"all tests pass"}}"#.to_string(),
        ],
        None,
    );
    finish_setup(world, "t1".to_string());
}

#[given(regex = r#"^a Codex rollout holding a "token_usage_record" and a "compacted" record$"#)]
fn rollout_with_usage_and_compaction(world: &mut BotSpyWorld) {
    let root = new_root(world);
    write_threads_index(&root, &[("t1", ROLLOUT, None, "/repo/demo")]);
    write_rollout(
        &root,
        ROLLOUT,
        &[
            r#"{"ordinal":1,"type":"token_usage_record","payload":{"input_tokens":1200,"cached_input_tokens":300,"output_tokens":800,"reasoning_tokens":150,"model":"gpt-5-codex"}}"#.to_string(),
            r#"{"ordinal":2,"type":"compacted","payload":{"window_id":"w2","previous_window":"w1","retained_context":"summary so far"}}"#.to_string(),
        ],
        None,
    );
    finish_setup(world, "t1".to_string());
}

#[given(
    regex = r#"^a Codex rollout holding a "task_complete" record with duration_ms ([0-9]+) and time_to_first_token_ms ([0-9]+)$"#
)]
fn rollout_with_task_complete(
    world: &mut BotSpyWorld,
    duration_ms: u64,
    time_to_first_token_ms: u64,
) {
    let root = new_root(world);
    write_threads_index(&root, &[("t1", ROLLOUT, None, "/repo/demo")]);
    write_rollout(
        &root,
        ROLLOUT,
        &[format!(
            r#"{{"ordinal":1,"type":"task_complete","payload":{{"turn_id":"turn-1","duration_ms":{duration_ms},"time_to_first_token_ms":{time_to_first_token_ms}}}}}"#
        )],
        None,
    );
    finish_setup(world, "t1".to_string());
}

#[given(
    regex = r#"^a Codex rollout holding a malformed line and ending in a partial trailing line$"#
)]
fn rollout_with_malformed_and_partial(world: &mut BotSpyWorld) {
    let root = new_root(world);
    write_threads_index(&root, &[("t1", ROLLOUT, None, "/repo/demo")]);
    write_rollout(
        &root,
        ROLLOUT,
        &["this line is { not json".to_string()],
        Some(
            r#"{"ordinal":2,"type":"response_item","payload":{"type":"message","role":"user","content":"cut off"#,
        ),
    );
    finish_setup(world, "t1".to_string());
}

#[when(regex = r#"^the importer detects changes$"#)]
fn detect_changes(world: &mut BotSpyWorld) {
    let source = world.cx_source.as_ref().expect("no Codex source");
    let (summaries, discovery) = source.discover();
    world.discovered = summaries;
    world.cx_discovery = Some(discovery);
}

#[when(regex = r#"^the importer extracts the rollout(?: from the stored offset)?$"#)]
fn extract_rollout(world: &mut BotSpyWorld) {
    let source = world.cx_source.as_ref().expect("no Codex source");
    let thread = world.cx_thread.as_deref().expect("no Codex thread");
    let extraction = source
        .extract(thread)
        .expect("no rollout for the Codex thread");
    world.skipped = extraction.skipped.clone();
    world.cx_extraction = Some(extraction);
}

#[then(regex = r#"^session "([^"]+)" links to parent session "([^"]+)"$"#)]
fn session_links_to_parent(world: &mut BotSpyWorld, child: String, parent: String) {
    let discovery: &CodexDiscovery = world
        .cx_discovery
        .as_ref()
        .expect("no Codex discovery result");
    assert_eq!(
        discovery.parent_links.get(&child).map(String::as_str),
        Some(parent.as_str()),
        "spawn edge {child} -> {parent} missing"
    );
}

#[then(regex = r#"^the normalized messages appear in ordinal order ([0-9]+(?:, [0-9]+)*)$"#)]
fn messages_in_ordinal_order(world: &mut BotSpyWorld, expected: String) {
    let expected_ordinals: Vec<u64> = expected
        .split(", ")
        .map(|part| part.parse().expect("ordinal number"))
        .collect();
    let extraction = world.cx_extraction.as_ref().expect("no Codex extraction");
    let ordinals: Vec<u64> = extraction
        .session
        .messages
        .iter()
        .map(|message| {
            message
                .provenance
                .as_ref()
                .and_then(|prov| prov.ordinal)
                .expect("message provenance ordinal")
        })
        .collect();
    assert_eq!(
        ordinals, expected_ordinals,
        "messages did not follow the explicit ordinal order"
    );
}

#[then(regex = r#"^extraction yields ([0-9]+) more good records?$"#)]
fn extraction_yields_good(world: &mut BotSpyWorld, count: usize) {
    let extraction = world.cx_extraction.as_ref().expect("no Codex extraction");
    assert_eq!(
        extraction.good, count,
        "extraction mapped the wrong records"
    );
}

#[then(
    regex = r#"^the normalized messages map the roles, reasoning kinds, and tool call/result kinds$"#
)]
fn messages_map_part_kinds(world: &mut BotSpyWorld) {
    let extraction = world.cx_extraction.as_ref().expect("no Codex extraction");
    let messages = &extraction.session.messages;
    assert_eq!(messages.len(), 6, "wrong number of normalized messages");
    assert_eq!(messages[0].role, botspy::Role::User);
    assert_eq!(messages[1].role, botspy::Role::Assistant);
    assert_eq!(messages[2].role, botspy::Role::System);
    assert_eq!(messages[2].origin, Some(botspy::Origin::Developer));
    assert!(
        matches!(
            messages[3].parts.first(),
            Some(botspy::Part::Known(botspy::KnownPart::Thinking { .. }))
        ),
        "the reasoning summary did not map to a thinking part"
    );
    assert!(
        matches!(
            messages[4].parts.first(),
            Some(botspy::Part::Known(botspy::KnownPart::ToolCall { .. }))
        ),
        "the function call did not map to a tool call part"
    );
    assert!(
        matches!(
            messages[5].parts.first(),
            Some(botspy::Part::Known(botspy::KnownPart::ToolResult { .. }))
        ),
        "the function output did not map to a tool result part"
    );
}

#[then(regex = r#"^the session usage carries the token counts$"#)]
fn session_usage_carries_tokens(world: &mut BotSpyWorld) {
    let extraction = world.cx_extraction.as_ref().expect("no Codex extraction");
    let usage = extraction
        .session
        .usage
        .as_ref()
        .expect("the token_usage_record did not land in session usage");
    assert_eq!(usage.input_tokens, Some(1200));
    assert_eq!(usage.cached_input_tokens, Some(300));
    assert_eq!(usage.output_tokens, Some(800));
    assert_eq!(usage.reasoning_tokens, Some(150));
    assert_eq!(usage.model.as_deref(), Some("gpt-5-codex"));
}

#[then(regex = r#"^the compaction window carries the previous window and the retained context$"#)]
fn compaction_window_carries_boundary(world: &mut BotSpyWorld) {
    let extraction = world.cx_extraction.as_ref().expect("no Codex extraction");
    let window = extraction
        .session
        .compaction_windows
        .get("w2")
        .expect("the compacted record did not land in the window chain");
    assert_eq!(window.previous_window.as_deref(), Some("w1"));
    assert_eq!(window.retained_context.as_deref(), Some("summary so far"));
}

#[then(regex = r#"^the turn has duration_ms ([0-9]+) and time_to_first_token_ms ([0-9]+)$"#)]
fn turn_has_timing(world: &mut BotSpyWorld, duration_ms: u64, time_to_first_token_ms: u64) {
    let extraction = world.cx_extraction.as_ref().expect("no Codex extraction");
    let turn = extraction
        .session
        .turns
        .get("turn-1")
        .expect("the task_complete record did not land in the turn registry");
    assert_eq!(turn.duration_ms, Some(duration_ms));
    assert_eq!(turn.time_to_first_token_ms, Some(time_to_first_token_ms));
}

#[then(regex = r#"^no partial or malformed record is normalized$"#)]
fn no_partial_or_malformed_normalized(world: &mut BotSpyWorld) {
    let extraction = world.cx_extraction.as_ref().expect("no Codex extraction");
    assert_eq!(extraction.good, 0, "a skipped record was normalized");
    assert_eq!(extraction.pending_partial, 1);
}
