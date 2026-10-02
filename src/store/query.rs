//! The read half of the store: one query interface for the engine, ingest,
//! refresh, and search to build on. Callers iterate sessions, their
//! messages, and those messages' parts in order — without knowing or caring
//! which engine (SQLite, sqlite-vec) executes the query. No SQL string, no
//! SQLite type, and no vector index leaks through the API.

use crate::schema::{Agent, Message, Part};
use crate::session::{Session, UnknownSession};
use crate::store::text::{self, SearchHit};
use crate::store::{agent_code, SessionIter, Store, StoreError, SESSIONS_PREFIX, SESSIONS_SUFFIX};
use rusqlite::types::Value;
use rusqlite::{params, OptionalExtension};
use std::cell::Cell;
use std::rc::Rc;

/// Filters for session iteration — composable, engine-pushed-down: the
/// WHERE clauses are compiled into the existing prepared statements, so
/// the engine filters and callers never materialize everything to compare
/// properties themselves. The surface mirrors the `botspy sessions` CLI
/// flags (--source/--project/--since/--until/--limit) so a future CLI verb
/// over the store is a thin shell.
#[derive(Debug, Default, Clone, PartialEq, Eq)]
pub struct SessionFilter {
    /// Only sessions from this agent (--source).
    pub agent: Option<Agent>,
    /// Only sessions in this project (--project).
    pub project: Option<String>,
    /// Only sessions with at least one part of this kind.
    pub part_kind: Option<String>,
    /// Only sessions last active at or after this instant (--since).
    pub since: Option<String>,
    /// Only sessions last active at or before this instant (--until).
    pub until: Option<String>,
    /// At most this many sessions (--limit).
    pub limit: Option<usize>,
}

/// Filters for message iteration.
#[derive(Debug, Default, Clone, PartialEq, Eq)]
pub struct MessageFilter {
    /// Only messages with at least one part of this kind.
    pub part_kind: Option<String>,
}

/// The WHERE conjuncts (" AND …") and their values (?first_index onward)
/// for a session filter — shared by the session cursor and search.
fn filter_conjuncts(filter: &SessionFilter, first_index: usize) -> (String, Vec<Value>) {
    let mut conjuncts = String::new();
    let mut values = Vec::new();
    if let Some(agent) = &filter.agent {
        let index = first_index + values.len();
        conjuncts.push_str(&format!(" AND agent = ?{index}"));
        values.push(Value::from(agent_code(agent).to_string()));
    }
    if let Some(project) = &filter.project {
        let index = first_index + values.len();
        conjuncts.push_str(&format!(" AND project_id = ?{index}"));
        values.push(Value::from(project.clone()));
    }
    if let Some(kind) = &filter.part_kind {
        let index = first_index + values.len();
        conjuncts.push_str(&format!(
            " AND EXISTS (SELECT 1 FROM parts WHERE parts.session_id = sessions.id AND parts.kind = ?{index})"
        ));
        values.push(Value::from(kind.clone()));
    }
    if let Some(since) = &filter.since {
        let index = first_index + values.len();
        conjuncts.push_str(&format!(" AND last_activity_at >= ?{index}"));
        values.push(Value::from(since.clone()));
    }
    if let Some(until) = &filter.until {
        let index = first_index + values.len();
        conjuncts.push_str(&format!(" AND last_activity_at <= ?{index}"));
        values.push(Value::from(until.clone()));
    }
    (conjuncts, values)
}

/// The session SQL and its filter values (?3 onward) for a filter.
fn sessions_sql_and_values(filter: &SessionFilter) -> (String, Vec<Value>) {
    let (conjuncts, values) = filter_conjuncts(filter, 3);
    let sql = format!("{SESSIONS_PREFIX}{conjuncts}{SESSIONS_SUFFIX}");
    (sql, values)
}

pub(crate) const MESSAGES_PREFIX: &str = "SELECT ordinal, message_json FROM messages \
     WHERE session_id = ?1 AND (?2 IS NULL OR ordinal > ?2)";
pub(crate) const MESSAGES_SUFFIX: &str = " ORDER BY ordinal ASC LIMIT 1";
/// The message SQL and its filter values (?3 onward) for a filter.
fn messages_sql_and_values(filter: &MessageFilter) -> (String, Vec<Value>) {
    let mut sql = MESSAGES_PREFIX.to_string();
    let mut values = Vec::new();
    if let Some(kind) = &filter.part_kind {
        sql.push_str(
            " AND EXISTS (SELECT 1 FROM parts WHERE parts.session_id = messages.session_id \
             AND parts.message_ordinal = messages.ordinal AND parts.kind = ?3)",
        );
        values.push(Value::from(kind.clone()));
    }
    sql.push_str(MESSAGES_SUFFIX);
    (sql, values)
}

/// Query observability counters — the same streaming discipline the
/// importer protocol exposes (`peak_buffered`/`SkipCounter` precedent):
/// `opens` counts session materializations, `rows_fetched` counts cursor
/// rows read from the store, so specs can prove that only what was
/// consumed was touched.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct QueryCounters {
    /// Sessions materialized via [`Query::open`].
    pub opens: usize,
    /// Rows read from store cursors (session summaries, messages, parts).
    pub rows_fetched: usize,
}

pub(crate) type SharedCounters = Rc<Cell<QueryCounters>>;

/// The read half of a store: one query interface, borrowed from its store,
/// with its own observability counters. Iterators are prepared-statement
/// cursors from day one — laziness is inherent.
pub struct Query<'s> {
    store: &'s Store,
    counters: SharedCounters,
}

impl<'s> Query<'s> {
    pub(crate) fn new(store: &'s Store) -> Self {
        Query {
            store,
            counters: Rc::new(Cell::new(QueryCounters::default())),
        }
    }

    /// The counters so far: sessions materialized and rows fetched.
    pub fn counters(&self) -> QueryCounters {
        self.counters.get()
    }

    fn bump_rows(&self) {
        let mut counters = self.counters.get();
        counters.rows_fetched += 1;
        self.counters.set(counters);
    }

    /// Sessions in the store, lazily, most recent activity first (ties
    /// broken by id — the chapter-1 order). The iterator is a cursor: rows
    /// are read only as the caller consumes them.
    pub fn sessions(&self) -> SessionIter {
        self.sessions_with(&SessionFilter::default())
    }

    /// Sessions matching the filter, same lazy cursor. The engine does the
    /// filtering — the WHERE clauses are compiled into the prepared
    /// statement, never evaluated in caller memory.
    pub fn sessions_with(&self, filter: &SessionFilter) -> SessionIter {
        let (sql, values) = sessions_sql_and_values(filter);
        self.store
            .sessions_with_filter(sql, values, filter.limit, Some(Rc::clone(&self.counters)))
    }

    /// Materialize one session out of the store: the serde JSON columns
    /// rebuild the exact session the adapter produced. An unknown id is a
    /// typed error, never a silent empty result.
    pub fn open(&self, id: &str) -> Result<Session, UnknownSession> {
        match self.store.open_session(id) {
            Ok(session) => {
                let mut counters = self.counters.get();
                counters.opens += 1;
                self.counters.set(counters);
                Ok(session)
            }
            Err(StoreError::UnknownSession { id }) => Err(UnknownSession { id }),
            Err(other) => panic!("store session open failed: {other}"),
        }
    }

    /// The session's messages in source order (ordinal). The iterator is a
    /// cursor: each `next()` reads one message row, so taking the first
    /// few messages never reads the rest.
    pub fn messages(&self, session_id: &str) -> MessagesIter {
        self.messages_with(session_id, &MessageFilter::default())
    }

    /// The session's messages matching the filter, same lazy cursor, the
    /// filter compiled into the prepared statement.
    pub fn messages_with(&self, session_id: &str, filter: &MessageFilter) -> MessagesIter {
        let conn = self
            .store
            .reader_conn()
            .expect("reader connection for message iteration");
        let (sql, filter_values) = messages_sql_and_values(filter);
        MessagesIter {
            conn,
            session_id: session_id.to_string(),
            sql,
            filter_values,
            cursor: None,
            done: false,
            counters: Rc::clone(&self.counters),
        }
    }

    /// Full-text search over the stored messages (FTS5): sessions with
    /// more matching messages rank first, then better (lower) bm25 rank,
    /// then id. Each hit locates the matched part and its text. A query
    /// that matches nothing is an empty result, never an error.
    pub fn search(&self, query: &str) -> Vec<SearchHit> {
        self.search_with(query, &SessionFilter::default())
    }

    /// Full-text search composed with a session filter.
    pub fn search_with(&self, query: &str, filter: &SessionFilter) -> Vec<SearchHit> {
        let conn = self
            .store
            .reader_conn()
            .expect("reader connection for text search");
        let (conjuncts, values) = filter_conjuncts(filter, 2);
        text::search(&conn, query, &conjuncts, &values).expect("store text search failed")
    }

    /// The parts of one message, in part-ordinal order, rebuilt losslessly
    /// from the message's JSON column. `None` when the session has no such
    /// message.
    pub fn parts(
        &self,
        session_id: &str,
        message_ordinal: usize,
    ) -> Option<impl Iterator<Item = Part> + '_> {
        let conn = self
            .store
            .reader_conn()
            .expect("reader connection for parts iteration");
        let message_json: Option<String> = conn
            .query_row(
                "SELECT message_json FROM messages \
                 WHERE session_id = ?1 AND ordinal = ?2",
                params![session_id, message_ordinal as i64],
                |row| row.get(0),
            )
            .optional()
            .expect("read message row");
        let message_json = message_json?;
        self.bump_rows();
        let message: Message =
            serde_json::from_str(&message_json).expect("the stored message JSON round-trips");
        Some(message.parts.into_iter())
    }
}

/// A lazy cursor over one session's messages, in ordinal order.
pub struct MessagesIter {
    conn: rusqlite::Connection,
    session_id: String,
    sql: String,
    filter_values: Vec<Value>,
    cursor: Option<i64>,
    done: bool,
    counters: SharedCounters,
}

impl Iterator for MessagesIter {
    type Item = Message;

    fn next(&mut self) -> Option<Message> {
        if self.done {
            return None;
        }
        let after = self.cursor;
        let mut values: Vec<Value> = Vec::with_capacity(2 + self.filter_values.len());
        values.push(Value::from(self.session_id.clone()));
        values.push(after.map(Value::from).unwrap_or(Value::Null));
        values.extend(self.filter_values.iter().cloned());
        let row = self.conn.prepare_cached(&self.sql).and_then(|mut stmt| {
            stmt.query_row(rusqlite::params_from_iter(values), |row| {
                Ok((
                    row.get::<_, i64>("ordinal")?,
                    row.get::<_, String>("message_json")?,
                ))
            })
            .optional()
        });
        match row {
            Ok(Some((ordinal, message_json))) => {
                self.cursor = Some(ordinal);
                let mut counters = self.counters.get();
                counters.rows_fetched += 1;
                self.counters.set(counters);
                let message: Message = serde_json::from_str(&message_json)
                    .expect("the stored message JSON round-trips");
                Some(message)
            }
            Ok(None) => {
                self.done = true;
                None
            }
            Err(err) => panic!("store message iteration failed: {err}"),
        }
    }
}

impl std::fmt::Debug for MessagesIter {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("MessagesIter")
            .field("session_id", &self.session_id)
            .field("started", &self.cursor.is_some())
            .field("done", &self.done)
            .finish()
    }
}
