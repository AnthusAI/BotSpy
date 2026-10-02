//! The Codex importer.
//!
//! Watches rollout JSONL event logs under `~/.codex/sessions/YYYY/MM/DD/`,
//! indexed by `state_5.sqlite` (threads, projects, spawn edges) and
//! `thread_history_1.sqlite` (turn and item projections with rollout byte
//! offsets), and emits normalized records as they appear. Every rollout
//! record carries an explicit `ordinal` — the authoritative order, so
//! extraction sorts by ordinal instead of file order. `response_item`
//! records become messages (user/assistant/developer roles, reasoning
//! summaries, tool calls with outputs); `token_usage_record` parks the
//! token counts in session usage; `compacted` records chain compaction
//! windows; `task_complete` records turn timing. Rollouts are appended
//! live: extraction resumes from the stored byte offset, partial trailing
//! lines are retried on the next read, and malformed or unknown records
//! are skipped with counters. The source tree is only ever read.

use crate::importer::SkipCounter;
use crate::schema::{
    Agent, CompactionWindow, KnownPart, Message, Origin, Part, Provenance, Role, ToolArguments,
    Usage,
};
use crate::session::SessionSummary;
use rusqlite::OpenFlags;
use serde_json::Value;
use std::cell::RefCell;
use std::collections::BTreeMap;
use std::fs::File;
use std::io::{Read, Seek, SeekFrom};
use std::path::{Path, PathBuf};

/// One row of the threads index (`state_5.sqlite`).
#[derive(Debug, Clone, Default, PartialEq, Eq)]
struct ThreadRow {
    id: String,
    rollout_path: String,
    parent_id: Option<String>,
    cwd: String,
}

/// What discovery saw in the threads index.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct CodexDiscovery {
    /// Threads whose rollout file exists.
    pub threads: usize,
    /// Threads whose rollout file is missing, skipped by discovery.
    pub missing_rollouts: usize,
    /// Spawn edges: child thread id -> parent thread id.
    pub parent_links: BTreeMap<String, String>,
}

/// Diagnostics over a Codex home root.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct CodexReport {
    pub root: PathBuf,
    pub threads: usize,
    pub missing_rollouts: usize,
    pub issues: Vec<String>,
}

/// The result of one extraction pass over a rollout.
#[derive(Debug, Clone, PartialEq)]
pub struct CodexExtraction {
    pub session: crate::session::Session,
    /// Messages normalized by this pass.
    pub good: usize,
    /// Records skipped as malformed, unknown, or held-back partials.
    pub skipped: SkipCounter,
    /// Partial trailing lines held back for the next read.
    pub pending_partial: usize,
}

/// The Codex source over one home root (e.g. `~/.codex`).
#[derive(Debug)]
pub struct CodexSource {
    root: PathBuf,
    /// Per-thread rollout byte offsets, so extraction resumes where it
    /// stopped (thread_history_1.sqlite stores these).
    offsets: RefCell<BTreeMap<String, u64>>,
}

impl CodexSource {
    /// A source over the Codex home root.
    pub fn new(root: impl Into<PathBuf>) -> Self {
        Self {
            root: root.into(),
            offsets: RefCell::new(BTreeMap::new()),
        }
    }

    /// The home root this source watches.
    pub fn root(&self) -> &Path {
        &self.root
    }

    /// Seed the stored rollout byte offset for one thread (as
    /// thread_history_1.sqlite would have recorded it).
    pub fn set_offset(&self, session_id: &str, offset: u64) {
        self.offsets
            .borrow_mut()
            .insert(session_id.to_string(), offset);
    }

    fn read_threads(&self) -> rusqlite::Result<Vec<ThreadRow>> {
        let path = self.root.join("state_5.sqlite");
        let conn = rusqlite::Connection::open_with_flags(&path, OpenFlags::SQLITE_OPEN_READ_ONLY)?;
        let mut stmt = conn.prepare("SELECT id, rollout_path, parent_id, cwd FROM threads")?;
        let rows = stmt.query_map([], |row| {
            Ok(ThreadRow {
                id: row.get(0)?,
                rollout_path: row.get(1)?,
                parent_id: row.get(2)?,
                cwd: row.get::<_, Option<String>>(3)?.unwrap_or_default(),
            })
        })?;
        rows.collect()
    }

    /// Discover sessions via the threads index. Threads whose rollout file
    /// is missing are skipped and counted; spawn edges are reported as
    /// parent links.
    pub fn discover(&self) -> (Vec<SessionSummary>, CodexDiscovery) {
        let mut summaries = Vec::new();
        let mut stats = CodexDiscovery::default();
        for thread in self.read_threads().unwrap_or_default() {
            if !self.root.join(&thread.rollout_path).is_file() {
                stats.missing_rollouts += 1;
                continue;
            }
            if let Some(parent) = &thread.parent_id {
                stats.parent_links.insert(thread.id.clone(), parent.clone());
            }
            summaries.push(SessionSummary {
                id: thread.id,
                agent: Agent::Codex,
                project_id: thread.cwd,
                ..SessionSummary::default()
            });
            stats.threads += 1;
        }
        summaries.sort_by(|a, b| a.id.cmp(&b.id));
        (summaries, stats)
    }

    /// The doctor report: threads, missing rollouts, and issues.
    pub fn doctor(&self) -> CodexReport {
        let discovery = self.discover().1;
        let mut report = CodexReport {
            root: self.root.clone(),
            threads: discovery.threads,
            missing_rollouts: discovery.missing_rollouts,
            issues: Vec::new(),
        };
        if !self.root.join("state_5.sqlite").is_file() {
            report
                .issues
                .push("threads index state_5.sqlite is missing".to_string());
        }
        for thread in self.read_threads().unwrap_or_default() {
            if !self.root.join(&thread.rollout_path).is_file() {
                report
                    .issues
                    .push(format!("rollout {} is missing", thread.rollout_path));
            }
        }
        report
    }

    fn rollout_path(&self, session_id: &str) -> Option<PathBuf> {
        self.read_threads()
            .ok()?
            .into_iter()
            .find(|thread| thread.id == session_id)
            .map(|thread| self.root.join(thread.rollout_path))
    }

    /// Extract one rollout, resuming from the stored byte offset. Records
    /// are ordered by their explicit `ordinal`, not by file order. Returns
    /// `None` when no thread with that id exists.
    pub fn extract(&self, session_id: &str) -> Option<CodexExtraction> {
        let path = self.rollout_path(session_id)?;
        let mut file = File::open(&path).ok()?;
        let offset = *self.offsets.borrow().get(session_id).unwrap_or(&0);
        file.seek(SeekFrom::Start(offset)).ok()?;
        let mut text = String::new();
        file.read_to_string(&mut text).ok()?;

        let mut session = crate::session::Session {
            id: session_id.to_string(),
            agent: Agent::Codex,
            ..crate::session::Session::default()
        };
        let mut good = 0;
        let mut skipped = SkipCounter::default();
        let mut pending_partial = 0;
        let mut consumed = 0usize;
        let mut records: Vec<(Value, u64)> = Vec::new();

        let mut lines = text.split('\n').peekable();
        while let Some(line) = lines.next() {
            let is_last = lines.peek().is_none();
            if is_last && line.is_empty() {
                // The text ended with a newline: nothing left to consume.
                break;
            }
            if is_last && !line.is_empty() {
                // A trailing line without its newline: hold it back for
                // the next read; the offset stays at its start.
                pending_partial += 1;
                skipped.partial += 1;
                break;
            }
            consumed += line.len() + 1;
            let trimmed = line.trim();
            if trimmed.is_empty() {
                continue;
            }
            let line_no = records.len() as u64 + 1;
            match serde_json::from_str::<Value>(trimmed) {
                Ok(record) if record.get("ordinal").and_then(Value::as_u64).is_some() => {
                    records.push((record, line_no));
                }
                Ok(_) => skipped.malformed += 1,
                Err(_) => skipped.malformed += 1,
            }
        }

        // The explicit ordinal is the authoritative order.
        records.sort_by_key(|(record, _)| {
            record
                .get("ordinal")
                .and_then(Value::as_u64)
                .unwrap_or(u64::MAX)
        });

        for (record, line_no) in records {
            let ty = record.get("type").and_then(Value::as_str).unwrap_or("");
            match ty {
                "response_item" => {
                    if let Some(message) =
                        normalize_response_item(&record, &path, line_no, &mut session)
                    {
                        session.messages.push(message);
                        good += 1;
                    }
                }
                "token_usage_record" => apply_usage(&mut session, record.get("payload")),
                "compacted" => apply_compaction(&mut session, record.get("payload")),
                "task_complete" => apply_task_complete(&mut session, record.get("payload")),
                _ => skipped.unknown += 1,
            }
        }

        self.offsets
            .borrow_mut()
            .insert(session_id.to_string(), offset + consumed as u64);
        Some(CodexExtraction {
            session,
            good,
            skipped,
            pending_partial,
        })
    }
}

fn normalize_response_item(
    record: &Value,
    source_file: &Path,
    line_no: u64,
    session: &mut crate::session::Session,
) -> Option<Message> {
    let payload = record.get("payload")?;
    let payload_ty = payload.get("type").and_then(Value::as_str)?;
    let (role, parts, origin) = match payload_ty {
        "message" => {
            let role = payload.get("role").and_then(Value::as_str)?;
            let (role, origin) = match role {
                "user" => (Role::User, None),
                "assistant" => (Role::Assistant, None),
                "developer" => (Role::System, Some(Origin::Developer)),
                "system" => (Role::System, None),
                _ => return None,
            };
            (role, message_parts(payload.get("content")?), origin)
        }
        "reasoning" => {
            let texts: Vec<&str> = payload
                .get("summary")
                .and_then(Value::as_array)
                .map(|items| {
                    items
                        .iter()
                        .filter_map(|item| item.get("text").and_then(Value::as_str))
                        .collect()
                })
                .unwrap_or_default();
            if texts.is_empty() {
                return None;
            }
            (
                Role::Assistant,
                Some(vec![Part::Known(KnownPart::Thinking {
                    text: Some(texts.join("\n")),
                    signature: None,
                    encrypted: None,
                    extra: None,
                })]),
                None,
            )
        }
        "function_call" => {
            let call_id = payload
                .get("call_id")
                .and_then(Value::as_str)
                .unwrap_or_default();
            let name = payload
                .get("name")
                .and_then(Value::as_str)
                .unwrap_or_default();
            let arguments = payload
                .get("arguments")
                .and_then(Value::as_str)
                .and_then(|arguments| serde_json::from_str::<Value>(arguments).ok())
                .map(ToolArguments::from_value);
            (
                Role::Assistant,
                Some(vec![Part::Known(KnownPart::ToolCall {
                    id: call_id.to_string(),
                    name: name.to_string(),
                    arguments,
                    status: None,
                    extra: None,
                })]),
                None,
            )
        }
        "function_call_output" => {
            let call_id = payload
                .get("call_id")
                .and_then(Value::as_str)
                .unwrap_or_default();
            let text = match payload.get("output") {
                Some(Value::String(text)) => Some(text.clone()),
                Some(value) => Some(value.to_string()),
                None => None,
            };
            (
                Role::Tool,
                Some(vec![Part::Known(KnownPart::ToolResult {
                    call_id: call_id.to_string(),
                    text,
                    status: None,
                    extra: None,
                })]),
                None,
            )
        }
        _ => return None,
    };
    let parts = parts?;
    if parts.is_empty() {
        return None;
    }
    let mut message = Message {
        role,
        parts,
        origin,
        ..Message::default()
    };
    message.timestamp = record
        .get("timestamp")
        .and_then(Value::as_str)
        .map(str::to_string);
    if let Some(timestamp) = &message.timestamp {
        if session.started_at.is_empty() {
            session.started_at = timestamp.clone();
        }
        session.last_activity_at = timestamp.clone();
    }
    message.provenance = Some(Provenance {
        source_file: source_file.display().to_string(),
        line: Some(line_no),
        record_id: payload
            .get("id")
            .and_then(Value::as_str)
            .map(str::to_string),
        record_type: Some(payload_ty.to_string()),
        ordinal: record.get("ordinal").and_then(Value::as_u64),
        ..Provenance::default()
    });
    Some(message)
}

fn message_parts(content: &Value) -> Option<Vec<Part>> {
    match content {
        Value::String(text) => Some(vec![text_part(text)]),
        Value::Array(items) => {
            let mut parts = Vec::new();
            for item in items {
                let item_ty = item.get("type").and_then(Value::as_str).unwrap_or("");
                if matches!(item_ty, "input_text" | "output_text" | "text") {
                    if let Some(text) = item.get("text").and_then(Value::as_str) {
                        parts.push(text_part(text));
                    }
                }
            }
            Some(parts)
        }
        _ => None,
    }
}

fn text_part(text: &str) -> Part {
    Part::Known(KnownPart::Text {
        text: text.to_string(),
        extra: None,
    })
}

fn apply_usage(session: &mut crate::session::Session, payload: Option<&Value>) {
    let payload = match payload {
        Some(payload) => payload,
        None => return,
    };
    session.usage = Some(Usage {
        input_tokens: payload.get("input_tokens").and_then(Value::as_u64),
        output_tokens: payload.get("output_tokens").and_then(Value::as_u64),
        cached_input_tokens: payload.get("cached_input_tokens").and_then(Value::as_u64),
        reasoning_tokens: payload.get("reasoning_tokens").and_then(Value::as_u64),
        model: payload
            .get("model")
            .and_then(Value::as_str)
            .map(str::to_string),
        ..Usage::default()
    });
}

fn apply_compaction(session: &mut crate::session::Session, payload: Option<&Value>) {
    let Some(payload) = payload else { return };
    let window_id = payload
        .get("window_id")
        .and_then(Value::as_str)
        .unwrap_or_default()
        .to_string();
    session.compaction_windows.insert(
        window_id,
        CompactionWindow {
            previous_window: payload
                .get("previous_window")
                .and_then(Value::as_str)
                .map(str::to_string),
            retained_context: payload
                .get("retained_context")
                .and_then(Value::as_str)
                .map(str::to_string),
        },
    );
}

fn apply_task_complete(session: &mut crate::session::Session, payload: Option<&Value>) {
    let Some(payload) = payload else { return };
    let turn_id = payload
        .get("turn_id")
        .and_then(Value::as_str)
        .map(str::to_string)
        .unwrap_or_else(|| "turn".to_string());
    session.turns.insert(
        turn_id.clone(),
        crate::schema::Turn {
            id: turn_id,
            duration_ms: payload.get("duration_ms").and_then(Value::as_u64),
            time_to_first_token_ms: payload
                .get("time_to_first_token_ms")
                .and_then(Value::as_u64),
            ..crate::schema::Turn::default()
        },
    );
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::schema::{KnownPart, Part, Role};

    fn write_rollout(root: &Path, path: &str, lines: &[String]) {
        let full = root.join(path);
        std::fs::create_dir_all(full.parent().unwrap()).unwrap();
        std::fs::write(
            full,
            lines
                .iter()
                .map(|line| format!("{line}\n"))
                .collect::<String>(),
        )
        .unwrap();
    }

    fn write_threads_index(root: &Path, rows: &[(&str, &str, Option<&str>, &str)]) {
        let conn = rusqlite::Connection::open(root.join("state_5.sqlite")).unwrap();
        conn.execute(
            "CREATE TABLE threads (id TEXT PRIMARY KEY, rollout_path TEXT NOT NULL, parent_id TEXT, cwd TEXT)",
            [],
        )
        .unwrap();
        for (id, rollout_path, parent_id, cwd) in rows {
            conn.execute(
                "INSERT INTO threads (id, rollout_path, parent_id, cwd) VALUES (?1, ?2, ?3, ?4)",
                rusqlite::params![id, rollout_path, parent_id, cwd],
            )
            .unwrap();
        }
    }

    fn user_record(ordinal: u64, timestamp: &str, text: &str) -> String {
        format!(
            r#"{{"ordinal":{ordinal},"timestamp":"{timestamp}","type":"response_item","payload":{{"type":"message","id":"m{ordinal}","role":"user","content":[{{"type":"input_text","text":"{text}"}}]}}}}"#
        )
    }

    fn temp_root(tag: &str) -> PathBuf {
        let root = std::env::temp_dir().join(format!("botspy-cx-{tag}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&root);
        std::fs::create_dir_all(&root).unwrap();
        root
    }

    #[test]
    fn discovery_orders_threads_and_reports_spawn_edges() {
        let root = temp_root("discover");
        write_threads_index(
            &root,
            &[
                (
                    "t2",
                    "sessions/2026/10/01/rollout-t2.jsonl",
                    Some("t1"),
                    "/repo/demo",
                ),
                (
                    "t1",
                    "sessions/2026/10/01/rollout-t1.jsonl",
                    None,
                    "/repo/demo",
                ),
            ],
        );
        write_rollout(
            &root,
            "sessions/2026/10/01/rollout-t1.jsonl",
            &[user_record(1, "2026-10-01T09:00:00Z", "hi")],
        );
        write_rollout(
            &root,
            "sessions/2026/10/01/rollout-t2.jsonl",
            &[user_record(1, "2026-10-01T09:00:05Z", "sub")],
        );
        write_rollout(
            &root,
            "sessions/2026/10/01/rollout-t3.jsonl",
            &[user_record(1, "2026-10-01T09:00:10Z", "orphan")],
        );
        let source = CodexSource::new(&root);
        let (summaries, stats) = source.discover();
        assert_eq!(summaries.len(), 2);
        assert_eq!(summaries[0].id, "t1");
        assert_eq!(summaries[0].project_id, "/repo/demo");
        assert_eq!(stats.threads, 2);
        assert_eq!(stats.parent_links.get("t2").map(String::as_str), Some("t1"));
    }

    #[test]
    fn doctor_flags_missing_rollouts() {
        let root = temp_root("doctor");
        write_threads_index(
            &root,
            &[("t1", "sessions/2026/10/01/rollout-t1.jsonl", None, "/repo")],
        );
        let source = CodexSource::new(&root);
        let report = source.doctor();
        assert_eq!(report.threads, 0);
        assert_eq!(report.missing_rollouts, 1);
        assert!(report
            .issues
            .iter()
            .any(|issue| issue.contains("rollout-t1.jsonl")));
    }

    #[test]
    fn ordinal_is_the_authoritative_order() {
        let root = temp_root("ordinal");
        write_threads_index(
            &root,
            &[("t1", "sessions/2026/10/01/rollout-t1.jsonl", None, "/repo")],
        );
        write_rollout(
            &root,
            "sessions/2026/10/01/rollout-t1.jsonl",
            &[
                user_record(3, "2026-10-01T09:00:15Z", "third"),
                user_record(1, "2026-10-01T09:00:05Z", "first"),
                user_record(2, "2026-10-01T09:00:10Z", "second"),
            ],
        );
        let source = CodexSource::new(&root);
        let extraction = source.extract("t1").expect("extract");
        let ordinals: Vec<u64> = extraction
            .session
            .messages
            .iter()
            .map(|message| {
                message
                    .provenance
                    .as_ref()
                    .and_then(|prov| prov.ordinal)
                    .expect("ordinal")
            })
            .collect();
        assert_eq!(ordinals, vec![1, 2, 3]);
        assert_eq!(extraction.good, 3);
    }

    #[test]
    fn extraction_resumes_from_the_stored_offset() {
        let root = temp_root("offset");
        write_threads_index(
            &root,
            &[("t1", "sessions/2026/10/01/rollout-t1.jsonl", None, "/repo")],
        );
        write_rollout(
            &root,
            "sessions/2026/10/01/rollout-t1.jsonl",
            &[
                user_record(1, "2026-10-01T09:00:05Z", "first"),
                user_record(2, "2026-10-01T09:00:10Z", "second"),
                user_record(3, "2026-10-01T09:00:15Z", "third"),
                user_record(4, "2026-10-01T09:00:20Z", "fourth"),
            ],
        );
        let path = root.join("sessions/2026/10/01/rollout-t1.jsonl");
        let content = std::fs::read_to_string(&path).unwrap();
        let after_two: usize = content.lines().take(2).map(|line| line.len() + 1).sum();
        let source = CodexSource::new(&root);
        source.set_offset("t1", after_two as u64);
        let resumed = source.extract("t1").expect("extract from stored offset");
        assert_eq!(resumed.good, 2);
        let ordinals: Vec<u64> = resumed
            .session
            .messages
            .iter()
            .map(|message| {
                message
                    .provenance
                    .as_ref()
                    .and_then(|prov| prov.ordinal)
                    .expect("ordinal")
            })
            .collect();
        assert_eq!(ordinals, vec![3, 4]);
    }

    #[test]
    fn response_items_map_roles_reasoning_and_tool_kinds() {
        let root = temp_root("parts");
        write_threads_index(
            &root,
            &[("t1", "sessions/2026/10/01/rollout-t1.jsonl", None, "/repo")],
        );
        write_rollout(
            &root,
            "sessions/2026/10/01/rollout-t1.jsonl",
            &[
                user_record(1, "2026-10-01T09:00:00Z", "run the tests"),
                r#"{"ordinal":2,"timestamp":"2026-10-01T09:00:05Z","type":"response_item","payload":{"type":"message","role":"assistant","content":[{"type":"output_text","text":"On it."}]}}"#.to_string(),
                r#"{"ordinal":3,"timestamp":"2026-10-01T09:00:06Z","type":"response_item","payload":{"type":"message","role":"developer","content":[{"type":"input_text","text":"Be concise."}]}}"#.to_string(),
                r#"{"ordinal":4,"timestamp":"2026-10-01T09:00:07Z","type":"response_item","payload":{"type":"reasoning","summary":[{"type":"summary_text","text":"thinking it through"}]}}"#.to_string(),
                r#"{"ordinal":5,"timestamp":"2026-10-01T09:00:08Z","type":"response_item","payload":{"type":"function_call","call_id":"call_1","name":"shell","arguments":"{\"cmd\":[\"cargo\",\"test\"]}"}}"#.to_string(),
                r#"{"ordinal":6,"timestamp":"2026-10-01T09:00:20Z","type":"response_item","payload":{"type":"function_call_output","call_id":"call_1","output":"all tests pass"}}"#.to_string(),
            ],
        );
        let source = CodexSource::new(&root);
        let extraction = source.extract("t1").expect("extract");
        let messages = &extraction.session.messages;
        assert_eq!(messages.len(), 6);
        assert_eq!(messages[0].role, Role::User);
        assert_eq!(messages[1].role, Role::Assistant);
        assert_eq!(messages[2].role, Role::System);
        assert_eq!(messages[2].origin, Some(Origin::Developer));
        assert!(matches!(
            messages[3].parts.first(),
            Some(Part::Known(KnownPart::Thinking { .. }))
        ));
        assert!(matches!(
            messages[4].parts.first(),
            Some(Part::Known(KnownPart::ToolCall { .. }))
        ));
        assert!(matches!(
            messages[5].parts.first(),
            Some(Part::Known(KnownPart::ToolResult { .. }))
        ));
        assert_eq!(messages[5].role, Role::Tool);
    }

    #[test]
    fn usage_compaction_and_task_complete_map_to_session_state() {
        let root = temp_root("aux");
        write_threads_index(
            &root,
            &[("t1", "sessions/2026/10/01/rollout-t1.jsonl", None, "/repo")],
        );
        write_rollout(
            &root,
            "sessions/2026/10/01/rollout-t1.jsonl",
            &[
                r#"{"ordinal":1,"type":"token_usage_record","payload":{"input_tokens":1200,"cached_input_tokens":300,"output_tokens":800,"reasoning_tokens":150,"model":"gpt-5-codex"}}"#.to_string(),
                r#"{"ordinal":2,"type":"compacted","payload":{"window_id":"w2","previous_window":"w1","retained_context":"summary so far"}}"#.to_string(),
                r#"{"ordinal":3,"type":"task_complete","payload":{"turn_id":"turn-1","duration_ms":14300,"time_to_first_token_ms":850}}"#.to_string(),
            ],
        );
        let source = CodexSource::new(&root);
        let extraction = source.extract("t1").expect("extract");
        let session = extraction.session;
        let usage = session.usage.as_ref().expect("session usage");
        assert_eq!(usage.input_tokens, Some(1200));
        assert_eq!(usage.cached_input_tokens, Some(300));
        assert_eq!(usage.output_tokens, Some(800));
        assert_eq!(usage.reasoning_tokens, Some(150));
        assert_eq!(usage.model.as_deref(), Some("gpt-5-codex"));
        let window = session.compaction_windows.get("w2").expect("window");
        assert_eq!(window.previous_window.as_deref(), Some("w1"));
        assert_eq!(window.retained_context.as_deref(), Some("summary so far"));
        let turn = session.turns.get("turn-1").expect("turn");
        assert_eq!(turn.duration_ms, Some(14300));
        assert_eq!(turn.time_to_first_token_ms, Some(850));
    }

    #[test]
    fn partial_trailing_and_malformed_are_skipped_not_normalized() {
        let root = temp_root("partial");
        write_threads_index(
            &root,
            &[("t1", "sessions/2026/10/01/rollout-t1.jsonl", None, "/repo")],
        );
        let full = root.join("sessions/2026/10/01/rollout-t1.jsonl");
        std::fs::create_dir_all(full.parent().unwrap()).unwrap();
        std::fs::write(
            &full,
            format!(
                "this line is {{ not json\n{}",
                r#"{"ordinal":2,"type":"response_item","payload":{"type":"message","role":"user","content":"cut off"#
            ),
        )
        .unwrap();
        let source = CodexSource::new(&root);
        let extraction = source.extract("t1").expect("extract");
        assert_eq!(extraction.good, 0);
        assert_eq!(extraction.skipped.malformed, 1);
        assert_eq!(extraction.skipped.partial, 1);
        assert_eq!(extraction.skipped.total(), 2);
        assert_eq!(extraction.pending_partial, 1);

        // Completing the append recovers the held-back record.
        std::fs::write(
            &full,
            format!(
                "this line is {{ not json\n{}\n",
                r#"{"ordinal":2,"type":"response_item","payload":{"type":"message","role":"user","content":[{"type":"input_text","text":"complete now"}]}}"#
            ),
        )
        .unwrap();
        let recovered = source.extract("t1").expect("extract again");
        assert_eq!(recovered.good, 1);
        assert_eq!(recovered.pending_partial, 0);
    }
}
