//! The read half of the store: one query interface for the engine, ingest,
//! refresh, and search to build on. Callers iterate sessions, their
//! messages, and those messages' parts in order — without knowing or caring
//! which engine (SQLite, sqlite-vec) executes the query. No SQL string, no
//! SQLite type, and no vector index leaks through the API.

use crate::schema::{Message, Part};
use crate::session::{Session, UnknownSession};
use crate::store::{SessionIter, Store, StoreError};
use rusqlite::{params, OptionalExtension};
use std::cell::Cell;
use std::rc::Rc;

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
        self.store
            .sessions_counting(Some(Rc::clone(&self.counters)))
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
        let conn = self
            .store
            .reader_conn()
            .expect("reader connection for message iteration");
        MessagesIter {
            conn,
            session_id: session_id.to_string(),
            cursor: None,
            done: false,
            counters: Rc::clone(&self.counters),
        }
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
    cursor: Option<i64>,
    done: bool,
    counters: SharedCounters,
}

const MESSAGES_PAGE_SQL: &str = "SELECT ordinal, message_json FROM messages \
     WHERE session_id = ?1 AND (?2 IS NULL OR ordinal > ?2) \
     ORDER BY ordinal ASC \
     LIMIT 1";

impl Iterator for MessagesIter {
    type Item = Message;

    fn next(&mut self) -> Option<Message> {
        if self.done {
            return None;
        }
        let after = self.cursor;
        let row = self
            .conn
            .prepare_cached(MESSAGES_PAGE_SQL)
            .and_then(|mut stmt| {
                stmt.query_row(params![self.session_id, after], |row| {
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
