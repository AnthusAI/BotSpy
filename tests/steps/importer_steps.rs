//! Steps for the adapter contract (spec 03_importers/contract.feature).

use crate::steps::{parse_agent, BotSpyWorld};
use botspy::importer::{digest_tree, MemorySource, SkipCounter};
use botspy::RecordStream;
use cucumber::{given, then, when};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicUsize, Ordering};

static UNIQUE_DIR: AtomicUsize = AtomicUsize::new(0);

fn materialize_root(world: &mut BotSpyWorld, given: &str) -> PathBuf {
    let base = std::env::temp_dir()
        .join("botspy-bdd")
        .join(format!(
            "{}-{}",
            std::process::id(),
            UNIQUE_DIR.fetch_add(1, Ordering::SeqCst)
        ))
        .join(given.replace(['/', ':'], "-"));
    std::fs::create_dir_all(&base).expect("create fixture source root");
    world.source_root = Some(base.clone());
    world.memory_source = None;
    world.source_name = None;
    base
}

fn write_transcript(root: &Path, name: &str, lines: &[String]) {
    std::fs::write(
        root.join(name),
        lines
            .iter()
            .map(|line| format!("{line}\n"))
            .collect::<String>(),
    )
    .expect("write fixture transcript");
}

fn record(ty: &str, text: &str) -> String {
    format!(r#"{{"type":"{ty}","text":"{text}"}}"#)
}

/// The reference source for the current root, built on first use so that
/// repeated discoveries observe the same incremental state.
fn source_for<'a>(world: &'a mut BotSpyWorld, source: &str) -> &'a mut MemorySource {
    let built_for_this_source =
        world.source_name.as_deref() == Some(source) && world.memory_source.is_some();
    if !built_for_this_source {
        let root = world
            .source_root
            .clone()
            .expect("no fixture source root was created");
        world.memory_source = Some(MemorySource::new(parse_agent(source), root));
        world.source_name = Some(source.to_string());
    }
    world.memory_source.as_mut().expect("reference source")
}

/// The reference source for the current root, whatever agent it is.
#[given(regex = r#"^a fixture source root "([^"]+)" with transcripts "([^"]+)" and "([^"]+)"$"#)]
fn root_with_transcripts(world: &mut BotSpyWorld, root: String, a: String, b: String) {
    let base = materialize_root(world, &root);
    write_transcript(&base, &a, &[record("user", "hello")]);
    write_transcript(&base, &b, &[record("assistant", "hi")]);
}

#[given(regex = r#"^a fixture source root "([^"]+)" with a transcript of ([0-9]+) records$"#)]
fn root_with_big_transcript(world: &mut BotSpyWorld, root: String, count: usize) {
    let base = materialize_root(world, &root);
    let lines: Vec<String> = (0..count)
        .map(|i| record("user", &format!("line {i}")))
        .collect();
    write_transcript(&base, "big.jsonl", &lines);
}

#[given(
    regex = r#"^a fixture source root "([^"]+)" with a transcript holding ([0-9]+) good records and ([0-9]+) malformed ones$"#
)]
fn root_with_malformed(world: &mut BotSpyWorld, root: String, good: usize, malformed: usize) {
    let base = materialize_root(world, &root);
    let mut lines = Vec::new();
    for i in 0..good {
        lines.push(record("user", &format!("good {i}")));
        if i < malformed {
            lines.push("this line is { not json".to_string());
        }
    }
    write_transcript(&base, "mixed.jsonl", &lines);
}

#[given(
    regex = r#"^a fixture source root "([^"]+)" with a transcript holding ([0-9]+) known records and ([0-9]+) of an unknown type$"#
)]
fn root_with_unknown(world: &mut BotSpyWorld, root: String, known: usize, unknown: usize) {
    let base = materialize_root(world, &root);
    let mut lines = Vec::new();
    for i in 0..known {
        lines.push(record("user", &format!("known {i}")));
        if i < unknown {
            lines.push(record("quantum_flux_sync", "opaque"));
        }
    }
    write_transcript(&base, "unknown.jsonl", &lines);
}

#[when(regex = r#"^I discover sessions from the source "([^"]+)" at that root$"#)]
fn discover_from_source(world: &mut BotSpyWorld, source: String) {
    let discovered = source_for(world, &source).discover();
    let new_ids = world
        .memory_source
        .as_ref()
        .expect("reference source")
        .last_discovery_new_ids();
    world.discovered = discovered;
    world.discovery_rounds.push(new_ids);
}

#[when(regex = r#"^I extract the records of that transcript$"#)]
fn extract_that_transcript(world: &mut BotSpyWorld) {
    let session_id = single_transcript_id(world);
    if world.memory_source.is_none() {
        let root = world
            .source_root
            .clone()
            .expect("no fixture source root was created");
        world.memory_source = Some(MemorySource::new(botspy::Agent::ClaudeCode, root));
    }
    let mut stream = world
        .memory_source
        .as_mut()
        .expect("reference source")
        .stream(&session_id)
        .unwrap_or_else(|| panic!("no transcript for session {session_id}"));
    let mut extracted = Vec::new();
    while let Some(record) = stream.next_record() {
        extracted.push(record);
    }
    let skipped = stream.skipped();
    let peak = stream.peak_buffered();
    drop(stream);
    world.extracted.append(&mut extracted);
    world.skipped = skipped;
    world.peak_buffered = world.peak_buffered.max(peak);
}

#[when(regex = r#"^I extract every record from the source "([^"]+)" at that root$"#)]
fn extract_every_record(world: &mut BotSpyWorld, source: String) {
    let root = world.source_root.clone().expect("no fixture source root");
    world.source_digest_before = Some(digest_tree(&root));
    let mut source = MemorySource::new(parse_agent(&source), root);
    let ids: Vec<String> = source
        .discover()
        .into_iter()
        .map(|summary| summary.id)
        .collect();
    world.extracted.clear();
    world.skipped = SkipCounter::default();
    world.peak_buffered = 0;
    for id in &ids {
        extract_from(&mut source, id, world);
    }
    let root = world.source_root.clone().expect("no fixture source root");
    world.source_digest_after = Some(digest_tree(&root));
}

fn extract_from(source: &mut MemorySource, session_id: &str, world: &mut BotSpyWorld) {
    let mut stream = source
        .stream(session_id)
        .unwrap_or_else(|| panic!("no transcript for session {session_id}"));
    let mut extracted = Vec::new();
    while let Some(record) = stream.next_record() {
        extracted.push(record);
    }
    let skipped = stream.skipped();
    let peak = stream.peak_buffered();
    drop(stream);
    world.extracted.append(&mut extracted);
    world.skipped = skipped;
    world.peak_buffered = world.peak_buffered.max(peak);
}

/// The id of the one transcript the current fixture root holds.
fn single_transcript_id(world: &BotSpyWorld) -> String {
    let root = world.source_root.as_ref().expect("no fixture source root");
    let mut transcripts = Vec::new();
    for entry in std::fs::read_dir(root)
        .expect("read fixture root")
        .flatten()
    {
        let path = entry.path();
        if path.extension().and_then(|ext| ext.to_str()) == Some("jsonl") {
            transcripts.push(
                path.file_stem()
                    .and_then(|stem| stem.to_str())
                    .expect("utf8 stem")
                    .to_string(),
            );
        }
    }
    assert_eq!(
        transcripts.len(),
        1,
        "expected exactly one transcript in the fixture root"
    );
    transcripts.remove(0)
}

#[then(regex = r#"^discovery yields ([0-9]+) sessions$"#)]
fn discovery_yields(world: &mut BotSpyWorld, count: usize) {
    assert_eq!(
        world.discovered.len(),
        count,
        "discovery returned the wrong sessions"
    );
}

#[then(regex = r#"^the second discovery reports no new sessions$"#)]
fn second_discovery_no_new(world: &mut BotSpyWorld) {
    assert!(
        world.discovery_rounds.len() >= 2,
        "discovery did not run twice"
    );
    for (round, new_ids) in world.discovery_rounds.iter().enumerate().skip(1) {
        assert!(
            new_ids.is_empty(),
            "discovery round {round} reported new sessions: {new_ids:?}"
        );
    }
}

#[then(regex = r#"^the records stream one at a time$"#)]
fn records_stream_one_at_a_time(world: &mut BotSpyWorld) {
    assert_eq!(
        world.peak_buffered, 1,
        "extraction buffered more than one record at a time"
    );
}

#[then(regex = r#"^the extractor never holds more than ([0-9]+) records in memory$"#)]
fn extractor_bounded(world: &mut BotSpyWorld, bound: usize) {
    assert!(
        world.peak_buffered <= bound,
        "extraction buffered {} records, bound is {bound}",
        world.peak_buffered
    );
}

#[then(regex = r#"^extraction yields ([0-9]+) good records$"#)]
fn extraction_yields(world: &mut BotSpyWorld, count: usize) {
    assert_eq!(
        world.extracted.len(),
        count,
        "extraction returned the wrong records"
    );
}

#[then(regex = r#"^extraction reports ([0-9]+) skipped records$"#)]
fn extraction_skips(world: &mut BotSpyWorld, count: u64) {
    assert_eq!(
        world.skipped.total(),
        count,
        "skip counters report {:?}",
        world.skipped
    );
}

#[then(regex = r#"^no file under the source root changed$"#)]
fn source_unchanged(world: &mut BotSpyWorld) {
    let before = world.source_digest_before.as_ref().expect("digest before");
    let after = world.source_digest_after.as_ref().expect("digest after");
    assert_eq!(before, after, "the source tree changed during extraction");
}
