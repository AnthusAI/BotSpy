//! Adapter-driven ingestion: mine every registered adapter into the store.
//!
//! Ingestion drives the adapter contract as-is — `discover()` to find
//! sessions, `open()` to pull each one — and writes normalized rows in one
//! transaction per pass. Sources stay read-only: adapters have no write
//! surface, and the store never touches them.

use crate::adapter::Adapter;
use crate::session::{Session, UnknownSession};
use crate::store::{
    delete_session_vectors, session_content_hash, sqlite_error, upsert_session, Store, StoreError,
};
use rusqlite::{params, OptionalExtension};
use std::collections::{BTreeMap, BTreeSet};
use std::sync::Arc;

/// What one ingest or refresh pass reports, per outcome.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct IngestReport {
    /// Sessions the store had not seen before.
    pub new: usize,
    /// Sessions whose stored rows were replaced because they changed.
    pub updated: usize,
    /// Sessions already in the store and unchanged — never re-opened from
    /// their adapter.
    pub unchanged: usize,
    /// Sessions pruned because their source no longer reports them.
    pub pruned: usize,
}

impl Store {
    /// Mine every registered adapter into the store. A session reported by
    /// two adapters is stored once — the first adapter to report it wins
    /// (the chapter-1 rule) — and sessions already in the store are not
    /// re-opened from their adapters (a refresh pass handles changes).
    pub fn ingest(&self, adapters: &[Arc<dyn Adapter>]) -> Result<IngestReport, StoreError> {
        let tx = self
            .conn
            .unchecked_transaction()
            .map_err(sqlite_error("begin ingest transaction"))?;
        if let Some(embedder) = self.embedder.as_deref() {
            crate::store::check_embedding_model(&tx, embedder)?;
        }
        let stored: BTreeSet<String> = self.stored_session_ids()?;
        let mut report = IngestReport::default();
        // First report wins within the pass: the first adapter to name an
        // id owns it for this transaction.
        let mut reported: BTreeSet<String> = BTreeSet::new();
        for adapter in adapters {
            for summary in adapter.discover() {
                if !reported.insert(summary.id.clone()) {
                    continue;
                }
                if stored.contains(&summary.id) {
                    report.unchanged += 1;
                    continue;
                }
                if let Some(session) = adapter.open(&summary.id) {
                    upsert_session(&tx, &session, self.embedder.as_deref())?;
                    report.new += 1;
                }
            }
        }
        tx.commit().map_err(sqlite_error("commit ingest"))?;
        Ok(report)
    }

    /// Materialize one session out of the store: the serde JSON columns
    /// rebuild the exact session the adapter produced. An unknown id is a
    /// typed error, never a silent empty result.
    pub fn open_session(&self, id: &str) -> Result<Session, StoreError> {
        let session_json: Option<String> = self
            .conn
            .query_row(
                "SELECT session_json FROM sessions WHERE id = ?1",
                params![id],
                |row| row.get(0),
            )
            .optional()
            .map_err(sqlite_error("read session row"))?;
        let Some(session_json) = session_json else {
            return Err(StoreError::UnknownSession { id: id.to_string() });
        };
        let mut session: Session =
            serde_json::from_str(&session_json).map_err(|err| StoreError::Sqlite {
                context: format!("deserialize session {id}"),
                source: rusqlite::Error::ToSqlConversionFailure(Box::new(err)),
            })?;
        let message_jsons: Vec<String> = self
            .conn
            .prepare("SELECT message_json FROM messages WHERE session_id = ?1 ORDER BY ordinal")
            .map_err(sqlite_error("read messages"))?
            .query_map(params![id], |row| row.get(0))
            .map_err(sqlite_error("read messages"))?
            .collect::<Result<Vec<_>, _>>()
            .map_err(sqlite_error("read messages"))?;
        for message_json in message_jsons {
            let message: crate::schema::Message =
                serde_json::from_str(&message_json).map_err(|err| StoreError::Sqlite {
                    context: format!("deserialize message of session {id}"),
                    source: rusqlite::Error::ToSqlConversionFailure(Box::new(err)),
                })?;
            session.messages.push(message);
        }
        Ok(session)
    }
}

impl Store {
    pub(crate) fn stored_session_ids(&self) -> Result<BTreeSet<String>, StoreError> {
        let mut ids = BTreeSet::new();
        let mut stmt = self
            .conn
            .prepare("SELECT id FROM sessions")
            .map_err(sqlite_error("list stored sessions"))?;
        let rows = stmt
            .query_map([], |row| row.get::<_, String>(0))
            .map_err(sqlite_error("list stored sessions"))?;
        for row in rows {
            ids.insert(row.map_err(sqlite_error("list stored sessions"))?);
        }
        Ok(ids)
    }
}

impl From<UnknownSession> for StoreError {
    fn from(unknown: UnknownSession) -> Self {
        StoreError::UnknownSession { id: unknown.id }
    }
}

impl Store {
    /// Keep the store in step with its sources without re-mining
    /// everything: staleness is judged from the discover summaries
    /// (id, last_activity_at, message_count), so unchanged sessions are
    /// never opened from their adapters; changed sessions are opened and
    /// re-ingested only when the content hash actually moved; sessions the
    /// sources no longer report are pruned. One transaction per pass.
    pub fn refresh(&self, adapters: &[Arc<dyn Adapter>]) -> Result<IngestReport, StoreError> {
        let tx = self
            .conn
            .unchecked_transaction()
            .map_err(sqlite_error("begin refresh transaction"))?;
        if let Some(embedder) = self.embedder.as_deref() {
            crate::store::check_embedding_model(&tx, embedder)?;
        }
        let stored: BTreeMap<String, (String, i64, String)> = {
            let mut map = BTreeMap::new();
            let mut stmt = self
                .conn
                .prepare("SELECT id, last_activity_at, message_count, content_hash FROM sessions")
                .map_err(sqlite_error("list stored sessions"))?;
            let rows = stmt
                .query_map([], |row| {
                    Ok((
                        row.get::<_, String>(0)?,
                        row.get::<_, String>(1)?,
                        row.get::<_, i64>(2)?,
                        row.get::<_, String>(3)?,
                    ))
                })
                .map_err(sqlite_error("list stored sessions"))?;
            for row in rows {
                let (id, last_activity, message_count, content_hash) =
                    row.map_err(sqlite_error("list stored sessions"))?;
                map.insert(id, (last_activity, message_count, content_hash));
            }
            map
        };
        let mut report = IngestReport::default();
        let mut reported: BTreeSet<String> = BTreeSet::new();
        for adapter in adapters {
            for summary in adapter.discover() {
                if !reported.insert(summary.id.clone()) {
                    continue;
                }
                let stale = match stored.get(&summary.id) {
                    None => true,
                    Some((last_activity_at, message_count, _)) => {
                        *last_activity_at != summary.last_activity_at
                            || *message_count != summary.message_count as i64
                    }
                };
                if !stale {
                    report.unchanged += 1;
                    continue;
                }
                let Some(session) = adapter.open(&summary.id) else {
                    continue;
                };
                match stored.get(&summary.id) {
                    None => {
                        upsert_session(&tx, &session, self.embedder.as_deref())?;
                        report.new += 1;
                    }
                    Some((_, _, content_hash)) => {
                        let hash = session_content_hash(&session)?;
                        if hash == *content_hash {
                            report.unchanged += 1;
                            continue;
                        }
                        upsert_session(&tx, &session, self.embedder.as_deref())?;
                        report.updated += 1;
                    }
                }
            }
        }
        for id in stored.keys() {
            if reported.contains(id) {
                continue;
            }
            tx.execute("DELETE FROM message_fts WHERE session_id = ?1", params![id])
                .map_err(sqlite_error("prune session"))?;
            delete_session_vectors(&tx, id)?;
            tx.execute("DELETE FROM sessions WHERE id = ?1", params![id])
                .map_err(sqlite_error("prune session"))?;
            report.pruned += 1;
        }
        tx.commit().map_err(sqlite_error("commit refresh"))?;
        Ok(report)
    }
}
