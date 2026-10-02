//! Versioned schema migrations for the local store.
//!
//! The DDL is embedded: a store file carries its schema version in the
//! `meta` table and migrates itself when it is opened. v1 lays down the
//! normalized relational shape — sessions, messages, and parts with
//! projected filter columns plus lossless serde JSON columns.

use rusqlite::Connection;

use super::StoreError;

/// The schema version this build of the store reads and writes.
pub(crate) const SCHEMA_VERSION: i64 = 1;

/// The v1 DDL: normalized rows plus projected filter columns. Message and
/// part rows hang off their session and cascade away with it; the serde
/// JSON columns keep the records lossless.
const DDL_V1: &str = r#"
CREATE TABLE sessions (
    id TEXT PRIMARY KEY,
    agent TEXT NOT NULL,
    project_id TEXT NOT NULL,
    started_at TEXT NOT NULL,
    last_activity_at TEXT NOT NULL,
    message_count INTEGER NOT NULL,
    content_hash TEXT NOT NULL,
    session_json TEXT NOT NULL
);
CREATE TABLE messages (
    session_id TEXT NOT NULL REFERENCES sessions(id) ON DELETE CASCADE,
    ordinal INTEGER NOT NULL,
    role TEXT NOT NULL,
    timestamp TEXT,
    turn_id TEXT,
    message_json TEXT NOT NULL,
    PRIMARY KEY (session_id, ordinal)
);
CREATE TABLE parts (
    session_id TEXT NOT NULL,
    message_ordinal INTEGER NOT NULL,
    part_ordinal INTEGER NOT NULL,
    kind TEXT NOT NULL,
    text TEXT,
    PRIMARY KEY (session_id, message_ordinal, part_ordinal),
    FOREIGN KEY (session_id, message_ordinal)
        REFERENCES messages(session_id, ordinal) ON DELETE CASCADE
);
CREATE INDEX idx_sessions_last_activity ON sessions(last_activity_at);
CREATE INDEX idx_sessions_agent ON sessions(agent);
CREATE INDEX idx_sessions_project ON sessions(project_id);
CREATE INDEX idx_parts_kind ON parts(kind);
CREATE VIRTUAL TABLE message_fts USING fts5(
    text,
    session_id UNINDEXED,
    message_ordinal UNINDEXED,
    part_ordinal UNINDEXED
);
"#;

/// Create the `meta` table (key/value settings) when it does not exist.
fn ensure_meta(conn: &Connection) -> Result<(), rusqlite::Error> {
    conn.execute_batch(
        "CREATE TABLE IF NOT EXISTS meta (
            key TEXT PRIMARY KEY,
            value TEXT NOT NULL
        );",
    )
}

/// The stored schema version, or `None` when the store has no schema yet.
fn stored_version(conn: &Connection) -> Result<Option<i64>, StoreError> {
    use rusqlite::OptionalExtension;
    conn.query_row(
        "SELECT value FROM meta WHERE key = 'schema_version'",
        [],
        |row| row.get::<_, String>(0),
    )
    .optional()
    .map_err(|err| StoreError::Sqlite {
        context: "read schema version".into(),
        source: err,
    })?
    .map(|text| {
        text.parse::<i64>().map_err(|_| StoreError::Sqlite {
            context: format!("schema version {text:?} is not a number"),
            source: rusqlite::Error::InvalidQuery,
        })
    })
    .transpose()
}

/// Bring the store's schema up to [`SCHEMA_VERSION`].
pub(crate) fn run(conn: &Connection) -> Result<(), StoreError> {
    ensure_meta(conn).map_err(|err| StoreError::Sqlite {
        context: "create meta table".into(),
        source: err,
    })?;
    match stored_version(conn)? {
        None => {
            conn.execute_batch(DDL_V1)
                .map_err(|err| StoreError::Sqlite {
                    context: "apply schema v1".into(),
                    source: err,
                })?;
            set_version(conn, SCHEMA_VERSION)?;
            Ok(())
        }
        Some(version) if version == SCHEMA_VERSION => Ok(()),
        Some(found) => Err(StoreError::SchemaVersion {
            found,
            expected: SCHEMA_VERSION,
        }),
    }
}

fn set_version(conn: &Connection, version: i64) -> Result<(), StoreError> {
    use rusqlite::params;
    conn.execute(
        "INSERT INTO meta (key, value) VALUES ('schema_version', ?1)
         ON CONFLICT(key) DO UPDATE SET value = excluded.value",
        params![version.to_string()],
    )
    .map_err(|err| StoreError::Sqlite {
        context: "record schema version".into(),
        source: err,
    })?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn fresh_store_gets_schema_v1() {
        let conn = Connection::open_in_memory().unwrap();
        run(&conn).unwrap();
        assert_eq!(stored_version(&conn).unwrap(), Some(SCHEMA_VERSION));
    }

    #[test]
    fn rerunning_migration_is_a_no_op() {
        let conn = Connection::open_in_memory().unwrap();
        run(&conn).unwrap();
        run(&conn).unwrap();
        assert_eq!(stored_version(&conn).unwrap(), Some(SCHEMA_VERSION));
    }
}
