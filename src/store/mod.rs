//! The local store: one SQLite file under one query interface.
//!
//! A [`Store`] is a single SQLite file in WAL mode — an out-of-process
//! scanner can write while readers query the same file. The engine stays
//! invisible: callers open a store at a path, write sessions in, and
//! iterate sessions out. Both query engines are compiled in: sqlite-vec
//! (registered as an auto extension before any connection opens) for
//! similarity search, and the bundled SQLite's FTS5 for text search.

pub mod assets;
pub mod embed;
pub mod embed_minilm;
pub mod ingest;
pub mod migrations;
pub mod query;
pub mod text;
pub mod vector;

use crate::schema::{Agent, KnownPart, Message, Part, Role};
use crate::session::{Session, SessionSummary};
use rusqlite::{params, Connection, OptionalExtension};
use std::fmt;
use std::path::{Path, PathBuf};
use std::sync::Once;
use std::time::Duration;

/// How long a connection waits on a locked store before giving up (WAL
/// lets writers and readers overlap; this covers the remaining races).
const BUSY_TIMEOUT: Duration = Duration::from_millis(5000);

/// Errors the store reports instead of panicking.
#[derive(Debug)]
pub enum StoreError {
    /// A path the store needed could not be created.
    Io {
        path: PathBuf,
        source: std::io::Error,
    },
    /// The file at the store's path is not a SQLite database.
    NotADatabase { path: PathBuf },
    /// A SQLite operation failed.
    Sqlite {
        context: String,
        source: rusqlite::Error,
    },
    /// The store file was written by an incompatible schema version.
    SchemaVersion { found: i64, expected: i64 },
    /// No BOTSPY_HOME and no $HOME to derive the default store path from.
    NoHomeDir,
    /// No session with the given id is stored.
    UnknownSession { id: String },
    /// The store already holds embeddings from another model; mixing
    /// models would make vectors incomparable, so the ingest is refused.
    EmbeddingModelMismatch { stored: String, attempted: String },
    /// The embedder failed while the store was embedding texts.
    Embed {
        source: crate::store::embed::EmbedError,
    },
}

impl fmt::Display for StoreError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            StoreError::Io { path, source } => {
                write!(f, "store path {} is unusable: {source}", path.display())
            }
            StoreError::NotADatabase { path } => write!(
                f,
                "the file at {} is not a BotSpy store (not a SQLite database)",
                path.display()
            ),
            StoreError::Sqlite { context, source } => {
                write!(f, "store error ({context}): {source}")
            }
            StoreError::SchemaVersion { found, expected } => write!(
                f,
                "store schema version {found} is not supported (expected {expected})"
            ),
            StoreError::NoHomeDir => {
                write!(f, "no home directory to derive the default store path from")
            }
            StoreError::UnknownSession { id } => write!(f, "unknown session id: {id}"),
            StoreError::EmbeddingModelMismatch { stored, attempted } => write!(
                f,
                "the store's embeddings came from model {stored:?}; ingesting with model \
                 {attempted:?} would mix models, which is refused"
            ),
            StoreError::Embed { source } => write!(f, "embedding failed: {source}"),
        }
    }
}

impl std::error::Error for StoreError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            StoreError::Io { source, .. } => Some(source),
            StoreError::Sqlite { source, .. } => Some(source),
            _ => None,
        }
    }
}

/// Register sqlite-vec once, before any connection opens: every connection
/// this process creates can then read and write vec0 tables.
fn register_sqlite_vec() {
    static ONCE: Once = Once::new();
    ONCE.call_once(|| unsafe {
        // SAFETY: sqlite-vec's documented registration — hand its C entry
        // point to sqlite3_auto_extension, which calls it with (db, errmsg,
        // api) on every new connection despite the `void (*)(void)`
        // declaration. sqlite3_vec_init matches that signature.
        let entry_point: unsafe extern "C" fn(
            *mut rusqlite::ffi::sqlite3,
            *mut *mut std::os::raw::c_char,
            *const rusqlite::ffi::sqlite3_api_routines,
        ) -> std::os::raw::c_int = std::mem::transmute(sqlite_vec::sqlite3_vec_init as *const ());
        let rc = rusqlite::ffi::sqlite3_auto_extension(Some(entry_point));
        assert_eq!(rc, 0, "sqlite3_auto_extension failed");
    });
}

fn sqlite_error(context: impl Into<String>) -> impl FnOnce(rusqlite::Error) -> StoreError {
    move |source| StoreError::Sqlite {
        context: context.into(),
        source,
    }
}

/// Map a failure that may be "not a database" to the typed error.
fn open_error(path: &Path, err: rusqlite::Error) -> StoreError {
    if err.sqlite_error_code() == Some(rusqlite::ErrorCode::NotADatabase) {
        StoreError::NotADatabase {
            path: path.to_path_buf(),
        }
    } else {
        StoreError::Sqlite {
            context: format!("open {}", path.display()),
            source: err,
        }
    }
}

/// The default store path: `BOTSPY_HOME/.botspy/store.db`, falling back to
/// `$HOME/.botspy/store.db` — the same home the CLI's snapshots live under.
pub fn default_store_path() -> Result<PathBuf, StoreError> {
    if let Some(home) = std::env::var_os("BOTSPY_HOME") {
        return Ok(PathBuf::from(home).join(".botspy").join("store.db"));
    }
    let home = std::env::var_os("HOME").ok_or(StoreError::NoHomeDir)?;
    Ok(PathBuf::from(home).join(".botspy").join("store.db"))
}

/// A connection to the local store, ready to read and write.
pub struct Store {
    path: PathBuf,
    conn: Connection,
    embedder: Option<std::sync::Arc<dyn crate::store::embed::Embedder>>,
}

impl fmt::Debug for Store {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Store").field("path", &self.path).finish()
    }
}

impl Store {
    /// Open (creating parent directories and the schema as needed) the
    /// store at `path`. A file that is not a store is a clean typed error.
    pub fn open(path: impl AsRef<Path>) -> Result<Self, StoreError> {
        let path = path.as_ref().to_path_buf();
        register_sqlite_vec();
        if let Some(parent) = path.parent() {
            if !parent.as_os_str().is_empty() {
                std::fs::create_dir_all(parent).map_err(|source| StoreError::Io {
                    path: parent.to_path_buf(),
                    source,
                })?;
            }
        }
        let conn = Connection::open(&path).map_err(|err| open_error(&path, err))?;
        Self::connect(conn, path)
    }

    /// Open the store at the default path (BOTSPY_HOME/$HOME derived).
    pub fn open_default() -> Result<Self, StoreError> {
        Self::open(default_store_path()?)
    }

    /// Open the store with an embedder configured: ingest embeds every
    /// text part into the vector index, and semantic search embeds the
    /// query with the same model. The store records the embedder's model
    /// id on first embed and refuses a different one — a store never
    /// mixes models.
    pub fn open_with_embedder(
        path: impl AsRef<Path>,
        embedder: std::sync::Arc<dyn crate::store::embed::Embedder>,
    ) -> Result<Self, StoreError> {
        let mut store = Self::open(path)?;
        store.embedder = Some(embedder);
        Ok(store)
    }

    /// The store's configured embedder, when one was set at open.
    pub(crate) fn embedder(&self) -> Option<&std::sync::Arc<dyn crate::store::embed::Embedder>> {
        self.embedder.as_ref()
    }

    /// The path this store was opened at.
    pub fn path(&self) -> &Path {
        &self.path
    }

    fn connect(conn: Connection, path: PathBuf) -> Result<Self, StoreError> {
        conn.busy_timeout(BUSY_TIMEOUT)
            .map_err(|err| open_error(&path, err))?;
        // WAL first: the journal-mode pragma reads the file header, so a
        // non-store file fails here with a clean typed error.
        let mode: String = conn
            .query_row("PRAGMA journal_mode=WAL", [], |row| row.get(0))
            .map_err(|err| open_error(&path, err))?;
        debug_assert_eq!(mode, "wal", "the store is in WAL mode");
        conn.execute_batch("PRAGMA synchronous=NORMAL; PRAGMA foreign_keys=ON;")
            .map_err(|err| open_error(&path, err))?;
        migrations::run(&conn).map_err(|err| match err {
            StoreError::Sqlite { context, source } => {
                if source.sqlite_error_code() == Some(rusqlite::ErrorCode::NotADatabase) {
                    StoreError::NotADatabase { path: path.clone() }
                } else {
                    StoreError::Sqlite { context, source }
                }
            }
            other => other,
        })?;
        Ok(Store {
            path,
            conn,
            embedder: None,
        })
    }

    /// A reader connection of its own: iteration and concurrent queries
    /// run on a separate connection so a reader never blocks the writer.
    pub(crate) fn reader_conn(&self) -> Result<Connection, StoreError> {
        let conn = Connection::open(&self.path).map_err(|err| open_error(&self.path, err))?;
        conn.busy_timeout(BUSY_TIMEOUT)
            .map_err(sqlite_error("reader connection"))?;
        conn.execute_batch("PRAGMA foreign_keys=ON;")
            .map_err(sqlite_error("reader connection"))?;
        Ok(conn)
    }

    /// Sessions in the store, lazily, most recent activity first (ties
    /// broken by id). The iterator is a cursor: it reads rows only as the
    /// caller consumes them.
    pub fn sessions(&self) -> SessionIter {
        self.sessions_with_filter(SESSIONS_PAGE_SQL.to_string(), Vec::new(), None, None)
    }

    /// The session cursor with shared observability counters (the query
    /// surface counts rows fetched for spec observability), with the
    /// filter SQL compiled in by the query surface.
    pub(crate) fn sessions_with_filter(
        &self,
        sql: String,
        filter_values: Vec<rusqlite::types::Value>,
        limit: Option<usize>,
        counters: Option<std::rc::Rc<std::cell::Cell<query::QueryCounters>>>,
    ) -> SessionIter {
        let conn = self
            .reader_conn()
            .expect("reader connection for session iteration");
        SessionIter::new(conn, sql, filter_values, limit, counters)
    }

    /// The query surface: one read interface for the engine, ingest,
    /// refresh, and search to build on, with its own observability
    /// counters. Iteration and queries run on reader connections, so a
    /// reader never blocks the writer.
    pub fn query(&self) -> query::Query<'_> {
        query::Query::new(self)
    }

    /// Write sessions into the store, replacing any earlier rows for the
    /// same session id. One transaction per call.
    pub fn ingest_sessions(
        &self,
        sessions: impl IntoIterator<Item = Session>,
    ) -> Result<(), StoreError> {
        let tx = self
            .conn
            .unchecked_transaction()
            .map_err(sqlite_error("begin ingest transaction"))?;
        for session in sessions {
            upsert_session(&tx, &session, self.embedder.as_deref())?;
        }
        tx.commit().map_err(sqlite_error("commit ingest"))?;
        Ok(())
    }
}

/// The session's own storage keys: agent and role snake_case codes that
/// match the serde names in the JSON columns.
fn agent_code(agent: &Agent) -> &'static str {
    match agent {
        Agent::ClaudeCode => "claude_code",
        Agent::Cursor => "cursor",
        Agent::Codex => "codex",
        Agent::GrokBot => "grok_bot",
        Agent::Antigravity => "antigravity",
    }
}

fn agent_from_code(code: &str) -> Option<Agent> {
    match code {
        "claude_code" => Some(Agent::ClaudeCode),
        "cursor" => Some(Agent::Cursor),
        "codex" => Some(Agent::Codex),
        "grok_bot" => Some(Agent::GrokBot),
        "antigravity" => Some(Agent::Antigravity),
        _ => None,
    }
}

fn role_code(role: &Role) -> &'static str {
    match role {
        Role::User => "user",
        Role::Assistant => "assistant",
        Role::System => "system",
        Role::Tool => "tool",
    }
}

/// The part's projected text, when the part carries plain text.
fn part_text(part: &Part) -> Option<&str> {
    match part {
        Part::Known(known) => match known {
            KnownPart::Text { text, .. } => Some(text),
            KnownPart::Thinking { text, .. } => text.as_deref(),
            KnownPart::ToolResult { text, .. } => text.as_deref(),
            KnownPart::System { text, .. } => Some(text),
            _ => None,
        },
        Part::Extra(_) => None,
    }
}

/// SHA-256 over the canonical (serde) form of the full session: any change
/// to any field changes the hash.
fn session_content_hash(session: &Session) -> Result<String, StoreError> {
    use sha2::{Digest, Sha256};
    let json = serde_json::to_vec(session).map_err(|err| StoreError::Sqlite {
        context: "serialize session for content hash".into(),
        source: rusqlite::Error::ToSqlConversionFailure(Box::new(err)),
    })?;
    let digest = Sha256::digest(&json);
    Ok(digest.iter().map(|byte| format!("{byte:02x}")).collect())
}

/// Write one session and its messages and parts (the caller is inside a
/// transaction): projected filter columns plus the lossless JSON columns.
/// With an embedder configured, the session's text parts are also
/// embedded into the vector index (the old index rows for the session are
/// removed either way, so re-ingest never leaves stale vectors).
fn upsert_session(
    tx: &Connection,
    session: &Session,
    embedder: Option<&dyn crate::store::embed::Embedder>,
) -> Result<(), StoreError> {
    let mut stripped = session.clone();
    stripped.messages.clear();
    let session_json = serde_json::to_string(&stripped).map_err(|err| StoreError::Sqlite {
        context: format!("serialize session {}", session.id),
        source: rusqlite::Error::ToSqlConversionFailure(Box::new(err)),
    })?;
    let content_hash = session_content_hash(session)?;
    tx.execute(
        "DELETE FROM message_fts WHERE session_id = ?1",
        params![session.id],
    )
    .map_err(sqlite_error("replace session"))?;
    tx.execute("DELETE FROM sessions WHERE id = ?1", params![session.id])
        .map_err(sqlite_error("replace session"))?;
    tx.execute(
        "INSERT INTO sessions (id, agent, project_id, started_at, last_activity_at, \
         message_count, content_hash, session_json) \
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)",
        params![
            session.id,
            agent_code(&session.agent),
            session.project_id,
            session.started_at,
            session.last_activity_at,
            session.messages.len() as i64,
            content_hash,
            session_json,
        ],
    )
    .map_err(sqlite_error("insert session"))?;
    for (ordinal, message) in session.messages.iter().enumerate() {
        insert_message(tx, &session.id, ordinal as i64, message)?;
    }
    delete_session_vectors(tx, &session.id)?;
    if let Some(embedder) = embedder {
        embed_session_vectors(tx, session, embedder)?;
    }
    Ok(())
}

/// Remove the session's vector-index rows (mapping rows first — their
/// presence proves the vec0 table exists).
fn delete_session_vectors(tx: &Connection, session_id: &str) -> Result<(), StoreError> {
    let rowids: Vec<i64> = tx
        .prepare("SELECT vec_rowid FROM message_vec_rows WHERE session_id = ?1")
        .map_err(sqlite_error("list vector rows"))?
        .query_map(params![session_id], |row| row.get(0))
        .map_err(sqlite_error("list vector rows"))?
        .collect::<Result<Vec<_>, _>>()
        .map_err(sqlite_error("list vector rows"))?;
    for rowid in rowids {
        tx.execute("DELETE FROM message_vec WHERE rowid = ?1", params![rowid])
            .map_err(sqlite_error("delete vector row"))?;
    }
    tx.execute(
        "DELETE FROM message_vec_rows WHERE session_id = ?1",
        params![session_id],
    )
    .map_err(sqlite_error("delete vector mapping rows"))?;
    Ok(())
}

/// Embed the session's text parts into the vector index (the caller is
/// inside a transaction, the embedder is the store's). The first embed
/// records the model id in `meta`; a different one is refused — mixing
/// models would make stored vectors incomparable.
fn embed_session_vectors(
    tx: &Connection,
    session: &Session,
    embedder: &dyn crate::store::embed::Embedder,
) -> Result<(), StoreError> {
    check_embedding_model(tx, embedder)?;
    ensure_vector_table(tx, embedder.dim())?;
    let texts: Vec<(i64, i64, String)> = session
        .messages
        .iter()
        .enumerate()
        .flat_map(|(ordinal, message)| {
            message
                .parts
                .iter()
                .enumerate()
                .filter_map(move |(part_ordinal, part)| {
                    let text = part_text(part)?;
                    Some((ordinal as i64, part_ordinal as i64, text.to_string()))
                })
        })
        .collect();
    if texts.is_empty() {
        return Ok(());
    }
    let borrowed: Vec<&str> = texts.iter().map(|(_, _, text)| text.as_str()).collect();
    let vectors = embedder
        .embed(&borrowed)
        .map_err(|source| StoreError::Embed { source })?;
    if vectors.len() != texts.len() {
        return Err(StoreError::Embed {
            source: crate::store::embed::EmbedError::Runtime {
                context: "embed session texts".to_string(),
                source: format!(
                    "the embedder returned {} vectors for {} texts",
                    vectors.len(),
                    texts.len()
                )
                .into(),
            },
        });
    }
    for ((message_ordinal, part_ordinal, _), vector) in texts.iter().zip(&vectors) {
        let bytes: Vec<u8> = vector.iter().flat_map(|f| f.to_le_bytes()).collect();
        tx.execute(
            "INSERT INTO message_vec (embedding) VALUES (?1)",
            params![bytes],
        )
        .map_err(sqlite_error("insert embedding"))?;
        let vec_rowid = tx.last_insert_rowid();
        tx.execute(
            "INSERT INTO message_vec_rows (vec_rowid, session_id, message_ordinal, part_ordinal) \
             VALUES (?1, ?2, ?3, ?4)",
            params![vec_rowid, session.id, message_ordinal, part_ordinal],
        )
        .map_err(sqlite_error("insert vector mapping row"))?;
    }
    Ok(())
}

/// Verify the store's recorded embedding model against the ingest's
/// embedder (recording it on first use), before any session work: a
/// store never mixes models, even when the ingest changes nothing.
pub(crate) fn check_embedding_model(
    conn: &Connection,
    embedder: &dyn crate::store::embed::Embedder,
) -> Result<(), StoreError> {
    match stored_embedding_model(conn)? {
        Some(stored) if stored != embedder.model_id() => Err(StoreError::EmbeddingModelMismatch {
            stored,
            attempted: embedder.model_id().to_string(),
        }),
        Some(_) => Ok(()),
        None => record_embedding_model(conn, embedder.model_id()),
    }
}

/// The embedding model recorded in `meta`, when any vectors were written.
fn stored_embedding_model(conn: &Connection) -> Result<Option<String>, StoreError> {
    use rusqlite::OptionalExtension;
    conn.query_row(
        "SELECT value FROM meta WHERE key = 'embedding_model'",
        [],
        |row| row.get(0),
    )
    .optional()
    .map_err(|err| StoreError::Sqlite {
        context: "read embedding model".into(),
        source: err,
    })
}

fn record_embedding_model(conn: &Connection, model_id: &str) -> Result<(), StoreError> {
    conn.execute(
        "INSERT INTO meta (key, value) VALUES ('embedding_model', ?1)
         ON CONFLICT(key) DO UPDATE SET value = excluded.value",
        params![model_id],
    )
    .map_err(|err| StoreError::Sqlite {
        context: "record embedding model".into(),
        source: err,
    })?;
    Ok(())
}

/// Create the vec0 table for the embedder's dimension if it does not
/// exist yet (lazy: a store that never embeds never needs it). Cosine
/// distance — MiniLM vectors are L2-normalized, so cosine similarity is
/// `1 - distance`.
pub(crate) fn ensure_vector_table(conn: &Connection, dim: usize) -> Result<(), StoreError> {
    conn.execute_batch(&format!(
        "CREATE VIRTUAL TABLE IF NOT EXISTS message_vec USING vec0(\
         embedding float[{dim}] distance_metric=cosine);"
    ))
    .map_err(|err| StoreError::Sqlite {
        context: "create vector index".into(),
        source: err,
    })
}

fn insert_message(
    tx: &Connection,
    session_id: &str,
    ordinal: i64,
    message: &Message,
) -> Result<(), StoreError> {
    let message_json = serde_json::to_string(message).map_err(|err| StoreError::Sqlite {
        context: format!("serialize message {ordinal} of {session_id}"),
        source: rusqlite::Error::ToSqlConversionFailure(Box::new(err)),
    })?;
    tx.execute(
        "INSERT INTO messages (session_id, ordinal, role, timestamp, turn_id, message_json) \
         VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
        params![
            session_id,
            ordinal,
            role_code(&message.role),
            message.timestamp,
            message.turn_id,
            message_json,
        ],
    )
    .map_err(sqlite_error("insert message"))?;
    for (part_ordinal, part) in message.parts.iter().enumerate() {
        let text = part_text(part);
        tx.execute(
            "INSERT INTO parts (session_id, message_ordinal, part_ordinal, kind, text) \
             VALUES (?1, ?2, ?3, ?4, ?5)",
            params![
                session_id,
                ordinal,
                part_ordinal as i64,
                part.kind_name().unwrap_or("extra"),
                text,
            ],
        )
        .map_err(sqlite_error("insert part"))?;
        if let Some(text) = text {
            tx.execute(
                "INSERT INTO message_fts (text, session_id, message_ordinal, part_ordinal) \
                 VALUES (?1, ?2, ?3, ?4)",
                params![text, session_id, ordinal, part_ordinal as i64],
            )
            .map_err(sqlite_error("insert fts row"))?;
        }
    }
    Ok(())
}

/// One row of the session cursor: keyset pagination on the
/// (last_activity_at, id) order, so each `next()` is one index seek and
/// rows are read only as they are consumed. `?1`/`?2` are the cursor
/// position; `?3`… are filter values appended by the query surface, which
/// splices its conjuncts between the prefix and the ORDER BY suffix.
pub(crate) const SESSIONS_PREFIX: &str =
    "SELECT id, agent, project_id, started_at, last_activity_at, \
     message_count \
     FROM sessions \
     WHERE (?1 IS NULL OR last_activity_at < ?1 OR (last_activity_at = ?1 AND id > ?2))";
pub(crate) const SESSIONS_SUFFIX: &str = " ORDER BY last_activity_at DESC, id ASC LIMIT 1";
pub(crate) const SESSIONS_PAGE_SQL: &str = concat!(
    "SELECT id, agent, project_id, started_at, last_activity_at, \
     message_count \
     FROM sessions \
     WHERE (?1 IS NULL OR last_activity_at < ?1 OR (last_activity_at = ?1 AND id > ?2))",
    " ORDER BY last_activity_at DESC, id ASC LIMIT 1"
);

/// A lazy cursor over the store's sessions, most recent activity first.
pub struct SessionIter {
    conn: Connection,
    sql: String,
    filter_values: Vec<rusqlite::types::Value>,
    cursor: Option<(String, String)>,
    done: bool,
    remaining: Option<usize>,
    counters: Option<std::rc::Rc<std::cell::Cell<query::QueryCounters>>>,
}

impl SessionIter {
    pub(crate) fn new(
        conn: Connection,
        sql: String,
        filter_values: Vec<rusqlite::types::Value>,
        limit: Option<usize>,
        counters: Option<std::rc::Rc<std::cell::Cell<query::QueryCounters>>>,
    ) -> Self {
        SessionIter {
            conn,
            sql,
            filter_values,
            cursor: None,
            done: false,
            remaining: limit,
            counters,
        }
    }
}

impl fmt::Debug for SessionIter {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("SessionIter")
            .field("started", &self.cursor.is_some())
            .field("done", &self.done)
            .finish()
    }
}

impl Iterator for SessionIter {
    type Item = SessionSummary;

    fn next(&mut self) -> Option<SessionSummary> {
        if self.done || self.remaining == Some(0) {
            self.done = true;
            return None;
        }
        let (after_activity, after_id) = match &self.cursor {
            Some((activity, id)) => (Some(activity.as_str()), Some(id.as_str())),
            None => (None, None),
        };
        let mut values: Vec<rusqlite::types::Value> =
            Vec::with_capacity(2 + self.filter_values.len());
        values.push(
            after_activity
                .map(|activity| rusqlite::types::Value::from(activity.to_string()))
                .unwrap_or(rusqlite::types::Value::Null),
        );
        values.push(
            after_id
                .map(|id| rusqlite::types::Value::from(id.to_string()))
                .unwrap_or(rusqlite::types::Value::Null),
        );
        values.extend(self.filter_values.iter().cloned());
        let row = self.conn.prepare_cached(&self.sql).and_then(|mut stmt| {
            stmt.query_row(rusqlite::params_from_iter(values), |row| {
                let agent_code: String = row.get("agent")?;
                Ok(SessionSummary {
                    id: row.get("id")?,
                    agent: agent_from_code(&agent_code).expect("stored agent code is known"),
                    project_id: row.get("project_id")?,
                    started_at: row.get("started_at")?,
                    last_activity_at: row.get("last_activity_at")?,
                    message_count: row.get::<_, i64>("message_count")? as usize,
                    metadata: Default::default(),
                    partial: None,
                })
            })
            .optional()
        });
        match row {
            Ok(Some(summary)) => {
                self.cursor = Some((summary.last_activity_at.clone(), summary.id.clone()));
                if let Some(remaining) = &mut self.remaining {
                    *remaining -= 1;
                }
                if let Some(counters) = &self.counters {
                    let mut next = counters.get();
                    next.rows_fetched += 1;
                    counters.set(next);
                }
                Some(summary)
            }
            Ok(None) => {
                self.done = true;
                None
            }
            Err(err) => panic!("store session iteration failed: {err}"),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A temp-dir store: each test gets its own directory.
    fn temp_store(tag: &str) -> Store {
        let dir = std::env::temp_dir().join(format!("botspy-store-{}-{}", tag, std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        Store::open(dir.join("store.db")).expect("store opens")
    }

    #[test]
    fn sqlite_vec_loads_and_reports_a_version() {
        let store = temp_store("vec-version");
        let version: String = store
            .conn
            .query_row("SELECT vec_version()", [], |row| row.get(0))
            .expect("vec_version()");
        assert!(version.starts_with('v'), "unexpected version {version:?}");
    }

    #[test]
    fn vec0_creates_inserts_and_answers_knn() {
        let store = temp_store("vec-knn");
        store
            .conn
            .execute_batch("CREATE VIRTUAL TABLE scratch_vec USING vec0(embedding float[4]);")
            .expect("vec0 table");
        let vectors: [([f32; 4], &str); 3] = [
            ([1.0, 0.0, 0.0, 0.0], "x"),
            ([0.0, 1.0, 0.0, 0.0], "y"),
            ([0.0, 0.0, 1.0, 0.0], "z"),
        ];
        for (vector, name) in vectors {
            let bytes: Vec<u8> = vector.iter().flat_map(|f| f.to_le_bytes()).collect();
            store
                .conn
                .execute(
                    "INSERT INTO scratch_vec (embedding) VALUES (?1)",
                    params![bytes],
                )
                .expect("vec insert");
            let _ = name;
        }
        let query: Vec<u8> = [1.0f32, 0.0, 0.0, 0.0]
            .iter()
            .flat_map(|f| f.to_le_bytes())
            .collect();
        let nearest: Vec<(i64, f64)> = store
            .conn
            .prepare("SELECT rowid, distance FROM scratch_vec WHERE embedding MATCH ?1 AND k = 2")
            .unwrap()
            .query_map(params![query], |row| Ok((row.get(0)?, row.get(1)?)))
            .unwrap()
            .map(Result::unwrap)
            .collect();
        assert_eq!(nearest.len(), 2, "k = 2 nearest neighbors");
        assert_eq!(nearest[0].0, 1, "the identical vector is nearest");
    }

    #[test]
    fn fts5_creates_and_matches_words() {
        let store = temp_store("fts5");
        store
            .conn
            .execute_batch(
                "CREATE VIRTUAL TABLE scratch_fts USING fts5(text);
                 INSERT INTO scratch_fts (text) VALUES ('fix the login bug');
                 INSERT INTO scratch_fts (text) VALUES ('add a retry loop');",
            )
            .expect("FTS5 table");
        let hits: i64 = store
            .conn
            .query_row(
                "SELECT count(*) FROM scratch_fts WHERE scratch_fts MATCH 'login'",
                [],
                |row| row.get(0),
            )
            .expect("FTS5 match");
        assert_eq!(hits, 1, "FTS5 finds the word");
    }

    #[test]
    fn opening_a_non_database_file_is_a_typed_error() {
        let dir = std::env::temp_dir().join(format!("botspy-store-notdb-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join("store.db");
        std::fs::write(&path, "this is not a sqlite database").unwrap();
        let err = Store::open(&path).expect_err("opening a non-store file fails");
        assert!(matches!(err, StoreError::NotADatabase { .. }), "{err:?}");
    }

    #[test]
    fn reopening_persists_written_sessions_in_order() {
        let store = temp_store("reopen");
        store
            .ingest_sessions(vec![
                crate::session::Session {
                    id: "b".into(),
                    agent: Agent::Cursor,
                    last_activity_at: "2026-10-01T10:00:00Z".into(),
                    ..Session::default()
                },
                crate::session::Session {
                    id: "a".into(),
                    agent: Agent::ClaudeCode,
                    last_activity_at: "2026-10-01T12:00:00Z".into(),
                    ..Session::default()
                },
            ])
            .expect("ingest");
        let reopened = Store::open(store.path()).expect("reopen");
        let ids: Vec<String> = reopened.sessions().map(|summary| summary.id).collect();
        assert_eq!(ids, vec!["a", "b"], "most recent activity first");
    }

    #[test]
    fn default_store_path_prefers_botspy_home() {
        // Only reads the env-derived path; never mutates process state.
        if let Some(home) = std::env::var_os("BOTSPY_HOME") {
            let expected = PathBuf::from(&home).join(".botspy").join("store.db");
            assert_eq!(default_store_path().unwrap(), expected);
        }
    }

    /// A text-only session with one message.
    fn text_session(id: &str, text: &str) -> crate::session::Session {
        crate::session::Session {
            id: id.to_string(),
            agent: Agent::ClaudeCode,
            project_id: "demo".to_string(),
            started_at: "2026-10-01T09:00:00Z".to_string(),
            last_activity_at: "2026-10-01T09:00:00Z".to_string(),
            messages: vec![crate::schema::Message {
                role: crate::schema::Role::User,
                parts: vec![crate::schema::Part::Known(crate::schema::KnownPart::Text {
                    text: text.to_string(),
                    extra: None,
                })],
                timestamp: Some("2026-10-01T09:00:00Z".to_string()),
                ..crate::schema::Message::default()
            }],
            ..crate::session::Session::default()
        }
    }

    #[test]
    fn embedded_ingest_answers_semantic_search() {
        let dir =
            std::env::temp_dir().join(format!("botspy-vec-{}-{}", "semantic", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        let store = Store::open_with_embedder(
            dir.join("store.db"),
            std::sync::Arc::new(embed::stub::StubEmbedder::new()),
        )
        .expect("store opens with embedder");
        store
            .ingest_sessions(vec![
                text_session("alpha", "alpha beta gamma"),
                text_session("beta", "delta epsilon zeta"),
            ])
            .expect("ingest with embeddings");
        let hits = store.query().search_semantic("alpha beta gamma", 2);
        assert_eq!(hits.len(), 2, "k = 2 nearest sessions");
        assert_eq!(hits[0].session_id, "alpha", "the identical text is nearest");
        assert!(
            (hits[0].score - 1.0).abs() < 1e-6,
            "cosine similarity of the identical vector is 1, got {}",
            hits[0].score
        );
        let hits = store.query().search_semantic("alpha beta gamma", 1);
        assert_eq!(hits.len(), 1, "k limits the sessions returned");
        assert_eq!(hits[0].session_id, "alpha");
    }

    #[test]
    fn a_store_refuses_a_different_embedding_model() {
        let dir =
            std::env::temp_dir().join(format!("botspy-vec-{}-{}", "mismatch", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        let path = dir.join("store.db");
        let store =
            Store::open_with_embedder(&path, std::sync::Arc::new(embed::stub::StubEmbedder::new()))
                .expect("store opens with embedder");
        store
            .ingest_sessions(vec![text_session("a", "alpha beta gamma")])
            .expect("first ingest embeds");
        let other = Store::open_with_embedder(
            &path,
            std::sync::Arc::new(embed::stub::StubEmbedder::with_model("other-model")),
        )
        .expect("a store with another embedder opens");
        let err = other
            .ingest_sessions(vec![text_session("b", "delta epsilon zeta")])
            .expect_err("mixing models is refused");
        assert!(
            matches!(
                err,
                StoreError::EmbeddingModelMismatch {
                    ref stored,
                    ref attempted,
                } if stored == "stub" && attempted == "other-model"
            ),
            "{err:?}"
        );
        // The refused ingest rolled back: re-opening with the original
        // embedder sees session "a" only.
        let reopened =
            Store::open_with_embedder(&path, std::sync::Arc::new(embed::stub::StubEmbedder::new()))
                .expect("reopen");
        let ids: Vec<String> = reopened.sessions().map(|summary| summary.id).collect();
        assert_eq!(ids, vec!["a"]);
    }

    #[test]
    fn without_an_embedder_text_search_works_and_semantic_is_empty() {
        let dir =
            std::env::temp_dir().join(format!("botspy-vec-{}-{}", "plain", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        let store = Store::open(dir.join("store.db")).expect("store opens");
        store
            .ingest_sessions(vec![text_session("a", "fix the login bug")])
            .expect("ingest");
        assert!(
            store.query().search_semantic("login", 5).is_empty(),
            "semantic search without an embedder is a clean empty result"
        );
        let hits = store.query().search("login");
        assert_eq!(hits.len(), 1, "text search is unaffected");
        assert!((hits[0].score - 1.0 / 61.0).abs() < 1e-12, "rank score");
        let hybrid = store.query().search_hybrid("login");
        assert_eq!(hybrid.len(), 1, "hybrid falls back to the text ranking");
    }

    #[test]
    fn hybrid_search_fuses_text_and_semantic_matches() {
        let dir =
            std::env::temp_dir().join(format!("botspy-vec-{}-{}", "hybrid", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        let store = Store::open_with_embedder(
            dir.join("store.db"),
            std::sync::Arc::new(embed::stub::StubEmbedder::new()),
        )
        .expect("store opens with embedder");
        // Session "both" text-matches "alpha" and embeds identically to
        // the query text; session "lex" only text-matches; session
        // "sem" only embeds near the query.
        store
            .ingest_sessions(vec![
                text_session("lex", "alpha nnn ooo"),
                text_session("sem", "alpha beta qqq"),
                text_session("both", "alpha beta gamma"),
            ])
            .expect("ingest");
        let hits = store.query().search_hybrid("alpha beta gamma");
        let sessions: Vec<String> = {
            let mut seen = Vec::new();
            for hit in &hits {
                if !seen.contains(&hit.session_id) {
                    seen.push(hit.session_id.clone());
                }
            }
            seen
        };
        assert_eq!(
            sessions,
            vec!["both", "sem", "lex"]
                .into_iter()
                .map(str::to_string)
                .collect::<Vec<_>>(),
            "the identical-embed session ranks first; similarity separates the rest"
        );
        for hit in &hits {
            assert!(hit.score.is_finite(), "every hybrid hit carries a score");
        }
    }
}
