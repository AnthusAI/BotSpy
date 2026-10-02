//! Safe reading of SQLite sources, including live WAL databases.
//!
//! Agents own their databases; BotSpy only ever reads them. A SQLite
//! source in WAL mode — a live app writing while we read — is read
//! through a snapshot copy taken over a read-only connection, so the
//! source database and its WAL are never mutated and the writer never
//! blocks on us.

use rusqlite::backup::Backup;
use rusqlite::{Connection, OpenFlags};
use std::fmt;
use std::path::{Path, PathBuf};
use std::time::Duration;

/// Flags for opening a source database strictly read-only.
const READ_ONLY: OpenFlags = OpenFlags::SQLITE_OPEN_READ_ONLY;

/// Opening or snapshotting a source database failed.
#[derive(Debug)]
pub enum SnapshotError {
    Sqlite(rusqlite::Error),
    Io(std::io::Error),
}

impl fmt::Display for SnapshotError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Sqlite(err) => write!(f, "sqlite source error: {err}"),
            Self::Io(err) => write!(f, "snapshot io error: {err}"),
        }
    }
}

impl std::error::Error for SnapshotError {}

/// Copy a SQLite database into `snapshot_dir/snapshot.db` through a
/// read-only connection to `source`, using the online backup API. The
/// source (main file, WAL, and shm) is never written to, and a concurrent
/// writer keeps working throughout.
pub fn snapshot_sqlite(source: &Path, snapshot_dir: &Path) -> Result<PathBuf, SnapshotError> {
    std::fs::create_dir_all(snapshot_dir).map_err(SnapshotError::Io)?;
    let snapshot_path = snapshot_dir.join("snapshot.db");
    let _ = std::fs::remove_file(&snapshot_path);
    let source_conn =
        Connection::open_with_flags(source, READ_ONLY).map_err(SnapshotError::Sqlite)?;
    let mut snapshot_conn = Connection::open(&snapshot_path).map_err(SnapshotError::Sqlite)?;
    let backup = Backup::new(&source_conn, &mut snapshot_conn).map_err(SnapshotError::Sqlite)?;
    backup
        .run_to_completion(64, Duration::from_millis(5), None)
        .map_err(SnapshotError::Sqlite)?;
    Ok(snapshot_path)
}

/// Content digest over the given files, in order. Missing files digest
/// distinctly from empty ones. Used to prove a source was not mutated
/// by a read.
pub fn digest_files(paths: &[&Path]) -> String {
    use sha2::Digest;
    let mut hasher = sha2::Sha256::new();
    for path in paths {
        hasher.update(path.to_string_lossy().as_bytes());
        match std::fs::read(path) {
            Ok(bytes) => {
                hasher.update([1]);
                hasher.update(&bytes);
            }
            Err(_) => hasher.update([0]),
        }
    }
    hasher
        .finalize()
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect()
}

/// Open a snapshot (or any of our own copies) read-only and count the
/// rows of the `events` table the fixture sources use.
pub fn count_events(db: &Path) -> Result<u64, SnapshotError> {
    let conn = Connection::open_with_flags(db, READ_ONLY).map_err(SnapshotError::Sqlite)?;
    let count: i64 = conn
        .query_row("SELECT COUNT(*) FROM events", [], |row| row.get(0))
        .map_err(SnapshotError::Sqlite)?;
    Ok(count.max(0) as u64)
}

/// Count the rows of a snapshot across the table names the sources use:
/// `events` for the fixture sources, `cursorDiskKV` for Cursor's KV
/// store. The first table that exists wins.
pub fn count_rows(db: &Path) -> Result<u64, SnapshotError> {
    let conn = Connection::open_with_flags(db, READ_ONLY).map_err(SnapshotError::Sqlite)?;
    for table in ["events", "cursorDiskKV"] {
        let exists = conn
            .query_row(
                "SELECT 1 FROM sqlite_master WHERE type = 'table' AND name = ?1",
                [table],
                |row| row.get::<_, i64>(0),
            )
            .map(Some)
            .or_else(|err| match err {
                rusqlite::Error::QueryReturnedNoRows => Ok(None),
                err => Err(err),
            })
            .map_err(SnapshotError::Sqlite)?;
        if exists.is_none() {
            continue;
        }
        let count: i64 = conn
            .query_row(&format!("SELECT COUNT(*) FROM {table}"), [], |row| {
                row.get(0)
            })
            .map_err(SnapshotError::Sqlite)?;
        return Ok(count.max(0) as u64);
    }
    Err(SnapshotError::Sqlite(rusqlite::Error::InvalidQuery))
}
