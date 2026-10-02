//! The Grok Bot importer.
//!
//! Watches the plain-JSON `sand-client-persistence` blobs — chat entry
//! logs keyed by opaque hash names, an agent roster (`roster.json`
//! mapping blob names to agents), and cloud-agent records
//! (`cloud-agents.json`) — plus `~/.grokbot/` settings, and emits
//! normalized records as they appear. Entry-log records carry an explicit
//! `seq` — the authoritative order; persona entries record the author;
//! voice-call entries are opaque and preserved as raw parts. Entry logs
//! are capped at 200 entries locally with the full history server-side,
//! so sessions are flagged as partial local views (`PartialReason::LocalCap`)
//! by design.

use crate::importer::SkipCounter;
use crate::schema::{
    Agent, KnownPart, Message, Part, PartialHistory, PartialReason, Provenance, Role,
};
use crate::session::SessionSummary;
use serde_json::Value;
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

/// What discovery saw in the persistence directory.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct GrokDiscovery {
    /// Entry-log blobs found (one session each).
    pub entry_logs: usize,
    /// Cloud-agent records found (one session stub each).
    pub cloud_agents: usize,
    /// Roster attributions: blob name -> agent name.
    pub attributions: BTreeMap<String, String>,
    /// Cloud-agent session stub ids (no local transcript).
    pub cloud_agent_ids: Vec<String>,
}

/// Diagnostics over a Grok Bot persistence directory.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct GrokReport {
    pub root: PathBuf,
    pub entry_logs: usize,
    pub cloud_agents: usize,
    pub issues: Vec<String>,
}

/// The result of one extraction pass over an entry log.
#[derive(Debug, Clone, PartialEq)]
pub struct GrokExtraction {
    pub session: crate::session::Session,
    /// Entries normalized by this pass.
    pub good: usize,
    /// Entries skipped as malformed or of unknown type.
    pub skipped: SkipCounter,
}

/// The Grok Bot source over one persistence directory.
#[derive(Debug)]
pub struct GrokBotSource {
    root: PathBuf,
}

const RESERVED_BLOBS: [&str; 2] = ["roster", "cloud-agents"];

impl GrokBotSource {
    /// A source over the persistence directory (e.g.
    /// `~/.grokbot/sand-client-persistence`).
    pub fn new(root: impl Into<PathBuf>) -> Self {
        Self { root: root.into() }
    }

    /// The persistence directory this source watches.
    pub fn root(&self) -> &Path {
        &self.root
    }

    /// The roster: blob name -> agent name.
    fn roster(&self) -> BTreeMap<String, String> {
        let Ok(text) = std::fs::read_to_string(self.root.join("roster.json")) else {
            return BTreeMap::new();
        };
        let Ok(value) = serde_json::from_str::<Value>(&text) else {
            return BTreeMap::new();
        };
        value
            .as_object()
            .map(|entries| {
                entries
                    .iter()
                    .filter_map(|(blob, agent)| {
                        agent
                            .as_str()
                            .map(|agent| (blob.clone(), agent.to_string()))
                    })
                    .collect()
            })
            .unwrap_or_default()
    }

    /// Entry-log blobs: every JSON file that is not a reserved record.
    fn entry_logs(&self) -> Vec<(String, PathBuf)> {
        let Ok(entries) = std::fs::read_dir(&self.root) else {
            return Vec::new();
        };
        let mut found = Vec::new();
        for entry in entries.flatten() {
            let path = entry.path();
            if path.extension().and_then(|ext| ext.to_str()) != Some("json") {
                continue;
            }
            let stem = path
                .file_stem()
                .and_then(|stem| stem.to_str())
                .unwrap_or_default();
            if RESERVED_BLOBS.contains(&stem) {
                continue;
            }
            found.push((stem.to_string(), path));
        }
        found.sort();
        found
    }

    /// Discover entry-log sessions and cloud-agent stubs.
    pub fn discover(&self) -> (Vec<SessionSummary>, GrokDiscovery) {
        let roster = self.roster();
        let mut summaries = Vec::new();
        let mut stats = GrokDiscovery::default();
        for (blob, _) in self.entry_logs() {
            summaries.push(SessionSummary {
                id: blob.clone(),
                agent: Agent::GrokBot,
                ..SessionSummary::default()
            });
            if let Some(agent) = roster.get(&blob) {
                stats.attributions.insert(blob.clone(), agent.clone());
            }
            stats.entry_logs += 1;
        }
        if let Ok(text) = std::fs::read_to_string(self.root.join("cloud-agents.json")) {
            if let Ok(records) = serde_json::from_str::<Value>(&text) {
                if let Some(records) = records.as_array() {
                    for record in records {
                        let Some(agent_id) = record.get("agentId").and_then(Value::as_str) else {
                            continue;
                        };
                        summaries.push(SessionSummary {
                            id: agent_id.to_string(),
                            agent: Agent::GrokBot,
                            metadata: crate::schema::SessionMetadata {
                                status: record
                                    .get("status")
                                    .and_then(Value::as_str)
                                    .map(str::to_string),
                                pr_url: record
                                    .get("prUrl")
                                    .and_then(Value::as_str)
                                    .map(str::to_string),
                                ..crate::schema::SessionMetadata::default()
                            },
                            ..SessionSummary::default()
                        });
                        stats.cloud_agent_ids.push(agent_id.to_string());
                        stats.cloud_agents += 1;
                    }
                }
            }
        }
        (summaries, stats)
    }

    /// The doctor report: entry logs, cloud agents, and issues.
    pub fn doctor(&self) -> GrokReport {
        let discovery = self.discover().1;
        let mut report = GrokReport {
            root: self.root.clone(),
            entry_logs: discovery.entry_logs,
            cloud_agents: discovery.cloud_agents,
            issues: Vec::new(),
        };
        if !self.root.is_dir() {
            report
                .issues
                .push("persistence directory is missing".to_string());
            return report;
        }
        for (blob, path) in self.entry_logs() {
            if std::fs::read_to_string(&path)
                .map_err(|_| ())
                .and_then(|text| serde_json::from_str::<Value>(&text).map_err(|_| ()))
                .is_err()
            {
                report
                    .issues
                    .push(format!("entry log {blob} is unreadable"));
            }
        }
        report
    }

    /// Extract one entry log. Entries are ordered by their explicit `seq`;
    /// voice-call entries stay raw; a capped log flags the session as a
    /// partial local view. Returns `None` when no blob with that name
    /// exists.
    pub fn extract(&self, blob_name: &str) -> Option<GrokExtraction> {
        let path = self
            .entry_logs()
            .into_iter()
            .find(|(blob, _)| blob == blob_name)
            .map(|(_, path)| path)?;
        let text = std::fs::read_to_string(&path).ok()?;
        let log: Value = serde_json::from_str(&text).ok()?;

        let mut session = crate::session::Session {
            id: blob_name.to_string(),
            agent: Agent::GrokBot,
            ..crate::session::Session::default()
        };
        if log.get("capped").and_then(Value::as_bool) == Some(true) {
            session.partial = Some(PartialHistory {
                reason: PartialReason::LocalCap,
                detail: Some("entry log capped locally; full history is server-side".to_string()),
            });
        }

        let mut entries: Vec<(Value, u64)> = log
            .get("entries")
            .and_then(Value::as_array)
            .map(|entries| {
                entries
                    .iter()
                    .filter_map(|entry| {
                        let seq = entry.get("seq").and_then(Value::as_u64)?;
                        Some((entry.clone(), seq))
                    })
                    .collect()
            })
            .unwrap_or_default();
        // The explicit seq is the authoritative order.
        entries.sort_by_key(|(_, seq)| *seq);

        let mut good = 0;
        let mut skipped = SkipCounter::default();
        for (entry, seq) in entries {
            let ty = entry.get("type").and_then(Value::as_str).unwrap_or("");
            let mut message = match ty {
                "message" => {
                    let role = match entry.get("role").and_then(Value::as_str) {
                        Some("assistant") => Role::Assistant,
                        Some("system") => Role::System,
                        _ => Role::User,
                    };
                    let Some(text) = entry.get("text").and_then(Value::as_str) else {
                        skipped.malformed += 1;
                        continue;
                    };
                    Message {
                        role,
                        parts: vec![Part::Known(KnownPart::Text {
                            text: text.to_string(),
                            extra: None,
                        })],
                        author: entry
                            .get("author")
                            .and_then(Value::as_str)
                            .map(str::to_string),
                        ..Message::default()
                    }
                }
                "voice_call" => Message {
                    role: Role::System,
                    parts: vec![Part::Extra(entry.clone())],
                    ..Message::default()
                },
                _ => {
                    skipped.unknown += 1;
                    continue;
                }
            };
            message.timestamp = entry
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
                source_file: path.display().to_string(),
                record_type: Some(ty.to_string()),
                ordinal: Some(seq),
                ..Provenance::default()
            });
            session.messages.push(message);
            good += 1;
        }
        Some(GrokExtraction {
            session,
            good,
            skipped,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn temp_root(tag: &str) -> PathBuf {
        let root = std::env::temp_dir().join(format!("botspy-gb-{tag}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&root);
        std::fs::create_dir_all(&root).unwrap();
        root
    }

    fn write_blob(root: &Path, name: &str, value: &Value) {
        std::fs::write(root.join(format!("{name}.json")), value.to_string()).unwrap();
    }

    fn message_entry(seq: u64, role: &str, text: &str, author: Option<&str>) -> Value {
        let mut entry = json!({"seq": seq, "type": "message", "role": role, "text": text});
        if let Some(author) = author {
            entry["author"] = json!(author);
        }
        entry
    }

    #[test]
    fn discovery_lists_blobs_cloud_agents_and_roster_attributions() {
        let root = temp_root("discover");
        write_blob(
            &root,
            "abc123",
            &json!({"entries": [message_entry(1, "user", "hi", None)]}),
        );
        write_blob(
            &root,
            "def456",
            &json!({"entries": [message_entry(1, "user", "hi", None)]}),
        );
        write_blob(&root, "notes", &json!({"not": "an entry log"}));
        std::fs::write(
            root.join("roster.json"),
            json!({"abc123": "rocket"}).to_string(),
        )
        .unwrap();
        std::fs::write(
            root.join("cloud-agents.json"),
            json!([{"agentId": "peer-1", "status": "running", "prUrl": "https://github.com/x/y/pull/9"}]).to_string(),
        )
        .unwrap();
        let source = GrokBotSource::new(&root);
        let (summaries, stats) = source.discover();
        let entry_ids: Vec<&str> = summaries
            .iter()
            .filter(|summary| !stats.cloud_agent_ids.contains(&summary.id))
            .map(|summary| summary.id.as_str())
            .collect();
        assert_eq!(entry_ids, vec!["abc123", "def456", "notes"]);
        assert_eq!(stats.entry_logs, 3);
        assert_eq!(stats.cloud_agents, 1);
        assert_eq!(
            stats.attributions.get("abc123").map(String::as_str),
            Some("rocket")
        );
        let stub = summaries
            .iter()
            .find(|summary| summary.id == "peer-1")
            .unwrap();
        assert_eq!(stub.metadata.status.as_deref(), Some("running"));
        assert_eq!(
            stub.metadata.pr_url.as_deref(),
            Some("https://github.com/x/y/pull/9")
        );
    }

    #[test]
    fn seq_is_the_authoritative_order() {
        let root = temp_root("seq");
        write_blob(
            &root,
            "abc123",
            &json!({"entries": [
                message_entry(3, "user", "third", None),
                message_entry(1, "user", "first", None),
                message_entry(2, "assistant", "second", None)
            ]}),
        );
        let source = GrokBotSource::new(&root);
        let extraction = source.extract("abc123").expect("extract");
        let seqs: Vec<u64> = extraction
            .session
            .messages
            .iter()
            .map(|message| {
                message
                    .provenance
                    .as_ref()
                    .and_then(|prov| prov.ordinal)
                    .unwrap()
            })
            .collect();
        assert_eq!(seqs, vec![1, 2, 3]);
        assert_eq!(extraction.good, 3);
    }

    #[test]
    fn persona_author_and_raw_voice_calls() {
        let root = temp_root("persona");
        write_blob(
            &root,
            "abc123",
            &json!({"entries": [
                message_entry(1, "assistant", "persona hello", Some("rocket")),
                json!({"seq": 2, "type": "voice_call", "callId": "vc-1", "payload": "opaque-bytes"}),
                json!({"seq": 3, "type": "quantum_flux", "payload": "unknown"})
            ]}),
        );
        let source = GrokBotSource::new(&root);
        let extraction = source.extract("abc123").expect("extract");
        let author = extraction
            .session
            .messages
            .iter()
            .find_map(|message| message.author.clone())
            .expect("no author");
        assert_eq!(author, "rocket");
        let raw = extraction
            .session
            .messages
            .iter()
            .find_map(|message| {
                message.parts.iter().find_map(|part| match part {
                    Part::Extra(value) => Some(value.clone()),
                    _ => None,
                })
            })
            .expect("no raw part");
        assert_eq!(raw.get("type").and_then(Value::as_str), Some("voice_call"));
        assert_eq!(extraction.skipped.unknown, 1);
        assert_eq!(extraction.good, 2);
    }

    #[test]
    fn capped_entry_logs_flag_partial_local_views() {
        let root = temp_root("capped");
        let entries: Vec<Value> = (1..=200)
            .map(|seq| message_entry(seq, "user", &format!("entry seq {seq}"), None))
            .collect();
        write_blob(
            &root,
            "abc123",
            &json!({"capped": true, "continuedServerSide": true, "entries": entries}),
        );
        let source = GrokBotSource::new(&root);
        let extraction = source.extract("abc123").expect("extract");
        assert_eq!(extraction.good, 200);
        let partial = extraction.session.partial.as_ref().expect("partial flag");
        assert_eq!(partial.reason, PartialReason::LocalCap);
        assert!(partial.detail.as_deref().unwrap().contains("server-side"));
    }

    #[test]
    fn doctor_flags_unreadable_blobs() {
        let root = temp_root("doctor");
        write_blob(&root, "abc123", &json!({"entries": []}));
        std::fs::write(root.join("def456.json"), "this is { not json").unwrap();
        let source = GrokBotSource::new(&root);
        let report = source.doctor();
        assert_eq!(report.entry_logs, 2);
        assert_eq!(report.issues, vec!["entry log def456 is unreadable"]);
    }
}
