//! Steps for the local store (spec 02_querying/04_store.feature and the
//! store specs that follow it in features/02_querying/).

use crate::steps::BotSpyWorld;
use cucumber::{given, then, when};

// 04_store.feature — opening, persistence, concurrent readers, errors.

#[given(regex = r#"^no store exists at "([^"]+)"$"#)]
fn no_store_exists_at(_world: &mut BotSpyWorld, _path: String) {
    todo!("BOTSPY-4df7c3: remove the store file if present")
}

#[when(regex = r#"^I open a store at "([^"]+)"$"#)]
fn open_store_at(_world: &mut BotSpyWorld, _path: String) {
    todo!("BOTSPY-4df7c3: Store::open at a caller-given path")
}

#[then(regex = r#"^the store file exists at "([^"]+)"$"#)]
fn store_file_exists_at(_world: &mut BotSpyWorld, _path: String) {
    todo!("BOTSPY-4df7c3: the store file is on disk")
}

#[given(regex = r#"^fixture sessions "([^"]+)" from "([^"]+)" and "([^"]+)" from "([^"]+)"$"#)]
fn fixture_sessions_two_agents(
    _world: &mut BotSpyWorld,
    _id1: String,
    _agent1: String,
    _id2: String,
    _agent2: String,
) {
    todo!("BOTSPY-4df7c3: two fixture sessions from two agents")
}

#[given(regex = r#"^a store at "([^"]+)" with those sessions ingested$"#)]
fn store_with_sessions_ingested(_world: &mut BotSpyWorld, _path: String) {
    todo!("BOTSPY-4df7c3: open a store and ingest the fixture sessions")
}

#[when(regex = r#"^I reopen a store at "([^"]+)"$"#)]
fn reopen_store_at(_world: &mut BotSpyWorld, _path: String) {
    todo!("BOTSPY-4df7c3: open the same store path again")
}

#[when(regex = r#"^I iterate the store's sessions$"#)]
fn iterate_store_sessions(_world: &mut BotSpyWorld) {
    todo!("BOTSPY-4df7c3: lazy session iteration over the store")
}

#[then(regex = r#"^the store iteration yields ([0-9]+) sessions?$"#)]
fn store_iteration_yields_count(_world: &mut BotSpyWorld, _count: usize) {
    todo!("BOTSPY-4df7c3: count the sessions the store iteration yielded")
}

#[then(regex = r#"^the store iteration yields sessions (.+)$"#)]
fn store_iteration_yields_ids(_world: &mut BotSpyWorld, _ids: String) {
    todo!("BOTSPY-4df7c3: the yielded session ids, in order")
}

#[given(regex = r#"^fixture sessions "([^"]+)", "([^"]+)", and "([^"]+)" from "([^"]+)"$"#)]
fn fixture_sessions_three(
    _world: &mut BotSpyWorld,
    _id1: String,
    _id2: String,
    _id3: String,
    _agent: String,
) {
    todo!("BOTSPY-4df7c3: three fixture sessions from one agent")
}

#[given(
    regex = r#"^a second reader is open on the store at "([^"]+)" and has taken its first session$"#
)]
fn second_reader_took_first(_world: &mut BotSpyWorld, _path: String) {
    todo!("BOTSPY-4df7c3: reader on a second connection mid-iteration")
}

#[when(regex = r#"^a new fixture session "([^"]+)" is ingested through the writer$"#)]
fn ingest_new_session_through_writer(_world: &mut BotSpyWorld, _id: String) {
    todo!("BOTSPY-4df7c3: ingest another session on the writer connection")
}

#[when(regex = r#"^the reader continues its iteration$"#)]
fn reader_continues_iteration(_world: &mut BotSpyWorld) {
    todo!("BOTSPY-4df7c3: the reader keeps pulling from its open cursor")
}

#[then(regex = r#"^the reader finishes its iteration without error$"#)]
fn reader_finishes_without_error(_world: &mut BotSpyWorld) {
    todo!("BOTSPY-4df7c3: the reader's iteration completed cleanly")
}

#[given(regex = r#"^a file "([^"]+)" containing "([^"]+)"$"#)]
fn file_containing(_world: &mut BotSpyWorld, _path: String, _content: String) {
    todo!("BOTSPY-4df7c3: write a non-store file")
}

#[then(regex = r#"^opening fails with a store error, not a panic$"#)]
fn opening_fails_with_store_error(_world: &mut BotSpyWorld) {
    todo!("BOTSPY-4df7c3: typed error for a non-store file")
}

#[given(regex = r#"^BOTSPY_HOME is "([^"]+)"$"#)]
fn botspy_home_is(_world: &mut BotSpyWorld, _home: String) {
    todo!("BOTSPY-4df7c3: point BOTSPY_HOME at a fixture home")
}

#[when(regex = r#"^I open the default store$"#)]
fn open_default_store(_world: &mut BotSpyWorld) {
    todo!("BOTSPY-4df7c3: open the store at the default path")
}
