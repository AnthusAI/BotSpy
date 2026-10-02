//! The importer layer: the typed adapter protocol every source obeys.
//!
//! Discovery finds sessions under the given roots; extraction streams
//! records off disk one at a time without loading whole transcripts into
//! memory (real transcripts exceed 100 MB). Malformed lines and records
//! of unknown shape are skipped and counted — never fatal. Sources are
//! read-only: the protocol has no surface for writing to them.

use crate::schema::Agent;
use crate::session::SessionSummary;
use serde_json::Value;
use std::cell::RefCell;
use std::collections::BTreeSet;
use std::fs::File;
use std::io::{BufRead, BufReader};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicUsize, Ordering};

/// Records skipped during extraction, by reason.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct SkipCounter {
    /// Lines that did not parse as records at all.
    pub malformed: u64,
    /// Records that parsed but carry an unknown record type.
    pub unknown: u64,
}

impl SkipCounter {
    /// Total records skipped for any reason.
    pub fn total(&self) -> u64 {
        self.malformed + self.unknown
    }
}

/// One raw record streamed off a source, before normalization.
#[derive(Debug, Clone, PartialEq)]
pub struct RawRecord {
    /// The transcript (session) the record came from.
    pub session_id: String,
    /// The record payload as parsed JSON.
    pub payload: Value,
}

/// Streaming extraction over one transcript. Records are pulled one at a
/// time; skipped records never surface and are counted instead.
pub trait RecordStream {
    /// Pull the next good record; `None` when the transcript is exhausted.
    fn next_record(&mut self) -> Option<RawRecord>;

    /// Records skipped so far, by reason.
    fn skipped(&self) -> SkipCounter;

    /// Peak number of records held in memory at once. A streaming
    /// extractor stays bounded no matter how long the transcript is.
    fn peak_buffered(&self) -> usize;
}

/// Record types the reference source understands. Anything else parses
/// but is skipped and counted as unknown.
const KNOWN_RECORD_TYPES: [&str; 3] = ["user", "assistant", "system"];

/// Content digest over every file under `root` (recursive, path-tagged).
/// Used to prove a source was not mutated.
pub fn digest_tree(root: &Path) -> String {
    use sha2::Digest;
    let mut hasher = sha2::Sha256::new();
    fn walk(root: &Path, dir: &Path, hasher: &mut sha2::Sha256) {
        let mut entries: Vec<PathBuf> = std::fs::read_dir(dir)
            .unwrap_or_else(|err| panic!("read {}: {err}", dir.display()))
            .flatten()
            .map(|entry| entry.path())
            .collect();
        entries.sort();
        for path in entries {
            let rel = path
                .strip_prefix(root)
                .expect("path under root")
                .to_string_lossy();
            hasher.update(rel.as_bytes());
            hasher.update([0]);
            if path.is_dir() {
                walk(root, &path, hasher);
            } else {
                hasher.update(
                    std::fs::read(&path)
                        .unwrap_or_else(|err| panic!("read {}: {err}", path.display())),
                );
                hasher.update([0]);
            }
        }
    }
    walk(root, root, &mut hasher);
    hex(&hasher.finalize())
}

fn hex(bytes: &[u8]) -> String {
    bytes.iter().map(|byte| format!("{byte:02x}")).collect()
}

/// The reference in-memory source: a directory of JSONL transcripts read
/// line by line. It never writes to the source root, and it streams —
/// the whole file is never held in memory.
#[derive(Debug)]
pub struct MemorySource {
    agent: Agent,
    root: PathBuf,
    /// How many times a transcript has been opened (for laziness proofs).
    opens: AtomicUsize,
    /// Session ids already reported by discovery, for incremental runs.
    reported: RefCell<BTreeSet<String>>,
    /// Ids new in the most recent discovery call.
    last_new_ids: RefCell<Vec<String>>,
}

impl MemorySource {
    /// A source over the JSONL transcripts under `root`.
    pub fn new(agent: Agent, root: impl Into<PathBuf>) -> Self {
        Self {
            agent,
            root: root.into(),
            opens: AtomicUsize::new(0),
            reported: RefCell::new(BTreeSet::new()),
            last_new_ids: RefCell::new(Vec::new()),
        }
    }

    /// Sessions found under the root, one per `*.jsonl` transcript, with
    /// the ids that were not reported by an earlier discovery marked new.
    pub fn discover(&self) -> Vec<SessionSummary> {
        let mut summaries = Vec::new();
        let mut new_ids = Vec::new();
        let mut reported = self.reported.borrow_mut();
        for entry in sorted_jsonl(&self.root) {
            let id = session_id_of(&entry);
            if reported.insert(id.clone()) {
                new_ids.push(id.clone());
            }
            summaries.push(SessionSummary {
                id,
                agent: self.agent,
                ..SessionSummary::default()
            });
        }
        *self.last_new_ids.borrow_mut() = new_ids;
        summaries
    }

    /// Session ids that the most recent discovery saw for the first time.
    pub fn last_discovery_new_ids(&self) -> Vec<String> {
        self.last_new_ids.borrow().clone()
    }

    /// How many transcripts have been opened so far.
    pub fn opens(&self) -> usize {
        self.opens.load(Ordering::SeqCst)
    }

    /// Stream one transcript's records without loading the whole file.
    pub fn stream(&self, session_id: &str) -> Option<JsonlStream> {
        let path = self.root.join(format!("{session_id}.jsonl"));
        let file = File::open(&path).ok()?;
        self.opens.fetch_add(1, Ordering::SeqCst);
        Some(JsonlStream {
            reader: BufReader::new(file),
            session_id: session_id.to_string(),
            skipped: SkipCounter::default(),
            peak_buffered: 0,
        })
    }
}

fn sorted_jsonl(root: &Path) -> Vec<PathBuf> {
    let mut files = Vec::new();
    if let Ok(entries) = std::fs::read_dir(root) {
        for entry in entries.flatten() {
            let path = entry.path();
            if path.extension().and_then(|ext| ext.to_str()) == Some("jsonl") {
                files.push(path);
            }
        }
    }
    files.sort();
    files
}

fn session_id_of(path: &Path) -> String {
    path.file_stem()
        .and_then(|stem| stem.to_str())
        .unwrap_or_default()
        .to_string()
}

/// A [`RecordStream`] over one JSONL transcript, reading line by line.
#[derive(Debug)]
pub struct JsonlStream {
    reader: BufReader<File>,
    session_id: String,
    skipped: SkipCounter,
    peak_buffered: usize,
}

impl RecordStream for JsonlStream {
    fn next_record(&mut self) -> Option<RawRecord> {
        let mut line = String::new();
        loop {
            line.clear();
            match self.reader.read_line(&mut line) {
                Ok(0) => return None,
                Ok(_) => {}
                Err(_) => {
                    self.skipped.malformed += 1;
                    continue;
                }
            }
            let trimmed = line.trim();
            if trimmed.is_empty() {
                continue;
            }
            self.peak_buffered = self.peak_buffered.max(1);
            match serde_json::from_str::<Value>(trimmed) {
                Err(_) => {
                    self.skipped.malformed += 1;
                }
                Ok(payload) => {
                    let known = payload
                        .get("type")
                        .and_then(Value::as_str)
                        .is_some_and(|ty| KNOWN_RECORD_TYPES.contains(&ty));
                    if known {
                        return Some(RawRecord {
                            session_id: self.session_id.clone(),
                            payload,
                        });
                    }
                    self.skipped.unknown += 1;
                }
            }
        }
    }

    fn skipped(&self) -> SkipCounter {
        self.skipped.clone()
    }

    fn peak_buffered(&self) -> usize {
        self.peak_buffered
    }
}
