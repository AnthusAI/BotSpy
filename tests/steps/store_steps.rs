//! Steps for the local store (spec 02_querying/04_store.feature and the
//! store specs that follow it in features/02_querying/).

use crate::steps::{adapter_for, parse_agent, BotSpyWorld};
use botspy::store::Store;
use botspy::{Adapter, Agent, KnownPart, Message, Part, Role, Session};
use cucumber::{given, then, when};
use std::path::{Path, PathBuf};

/// Store fixture paths live under the test target dir, so scenarios never
/// litter the crate and never collide across reruns.
fn store_path(path: &str) -> PathBuf {
    std::env::var_os("CARGO_TARGET_TMPDIR")
        .map(PathBuf::from)
        .unwrap_or_else(std::env::temp_dir)
        .join(path)
}

/// A plain fixture session with one text message.
fn fixture_session(id: &str, agent: Agent, project: &str, last_activity: &str) -> Session {
    Session {
        id: id.to_string(),
        agent,
        project_id: project.to_string(),
        started_at: last_activity.to_string(),
        last_activity_at: last_activity.to_string(),
        messages: vec![Message {
            role: Role::User,
            parts: vec![Part::Known(KnownPart::Text {
                text: format!("hello from {id}"),
                extra: None,
            })],
            timestamp: Some(last_activity.to_string()),
            ..Message::default()
        }],
        ..Session::default()
    }
}

/// The store file plus its WAL sidecars, so a scenario can start clean.
fn store_files(path: &Path) -> Vec<PathBuf> {
    let stem = path
        .file_name()
        .map(|name| name.to_string_lossy().into_owned())
        .unwrap_or_default();
    let mut files = vec![path.to_path_buf()];
    for suffix in ["-wal", "-shm"] {
        files.push(path.with_file_name(format!("{stem}{suffix}")));
    }
    files
}

/// Remove the store file and its WAL sidecars.
fn remove_store_files(path: &str) {
    for candidate in store_files(&store_path(path)) {
        let _ = std::fs::remove_file(candidate);
    }
}

/// Every fixture session known to the world's adapters, in stable order.
fn all_fixture_sessions(world: &BotSpyWorld) -> Vec<Session> {
    let mut sessions = Vec::new();
    for adapter in world.adapters.values() {
        for summary in adapter.discover() {
            if let Some(session) = adapter.open(&summary.id) {
                sessions.push(session);
            }
        }
    }
    sessions
}

// 04_store.feature — opening, persistence, concurrent readers, errors.

#[given(regex = r#"^no store exists at "([^"]+)"$"#)]
fn no_store_exists_at(_world: &mut BotSpyWorld, path: String) {
    remove_store_files(&path);
}

#[when(regex = r#"^I open a store at "([^"]+)"$"#)]
fn open_store_at(world: &mut BotSpyWorld, path: String) {
    world.local_open_error = None;
    match Store::open(store_path(&path)) {
        Ok(store) => world.local_store = Some(store),
        Err(err) => world.local_open_error = Some(err),
    }
}

#[then(regex = r#"^the store file exists at "([^"]+)"$"#)]
fn store_file_exists_at(world: &mut BotSpyWorld, path: String) {
    assert!(world.local_store.is_some(), "no store was opened");
    assert!(
        store_path(&path).exists(),
        "the store file does not exist at {path}"
    );
}

#[given(regex = r#"^fixture sessions "([^"]+)" from "([^"]+)" and "([^"]+)" from "([^"]+)"$"#)]
fn fixture_sessions_two_agents(
    world: &mut BotSpyWorld,
    id1: String,
    agent1: String,
    id2: String,
    agent2: String,
) {
    for (id, agent) in [(&id1, &agent1), (&id2, &agent2)] {
        let adapter = adapter_for(world, agent);
        adapter.add_session(fixture_session(
            id,
            parse_agent(agent),
            "",
            "2026-10-01T09:00:00Z",
        ));
    }
}

#[given(regex = r#"^fixture sessions "([^"]+)", "([^"]+)", and "([^"]+)" from "([^"]+)"$"#)]
fn fixture_sessions_three(
    world: &mut BotSpyWorld,
    id1: String,
    id2: String,
    id3: String,
    agent: String,
) {
    for id in [&id1, &id2, &id3] {
        let adapter = adapter_for(world, &agent);
        adapter.add_session(fixture_session(
            id,
            parse_agent(&agent),
            "",
            "2026-10-01T09:00:00Z",
        ));
    }
}

#[given(regex = r#"^a store at "([^"]+)" with those sessions ingested$"#)]
fn store_with_sessions_ingested(world: &mut BotSpyWorld, path: String) {
    remove_store_files(&path);
    let store = Store::open(store_path(&path)).expect("store opens for ingest");
    store
        .ingest_sessions(all_fixture_sessions(world))
        .expect("fixture sessions ingest");
    world.local_store = Some(store);
}

#[when(regex = r#"^I reopen a store at "([^"]+)"$"#)]
fn reopen_store_at(world: &mut BotSpyWorld, path: String) {
    world.local_open_error = None;
    match Store::open(store_path(&path)) {
        Ok(store) => world.local_store = Some(store),
        Err(err) => world.local_open_error = Some(err),
    }
}

#[when(regex = r#"^I iterate the store's sessions$"#)]
fn iterate_store_sessions(world: &mut BotSpyWorld) {
    let store = world.local_store.as_ref().expect("no store is open");
    world.local_sessions = store.sessions().collect();
}

#[then(regex = r#"^the store iteration yields ([0-9]+) sessions?$"#)]
fn store_iteration_yields_count(world: &mut BotSpyWorld, count: usize) {
    assert_eq!(
        world.local_sessions.len(),
        count,
        "the store iteration yielded the wrong number of sessions"
    );
}

/// The quoted tokens of `"a", "b", and "c"` in order.
fn quoted_list(text: &str) -> Vec<String> {
    text.split('"')
        .skip(1)
        .step_by(2)
        .map(str::to_string)
        .collect()
}

#[then(regex = r#"^the store iteration yields sessions (.+)$"#)]
fn store_iteration_yields_ids(world: &mut BotSpyWorld, ids: String) {
    let expected = quoted_list(&ids);
    let actual: Vec<String> = world
        .local_sessions
        .iter()
        .map(|summary| summary.id.clone())
        .collect();
    assert_eq!(actual, expected, "the store iteration is out of order");
}

#[given(
    regex = r#"^a second reader is open on the store at "([^"]+)" and has taken its first session$"#
)]
fn second_reader_took_first(world: &mut BotSpyWorld, path: String) {
    let reader = Store::open(store_path(&path)).expect("second reader opens");
    let mut iter = reader.sessions();
    let first = iter.next().expect("the reader sees the ingested sessions");
    world.local_reader = Some(reader);
    world.local_reader_iter = Some(iter);
    world.local_sessions = vec![first];
    world.local_reader_finished = false;
}

#[when(regex = r#"^a new fixture session "([^"]+)" is ingested through the writer$"#)]
fn ingest_new_session_through_writer(world: &mut BotSpyWorld, id: String) {
    let agent = "claude_code";
    let adapter = adapter_for(world, agent);
    let session = fixture_session(&id, parse_agent(agent), "", "2026-10-01T12:00:00Z");
    adapter.add_session(session.clone());
    let writer = world.local_store.as_ref().expect("no writer store is open");
    writer
        .ingest_sessions(vec![session])
        .expect("the writer ingests while the reader is mid-iteration");
}

#[when(regex = r#"^the reader continues its iteration$"#)]
fn reader_continues_iteration(world: &mut BotSpyWorld) {
    let iter = world
        .local_reader_iter
        .as_mut()
        .expect("the reader has no open iteration");
    for summary in iter.by_ref() {
        world.local_sessions.push(summary);
    }
    world.local_reader_finished = true;
}

#[then(regex = r#"^the reader finishes its iteration without error$"#)]
fn reader_finishes_without_error(world: &mut BotSpyWorld) {
    assert!(
        world.local_reader_finished,
        "the reader's iteration did not run to completion"
    );
}

#[given(regex = r#"^a file "([^"]+)" containing "([^"]+)"$"#)]
fn file_containing(_world: &mut BotSpyWorld, path: String, content: String) {
    let path = store_path(&path);
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent).expect("parent directory");
    }
    std::fs::write(path, content).expect("write the non-store file");
}

#[then(regex = r#"^opening fails with a store error, not a panic$"#)]
fn opening_fails_with_store_error(world: &mut BotSpyWorld) {
    assert!(
        world.local_store.is_none(),
        "opening a non-store file succeeded"
    );
    let err = world
        .local_open_error
        .as_ref()
        .expect("opening a non-store file should fail");
    assert!(
        matches!(err, botspy::store::StoreError::NotADatabase { .. }),
        "expected a typed not-a-database error, got {err:?}"
    );
}

#[given(regex = r#"^BOTSPY_HOME is "([^"]+)"$"#)]
fn botspy_home_is(world: &mut BotSpyWorld, home: String) {
    world.saved_botspy_home = std::env::var_os("BOTSPY_HOME");
    std::env::set_var("BOTSPY_HOME", store_path(&home));
}

#[when(regex = r#"^I open the default store$"#)]
fn open_default_store(world: &mut BotSpyWorld) {
    world.local_store = Some(Store::open_default().expect("the default store opens"));
    if let Some(previous) = world.saved_botspy_home.take() {
        std::env::set_var("BOTSPY_HOME", previous);
    } else {
        std::env::remove_var("BOTSPY_HOME");
    }
}

// 05_ingest.feature — adapter-driven ingestion into the store (red until
// BOTSPY-a48fb1), plus the 01_iteration.feature steps (red until
// BOTSPY-a927fc): fixture adapters → store → ingest → iterate.

#[given(regex = r#"^a store at "([^"]+)"$"#)]
fn a_store_at(world: &mut BotSpyWorld, path: String) {
    remove_store_files(&path);
    let file = store_path(&path);
    world.local_store = Some(Store::open(&file).expect("store opens"));
}

#[when(regex = r#"^I ingest the registered adapters into the store$"#)]
fn ingest_registered_adapters(world: &mut BotSpyWorld) {
    let store = world.local_store.as_ref().expect("a store");
    let adapters: Vec<std::sync::Arc<dyn Adapter>> = world
        .adapters
        .values()
        .map(|adapter| {
            let owned: std::sync::Arc<dyn Adapter> = adapter.clone();
            owned
        })
        .collect();
    world.ingest_report = Some(store.ingest(&adapters).expect("ingest succeeds"));
}

#[then(regex = r#"^the ingest reports ([0-9]+) new sessions?$"#)]
fn ingest_reports_new_sessions(world: &mut BotSpyWorld, count: usize) {
    let report = world.ingest_report.expect("an ingest report");
    assert_eq!(report.new, count, "ingest report new-session count");
}

#[given(regex = r#"^fixture session "([^"]+)" from "([^"]+)" and "([^"]+)" from "([^"]+)"$"#)]
fn fixture_session_two_adapters(
    world: &mut BotSpyWorld,
    id1: String,
    agent1: String,
    id2: String,
    agent2: String,
) {
    adapter_for(world, &agent1).add_session(fixture_session(
        &id1,
        parse_agent(&agent1),
        "demo",
        "2026-10-01T09:00:00Z",
    ));
    adapter_for(world, &agent2).add_session(fixture_session(
        &id2,
        parse_agent(&agent2),
        "demo",
        "2026-10-01T09:00:00Z",
    ));
}

#[when(regex = r#"^I open session "([^"]+)" from the store$"#)]
fn open_session_from_store(world: &mut BotSpyWorld, id: String) {
    let store = world.local_store.as_ref().expect("a store");
    world.local_opened = Some(store.open_session(&id));
}

#[then(regex = r#"^it equals the session the adapter returns$"#)]
fn opened_equals_adapter_session(world: &mut BotSpyWorld) {
    let opened = world
        .local_opened
        .as_ref()
        .expect("an opened session")
        .as_ref()
        .expect("opening succeeds");
    let id = world.current_session.as_deref().expect("a current session");
    let expected = crate::steps::find_session(world, id).expect("fixture session");
    assert_eq!(opened, &expected, "lossless round-trip");
}

fn adapters_sessions_digest(world: &BotSpyWorld) -> String {
    use sha2::{Digest, Sha256};
    let mut hasher = Sha256::new();
    for adapter in world.adapters.values() {
        for summary in adapter.discover() {
            if let Some(session) = adapter.open(&summary.id) {
                let json = serde_json::to_string(&session).expect("session serializes");
                hasher.update(json.as_bytes());
            }
        }
    }
    format!("{:x}", hasher.finalize())
}

#[when(regex = r#"^I snapshot the adapters' sessions digest$"#)]
fn snapshot_sessions_digest(world: &mut BotSpyWorld) {
    world.local_digest_before = Some(adapters_sessions_digest(world));
}

#[when(regex = r#"^I snapshot the adapters' sessions digest again$"#)]
fn snapshot_sessions_digest_again(world: &mut BotSpyWorld) {
    world.local_digest_after = Some(adapters_sessions_digest(world));
}

#[then(regex = r#"^the sources are unchanged by the ingest$"#)]
fn sources_unchanged_by_ingest(world: &mut BotSpyWorld) {
    let before = world.local_digest_before.as_ref().expect("digest before");
    let after = world.local_digest_after.as_ref().expect("digest after");
    assert_eq!(before, after, "sources must stay read-only");
}

// 01_iteration.feature — cross-source iteration through the store, plus
// 03_laziness.feature — streaming discipline, proven with the query
// counters (the importer protocol's peak_buffered/SkipCounter precedent).

/// A store holding every fixture session so far, opened on first use: the
/// iteration specs describe behavior, the harness hides the plumbing. The
/// path is derived from the registered adapters so parallel scenarios with
/// different fixtures never collide.
fn ensure_store_ingested(world: &mut BotSpyWorld) {
    if world.local_store.is_some() {
        return;
    }
    let key: String = world.adapters.keys().cloned().collect::<Vec<_>>().join("-");
    let path = format!("iteration/{key}.db");
    remove_store_files(&path);
    let store = Store::open(store_path(&path)).expect("store opens");
    store
        .ingest_sessions(all_fixture_sessions(world))
        .expect("fixture sessions ingest");
    world.local_store = Some(store);
}

#[given(
    regex = r#"^fixture sessions "([^"]+)" from "([^"]+)" and "([^"]+)" from "([^"]+)" and "([^"]+)" from "([^"]+)"$"#
)]
#[allow(clippy::too_many_arguments)]
fn fixture_sessions_three_agents(
    world: &mut BotSpyWorld,
    id1: String,
    agent1: String,
    id2: String,
    agent2: String,
    id3: String,
    agent3: String,
) {
    for (id, agent) in [(&id1, &agent1), (&id2, &agent2), (&id3, &agent3)] {
        let adapter = adapter_for(world, agent);
        adapter.add_session(fixture_session(
            id,
            parse_agent(agent),
            "demo",
            "2026-10-01T09:00:00Z",
        ));
    }
}

#[when(regex = r#"^I iterate the sessions$"#)]
fn iterate_the_sessions(world: &mut BotSpyWorld) {
    ensure_store_ingested(world);
    let store = world.local_store.as_ref().expect("no store is open");
    let query = store.query();
    world.local_sessions = query.sessions().collect();
    world.local_query_counters = Some(query.counters());
}

#[then(regex = r#"^the iteration yields ([0-9]+) sessions?$"#)]
fn iteration_yields_count(world: &mut BotSpyWorld, count: usize) {
    assert_eq!(
        world.local_sessions.len(),
        count,
        "the session iteration yielded the wrong number of sessions"
    );
}

#[then(regex = r#"^the iteration yields sessions (.+)$"#)]
fn iteration_yields_ids(world: &mut BotSpyWorld, ids: String) {
    let expected = quoted_list(&ids);
    let actual: Vec<String> = world
        .local_sessions
        .iter()
        .map(|summary| summary.id.clone())
        .collect();
    assert_eq!(actual, expected, "the session iteration is out of order");
}

#[given(regex = r#"^the session has messages "([^"]+)", "([^"]+)", and "([^"]+)"$"#)]
fn session_has_messages(world: &mut BotSpyWorld, text1: String, text2: String, text3: String) {
    let mut session = crate::steps::current_session(world);
    for text in [text1, text2, text3] {
        session.messages.push(Message {
            role: Role::User,
            parts: vec![Part::Known(KnownPart::Text { text, extra: None })],
            timestamp: Some("2026-10-01T09:00:00Z".to_string()),
            ..Message::default()
        });
    }
    crate::steps::save_session(world, session);
}

#[when(regex = r#"^I iterate the messages of session "([^"]+)"$"#)]
fn iterate_messages_of_session(world: &mut BotSpyWorld, id: String) {
    ensure_store_ingested(world);
    let store = world.local_store.as_ref().expect("no store is open");
    let query = store.query();
    world.local_messages = query.messages(&id).collect();
    world.local_query_counters = Some(query.counters());
}

fn first_text(message: &Message) -> String {
    message
        .parts
        .iter()
        .find_map(|part| match part {
            Part::Known(KnownPart::Text { text, .. }) => Some(text.clone()),
            _ => None,
        })
        .expect("the message has a text part")
}

#[then(regex = r#"^the iteration yields the texts (.+) in that order$"#)]
fn iteration_yields_texts(world: &mut BotSpyWorld, texts: String) {
    let expected = quoted_list(&texts);
    let actual: Vec<String> = world.local_messages.iter().map(first_text).collect();
    assert_eq!(actual, expected, "the message iteration is out of order");
}

#[given(regex = r#"^the session has a message with parts "([^"]+)", "([^"]+)", and "([^"]+)"$"#)]
fn session_has_message_with_parts(
    world: &mut BotSpyWorld,
    kind1: String,
    kind2: String,
    kind3: String,
) {
    let part = |kind: &str| {
        Part::Known(match kind {
            "text" => KnownPart::Text {
                text: "a text part".to_string(),
                extra: None,
            },
            "thinking" => KnownPart::Thinking {
                text: Some("a thinking part".to_string()),
                signature: None,
                encrypted: None,
                extra: None,
            },
            "tool_call" => KnownPart::ToolCall {
                id: "call_1".to_string(),
                name: "read_file".to_string(),
                arguments: None,
                status: None,
                extra: None,
            },
            other => panic!("unsupported part kind in feature: {other}"),
        })
    };
    let mut session = crate::steps::current_session(world);
    session.messages.push(Message {
        role: Role::Assistant,
        parts: vec![part(&kind1), part(&kind2), part(&kind3)],
        timestamp: Some("2026-10-01T09:00:00Z".to_string()),
        ..Message::default()
    });
    crate::steps::save_session(world, session);
}

#[when(regex = r#"^I iterate the parts of the last message of session "([^"]+)"$"#)]
fn iterate_parts_of_last_message(world: &mut BotSpyWorld, id: String) {
    ensure_store_ingested(world);
    let store = world.local_store.as_ref().expect("no store is open");
    let query = store.query();
    let message_count = query.messages(&id).count();
    let last_ordinal = message_count - 1;
    world.local_parts = query
        .parts(&id, last_ordinal)
        .expect("the last message exists")
        .collect();
    world.local_query_counters = Some(query.counters());
}

#[then(regex = r#"^the iteration yields the kinds (.+) in that order$"#)]
fn iteration_yields_kinds(world: &mut BotSpyWorld, kinds: String) {
    let expected = quoted_list(&kinds);
    let actual: Vec<String> = world
        .local_parts
        .iter()
        .map(|part| part.kind_name().unwrap_or("extra").to_string())
        .collect();
    assert_eq!(actual, expected, "the parts iteration is out of order");
}

// 03_laziness.feature — only what was consumed was touched.

#[when(regex = r#"^I take the first ([0-9]+) sessions? from the iteration$"#)]
fn take_first_sessions(world: &mut BotSpyWorld, count: usize) {
    ensure_store_ingested(world);
    let store = world.local_store.as_ref().expect("no store is open");
    let (summaries, opened_id, counters) = {
        let query = store.query();
        let mut summaries = Vec::new();
        let mut opened_id = None;
        for summary in query.sessions().take(count) {
            let session = query.open(&summary.id).expect("the session opens");
            opened_id = Some(session.id);
            summaries.push(summary);
        }
        (summaries, opened_id, query.counters())
    };
    world.local_sessions = summaries;
    world.local_opened_id = opened_id;
    world.local_query_counters = Some(counters);
}

#[when(regex = r#"^I iterate ([0-9]+) sessions? and drop the iterator$"#)]
fn iterate_and_drop(world: &mut BotSpyWorld, count: usize) {
    ensure_store_ingested(world);
    let store = world.local_store.as_ref().expect("no store is open");
    let (summaries, opened_id, counters) = {
        let query = store.query();
        let mut summaries = Vec::new();
        let mut opened_id = None;
        for summary in query.sessions().take(count) {
            let session = query.open(&summary.id).expect("the session opens");
            opened_id = Some(session.id);
            summaries.push(summary);
        }
        (summaries, opened_id, query.counters())
    };
    world.local_sessions = summaries;
    world.local_opened_id = opened_id;
    world.local_query_counters = Some(counters);
}

#[then(regex = r#"^only session "([^"]+)" was opened$"#)]
fn only_session_was_opened(world: &mut BotSpyWorld, id: String) {
    let counters = world.local_query_counters.expect("query counters");
    assert_eq!(counters.opens, 1, "exactly one session was opened");
    assert_eq!(
        world.local_opened_id.as_deref(),
        Some(id.as_str()),
        "the opened session"
    );
}

#[then(regex = r#"^the store performed only ([0-9]+) session opens?$"#)]
fn store_performed_session_opens(world: &mut BotSpyWorld, count: usize) {
    let counters = world.local_query_counters.expect("query counters");
    assert_eq!(counters.opens, count, "session materializations");
}

#[given(regex = r#"^a fixture session "([^"]+)" from "([^"]+)" with ([0-9]+) messages$"#)]
fn fixture_session_with_n_messages(
    world: &mut BotSpyWorld,
    id: String,
    agent: String,
    count: usize,
) {
    let messages = (0..count)
        .map(|ordinal| Message {
            role: if ordinal % 2 == 0 {
                Role::User
            } else {
                Role::Assistant
            },
            parts: vec![Part::Known(KnownPart::Text {
                text: format!("message {ordinal}"),
                extra: None,
            })],
            timestamp: Some("2026-10-01T09:00:00Z".to_string()),
            ..Message::default()
        })
        .collect();
    let session = Session {
        id: id.clone(),
        agent: parse_agent(&agent),
        project_id: "demo".to_string(),
        started_at: "2026-10-01T09:00:00Z".to_string(),
        last_activity_at: "2026-10-01T09:00:00Z".to_string(),
        messages,
        ..Session::default()
    };
    adapter_for(world, &agent).add_session(session);
    world.current_session = Some(id);
}

#[when(regex = r#"^I iterate the first ([0-9]+) messages of session "([^"]+)"$"#)]
fn iterate_first_messages(world: &mut BotSpyWorld, count: usize, id: String) {
    ensure_store_ingested(world);
    let store = world.local_store.as_ref().expect("no store is open");
    let query = store.query();
    world.local_messages = query.messages(&id).take(count).collect();
    world.local_query_counters = Some(query.counters());
}

#[then(regex = r#"^the iteration yields ([0-9]+) messages$"#)]
fn iteration_yields_messages(world: &mut BotSpyWorld, count: usize) {
    assert_eq!(
        world.local_messages.len(),
        count,
        "the message iteration yielded the wrong number of messages"
    );
}

#[then(regex = r#"^the session was not fully materialized$"#)]
fn session_not_fully_materialized(world: &mut BotSpyWorld) {
    let counters = world.local_query_counters.expect("query counters");
    let expected = world.local_messages.len();
    assert_eq!(
        counters.rows_fetched, expected,
        "only the consumed messages were read from the store"
    );
}
