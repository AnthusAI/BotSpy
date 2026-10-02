//! The Claude Code importer.
//!
//! Watches append-only JSONL transcripts under
//! `~/.claude/projects/<encoded-cwd>/` — one file per session — plus
//! sibling `subagents/agent-*.jsonl` sidechain files. Discovery skips the
//! sidechains (counting them); extraction follows each transcript by byte
//! offset, maps records into the chapter-1 schema, parks auxiliary records
//! (`cost-state`, `pr-link`, `custom-title`) in session state, holds back
//! partial trailing lines for the next read, and skips malformed lines and
//! unknown record types with counters. The source tree is only ever read.

use crate::schema::{
    Agent, KnownPart, Message, Part, Provenance, Role, SessionCost, ToolArguments,
};
use crate::session::SessionSummary;
use serde_json::Value;
use std::collections::BTreeMap;
use std::fs::File;
use std::io::{Read, Seek, SeekFrom};
use std::path::{Path, PathBuf};
use std::sync::Mutex;

/// What discovery saw under the projects root.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct ClaudeDiscovery {
    /// Session transcripts found (one per `*.jsonl` under a project dir).
    pub transcripts: usize,
    /// Sidechain transcripts under `subagents/`, skipped by discovery.
    pub skipped_subagents: usize,
    /// Other files that are not session transcripts, skipped.
    pub skipped_files: usize,
}

/// Diagnostics over a Claude Code projects root.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct ClaudeReport {
    pub root: PathBuf,
    pub project_dirs: usize,
    pub transcripts: usize,
    pub skipped_subagents: usize,
    pub skipped_files: usize,
    pub issues: Vec<String>,
}

/// The result of one extraction pass over a transcript.
#[derive(Debug, Clone, PartialEq)]
pub struct ClaudeExtraction {
    pub session: crate::session::Session,
    /// Messages normalized by this pass.
    pub good: usize,
    /// Lines skipped as malformed or of unknown type.
    pub skipped: crate::importer::SkipCounter,
    /// Partial trailing lines held back for the next read.
    pub pending_partial: usize,
}

/// The Claude Code source over one projects root.
#[derive(Debug)]
pub struct ClaudeCodeSource {
    root: PathBuf,
    /// Per-session byte offsets, so extraction resumes where it stopped.
    offsets: Mutex<BTreeMap<String, u64>>,
}

impl ClaudeCodeSource {
    /// A source over the projects root (e.g. `~/.claude/projects`).
    pub fn new(root: impl Into<PathBuf>) -> Self {
        Self {
            root: root.into(),
            offsets: Mutex::new(BTreeMap::new()),
        }
    }

    /// The projects root this source watches.
    pub fn root(&self) -> &Path {
        &self.root
    }

    /// Discover sessions under the root. Sidechain files under a project
    /// dir's `subagents/` folder are skipped and counted, not listed.
    pub fn discover(&self) -> (Vec<SessionSummary>, ClaudeDiscovery) {
        let mut summaries = Vec::new();
        let mut stats = ClaudeDiscovery::default();
        for project in sorted_dirs(&self.root) {
            let mut saw_session = false;
            if let Ok(entries) = std::fs::read_dir(&project) {
                for entry in entries.flatten() {
                    let path = entry.path();
                    if !path.is_file() {
                        continue;
                    }
                    if path.extension().and_then(|ext| ext.to_str()) == Some("jsonl") {
                        summaries.push(SessionSummary {
                            id: file_stem(&path),
                            agent: Agent::ClaudeCode,
                            project_id: file_stem(&project),
                            ..SessionSummary::default()
                        });
                        stats.transcripts += 1;
                        saw_session = true;
                    } else {
                        stats.skipped_files += 1;
                    }
                }
            }
            if !saw_session {
                continue;
            }
            let subagents = project.join("subagents");
            if subagents.is_dir() {
                stats.skipped_subagents += count_files(&subagents, "jsonl");
            }
        }
        summaries.sort_by(|a, b| a.id.cmp(&b.id));
        (summaries, stats)
    }

    /// The doctor report: roots, transcript counts, skipped subagents,
    /// and issues.
    pub fn doctor(&self) -> ClaudeReport {
        let (_, discovery) = self.discover();
        let mut report = ClaudeReport {
            root: self.root.clone(),
            project_dirs: sorted_dirs(&self.root).len(),
            transcripts: discovery.transcripts,
            skipped_subagents: discovery.skipped_subagents,
            skipped_files: discovery.skipped_files,
            issues: Vec::new(),
        };
        for project in sorted_dirs(&self.root) {
            if !project.is_dir() {
                report
                    .issues
                    .push(format!("{} is not a directory", project.display()));
            }
        }
        for summary in self.discover().0 {
            if File::open(self.transcript_path(&summary.id)).is_err() {
                report
                    .issues
                    .push(format!("transcript {} is unreadable", summary.id));
            }
        }
        if report.transcripts == 0 {
            report
                .issues
                .push("no session transcripts found under the root".to_string());
        }
        report
    }

    fn transcript_path(&self, session_id: &str) -> PathBuf {
        for project in sorted_dirs(&self.root) {
            let candidate = project.join(format!("{session_id}.jsonl"));
            if candidate.is_file() {
                return candidate;
            }
        }
        self.root.join(format!("{session_id}.jsonl"))
    }

    /// Extract one transcript, resuming from the stored byte offset.
    /// Returns `None` when no transcript with that id exists.
    pub fn extract(&self, session_id: &str) -> Option<ClaudeExtraction> {
        let path = self.transcript_path(session_id);
        let mut file = File::open(&path).ok()?;
        let offset = *self
            .offsets
            .lock()
            .expect("offsets")
            .get(session_id)
            .unwrap_or(&0);
        file.seek(SeekFrom::Start(offset)).ok()?;
        let mut text = String::new();
        file.read_to_string(&mut text).ok()?;

        let mut session = crate::session::Session {
            id: session_id.to_string(),
            agent: Agent::ClaudeCode,
            project_id: path.parent().map(file_stem).unwrap_or_default(),
            ..crate::session::Session::default()
        };
        let mut good = 0;
        let mut skipped = crate::importer::SkipCounter::default();
        let mut pending_partial = 0;
        let mut consumed = 0usize;

        let mut lines = text.split('\n').peekable();
        while let Some(line) = lines.next() {
            let is_last = lines.peek().is_none();
            if is_last && line.is_empty() {
                // The text ended with a newline: nothing left to consume.
                break;
            }
            let partial = is_last && !line.is_empty();
            if partial {
                // A trailing line without its newline: hold it back for
                // the next read; the offset stays at its start.
                pending_partial += 1;
                break;
            }
            consumed += line.len() + 1;
            let trimmed = line.trim();
            if trimmed.is_empty() {
                continue;
            }
            let record: Value = match serde_json::from_str(trimmed) {
                Ok(record) => record,
                Err(_) => {
                    skipped.malformed += 1;
                    continue;
                }
            };
            match record.get("type").and_then(Value::as_str) {
                Some("cost-state") => apply_cost(&mut session, &record),
                Some("pr-link") => {
                    session.metadata.pr_url = record
                        .get("url")
                        .or_else(|| record.get("prUrl"))
                        .and_then(Value::as_str)
                        .map(str::to_string);
                }
                Some("custom-title") => {
                    session.metadata.title = record
                        .get("title")
                        .and_then(Value::as_str)
                        .map(str::to_string);
                }
                _ => match self.normalize_record(&record, &path, consumed) {
                    Some(Ok(message)) => {
                        if let Some(timestamp) = &message.timestamp {
                            if session.last_activity_at.is_empty() {
                                session.started_at = timestamp.clone();
                            }
                            session.last_activity_at = timestamp.clone();
                        }
                        session.messages.push(message);
                        good += 1;
                    }
                    Some(Err(reason)) => match reason {
                        SkipReason::Malformed => skipped.malformed += 1,
                        SkipReason::Unknown => skipped.unknown += 1,
                    },
                    None => skipped.unknown += 1,
                },
            }
        }

        // Always record the offset: when a partial trailing line is held
        // back, the offset stays at its start so the next read resumes
        // there.
        self.offsets
            .lock()
            .expect("offsets")
            .insert(session_id.to_string(), offset + consumed as u64);
        Some(ClaudeExtraction {
            session,
            good,
            skipped,
            pending_partial,
        })
    }

    /// Map one record into a message (or session-state update). `None`
    /// means the record is not message-shaped; `Some(Err(..))` counts a
    /// skip. Auxiliary records (`cost-state`, `pr-link`, `custom-title`)
    /// carry no message and are handled by the caller.
    fn normalize_record(
        &self,
        record: &Value,
        source_file: &Path,
        line_no: usize,
    ) -> Option<Result<Message, SkipReason>> {
        let ty = record.get("type").and_then(Value::as_str)?;
        let known_types = [
            "user",
            "assistant",
            "system",
            "summary",
            "cost-state",
            "pr-link",
            "custom-title",
        ];
        if !known_types.contains(&ty) {
            return Some(Err(SkipReason::Unknown));
        }
        let parts = match ty {
            "user" => user_parts(record.get("message")?).filter(|parts| !parts.is_empty()),
            "assistant" => {
                assistant_parts(record.get("message")?).filter(|parts| !parts.is_empty())
            }
            "system" => Some(vec![Part::Known(KnownPart::System {
                text: record
                    .get("content")
                    .and_then(Value::as_str)
                    .unwrap_or_default()
                    .to_string(),
                extra: None,
            })]),
            "summary" => Some(vec![Part::Known(KnownPart::Text {
                text: record
                    .get("summary")
                    .and_then(Value::as_str)
                    .unwrap_or_default()
                    .to_string(),
                extra: None,
            })]),
            _ => return None,
        }?;
        if parts.is_empty() {
            return Some(Err(SkipReason::Malformed));
        }
        let role = match ty {
            "user" | "summary" => Role::User,
            "assistant" => Role::Assistant,
            _ => Role::System,
        };
        let mut message = Message {
            role,
            parts,
            is_compaction_summary: ty == "summary",
            ..Message::default()
        };
        message.timestamp = record
            .get("timestamp")
            .and_then(Value::as_str)
            .map(str::to_string);
        message.provenance = Some(Provenance {
            source_file: source_file.display().to_string(),
            line: Some(line_no as u64),
            record_id: record
                .get("uuid")
                .and_then(Value::as_str)
                .map(str::to_string),
            parent_record: record
                .get("parentUuid")
                .and_then(Value::as_str)
                .filter(|parent| !parent.is_empty())
                .map(str::to_string),
            record_type: Some(ty.to_string()),
            ..Provenance::default()
        });
        Some(Ok(message))
    }
}

enum SkipReason {
    Malformed,
    Unknown,
}

fn apply_cost(session: &mut crate::session::Session, record: &Value) {
    session.cost = Some(SessionCost {
        total_cost_usd: num_f64(record.get("totalCostUsd")),
        total_api_duration_ms: num_u64(record.get("totalApiDurationMs")),
        total_tool_duration_ms: num_u64(record.get("totalToolDurationMs")),
        total_lines_added: num_u64(record.get("totalLinesAdded")),
        total_lines_removed: num_u64(record.get("totalLinesRemoved")),
        ..SessionCost::default()
    });
}

fn user_parts(message: &Value) -> Option<Vec<Part>> {
    let content = message.get("content")?;
    match content {
        Value::String(text) => Some(vec![Part::Known(KnownPart::Text {
            text: text.clone(),
            extra: None,
        })]),
        Value::Array(items) => {
            let mut parts = Vec::new();
            for item in items {
                match item.get("type").and_then(Value::as_str) {
                    Some("text") => {
                        if let Some(text) = item.get("text").and_then(Value::as_str) {
                            parts.push(Part::Known(KnownPart::Text {
                                text: text.to_string(),
                                extra: None,
                            }));
                        }
                    }
                    Some("tool_result") => {
                        let call_id = item
                            .get("tool_use_id")
                            .and_then(Value::as_str)
                            .unwrap_or_default();
                        let text = match item.get("content") {
                            Some(Value::String(text)) => Some(text.clone()),
                            Some(value @ Value::Array(_)) => Some(value.to_string()),
                            _ => None,
                        };
                        parts.push(Part::Known(KnownPart::ToolResult {
                            call_id: call_id.to_string(),
                            text,
                            status: None,
                            extra: None,
                        }));
                    }
                    _ => {}
                }
            }
            Some(parts)
        }
        _ => None,
    }
}

fn assistant_parts(message: &Value) -> Option<Vec<Part>> {
    let content = message.get("content")?.as_array()?;
    let mut parts = Vec::new();
    for item in content {
        match item.get("type").and_then(Value::as_str) {
            Some("text") => {
                if let Some(text) = item.get("text").and_then(Value::as_str) {
                    parts.push(Part::Known(KnownPart::Text {
                        text: text.to_string(),
                        extra: None,
                    }));
                }
            }
            Some("thinking") => parts.push(Part::Known(KnownPart::Thinking {
                text: item
                    .get("thinking")
                    .and_then(Value::as_str)
                    .map(str::to_string),
                signature: item
                    .get("signature")
                    .and_then(Value::as_str)
                    .map(str::to_string),
                encrypted: None,
                extra: None,
            })),
            Some("tool_use") => parts.push(Part::Known(KnownPart::ToolCall {
                id: item
                    .get("id")
                    .and_then(Value::as_str)
                    .unwrap_or_default()
                    .to_string(),
                name: item
                    .get("name")
                    .and_then(Value::as_str)
                    .unwrap_or_default()
                    .to_string(),
                arguments: item.get("input").cloned().map(ToolArguments::from_value),
                status: None,
                extra: None,
            })),
            _ => {}
        }
    }
    Some(parts)
}

fn num_f64(value: Option<&Value>) -> Option<f64> {
    value.and_then(Value::as_f64)
}

fn num_u64(value: Option<&Value>) -> Option<u64> {
    value.and_then(Value::as_u64)
}

fn sorted_dirs(root: &Path) -> Vec<PathBuf> {
    let mut dirs = Vec::new();
    if let Ok(entries) = std::fs::read_dir(root) {
        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_dir() {
                dirs.push(path);
            }
        }
    }
    dirs.sort();
    dirs
}

fn count_files(dir: &Path, extension: &str) -> usize {
    std::fs::read_dir(dir)
        .map(|entries| {
            entries
                .flatten()
                .filter(|entry| {
                    entry
                        .path()
                        .extension()
                        .and_then(|ext| ext.to_str())
                        .is_some_and(|ext| ext == extension)
                })
                .count()
        })
        .unwrap_or(0)
}

fn file_stem(path: &Path) -> String {
    path.file_stem()
        .and_then(|stem| stem.to_str())
        .unwrap_or_default()
        .to_string()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::schema::{KnownPart, Part, Role};

    fn write_transcript(root: &Path, project: &str, name: &str, lines: &[String]) {
        let dir = root.join(project);
        std::fs::create_dir_all(&dir).unwrap();
        std::fs::write(
            dir.join(name),
            lines
                .iter()
                .map(|line| format!("{line}\n"))
                .collect::<String>(),
        )
        .unwrap();
    }

    fn user_record(uuid: &str, parent: &str, timestamp: &str, text: &str) -> String {
        format!(
            r#"{{"type":"user","uuid":"{uuid}","parentUuid":"{parent}","timestamp":"{timestamp}","message":{{"role":"user","content":"{text}"}}}}"#
        )
    }

    fn assistant_record(uuid: &str, parent: &str, timestamp: &str) -> String {
        format!(
            r#"{{"type":"assistant","uuid":"{uuid}","parentUuid":"{parent}","timestamp":"{timestamp}","message":{{"role":"assistant","content":[{{"type":"thinking","thinking":"why","signature":"sig"}},{{"type":"tool_use","id":"call_1","name":"read_file","input":{{"path":"main.rs"}}}}]}}}}"#
        )
    }

    #[test]
    fn discovery_skips_sidechains_and_counts_them() {
        let root = std::env::temp_dir().join(format!("botspy-cc-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&root);
        write_transcript(
            &root,
            "proj-a",
            "s1.jsonl",
            &[user_record("u1", "", "2026-10-01T09:00:00Z", "hi")],
        );
        let sub = root.join("proj-a").join("subagents");
        std::fs::create_dir_all(&sub).unwrap();
        std::fs::write(sub.join("agent-1.jsonl"), "{}\n").unwrap();
        std::fs::write(root.join("proj-a").join("notes.txt"), "junk").unwrap();
        let source = ClaudeCodeSource::new(&root);
        let (summaries, stats) = source.discover();
        assert_eq!(summaries.len(), 1);
        assert_eq!(summaries[0].id, "s1");
        assert_eq!(summaries[0].project_id, "proj-a");
        assert_eq!(stats.transcripts, 1);
        assert_eq!(stats.skipped_subagents, 1);
        assert_eq!(stats.skipped_files, 1);
    }

    #[test]
    fn extraction_maps_parts_and_provenance() {
        let root = std::env::temp_dir().join(format!("botspy-cc2-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&root);
        write_transcript(
            &root,
            "proj-a",
            "s1.jsonl",
            &[
                user_record("u1", "", "2026-10-01T09:00:00Z", "hello"),
                assistant_record("u2", "u1", "2026-10-01T09:00:05Z"),
                r#"{"type":"user","uuid":"u3","parentUuid":"u2","timestamp":"2026-10-01T09:00:10Z","message":{"role":"user","content":[{"type":"tool_result","tool_use_id":"call_1","content":"contents"}]}}"#.to_string(),
                r#"{"type":"cost-state","totalCostUsd":0.42,"totalApiDurationMs":1200}"#.to_string(),
                r#"{"type":"pr-link","url":"https://github.com/x/y/pull/7"}"#.to_string(),
                r#"{"type":"custom-title","title":"Fix parser"}"#.to_string(),
            ],
        );
        let source = ClaudeCodeSource::new(&root);
        let extraction = source.extract("s1").expect("extract");
        assert_eq!(extraction.good, 3);
        assert_eq!(extraction.skipped.total(), 0);
        let session = extraction.session;
        assert_eq!(
            session.cost.as_ref().and_then(|cost| cost.total_cost_usd),
            Some(0.42)
        );
        assert_eq!(
            session.metadata.pr_url.as_deref(),
            Some("https://github.com/x/y/pull/7")
        );
        assert_eq!(session.metadata.title.as_deref(), Some("Fix parser"));
        assert_eq!(session.started_at, "2026-10-01T09:00:00Z");
        assert_eq!(session.last_activity_at, "2026-10-01T09:00:10Z");
        let first = &session.messages[0];
        assert_eq!(first.role, Role::User);
        let provenance = first.provenance.as_ref().expect("provenance");
        assert_eq!(provenance.record_id.as_deref(), Some("u1"));
        assert_eq!(provenance.parent_record, None);
        assert_eq!(provenance.record_type.as_deref(), Some("user"));
        let second = &session.messages[1];
        assert!(matches!(
            second.parts.first(),
            Some(Part::Known(KnownPart::Thinking { .. }))
        ));
        assert!(matches!(
            second.parts.get(1),
            Some(Part::Known(KnownPart::ToolCall { .. }))
        ));
        let third = &session.messages[2];
        assert!(matches!(
            third.parts.first(),
            Some(Part::Known(KnownPart::ToolResult { .. }))
        ));
    }

    #[test]
    fn malformed_and_unknown_are_skipped_and_counted() {
        let root = std::env::temp_dir().join(format!("botspy-cc3-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&root);
        write_transcript(
            &root,
            "proj-a",
            "s1.jsonl",
            &[
                user_record("u1", "", "2026-10-01T09:00:00Z", "hi"),
                "not json at all".to_string(),
                r#"{"type":"file-history-snapshot","files":[]}"#.to_string(),
            ],
        );
        let source = ClaudeCodeSource::new(&root);
        let extraction = source.extract("s1").expect("extract");
        assert_eq!(extraction.good, 1);
        assert_eq!(extraction.skipped.malformed, 1);
        assert_eq!(extraction.skipped.unknown, 1);
    }

    #[test]
    fn partial_trailing_lines_are_held_back_then_recovered() {
        let root = std::env::temp_dir().join(format!("botspy-cc4-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&root);
        let path = root.join("proj-a").join("s1.jsonl");
        std::fs::create_dir_all(path.parent().unwrap()).unwrap();
        std::fs::write(
            &path,
            format!(
                "{}\n{}",
                user_record("u1", "", "2026-10-01T09:00:00Z", "hi"),
                "{\"type\":\"ass"
            ),
        )
        .unwrap();
        let source = ClaudeCodeSource::new(&root);
        let first = source.extract("s1").expect("extract");
        assert_eq!(first.good, 1);
        assert_eq!(first.pending_partial, 1);
        std::fs::write(
            &path,
            format!(
                "{}\n{}\n",
                user_record("u1", "", "2026-10-01T09:00:00Z", "hi"),
                assistant_record("u2", "u1", "2026-10-01T09:00:05Z")
            ),
        )
        .unwrap();
        let second = source.extract("s1").expect("extract again");
        assert_eq!(second.good, 1);
        assert_eq!(second.pending_partial, 0);
    }
}
