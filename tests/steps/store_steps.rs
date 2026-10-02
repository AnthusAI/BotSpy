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
