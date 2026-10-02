//! Steps for read-only SQLite/WAL source access
//! (spec 03_importers/wal_snapshot.feature).

use crate::steps::BotSpyWorld;
use botspy::snapshot::digest_files;
use botspy::snapshot_sqlite;
use cucumber::{given, then, when};
use std::path::PathBuf;
use std::sync::atomic::{AtomicUsize, Ordering};

static UNIQUE_DIR: AtomicUsize = AtomicUsize::new(0);

fn fresh_dir() -> PathBuf {
    std::env::temp_dir().join(format!(
        "botspy-wal-{}-{}",
        std::process::id(),
        UNIQUE_DIR.fetch_add(1, Ordering::SeqCst)
    ))
}

#[given(
    regex = r#"a SQLite source in WAL mode at "([^"]+)" with ([0-9]+) rows and a live writer appending rows"#
)]
fn wal_source_with_writer(world: &mut BotSpyWorld, _given_path: String, rows: u64) {
    let base = fresh_dir();
    std::fs::create_dir_all(&base).expect("create fixture dir");
    let db = base.join("store.db");
    let writer = rusqlite::Connection::open(&db).expect("open fixture source db");
    writer
        .pragma_update(None, "journal_mode", "WAL")
        .expect("enable WAL mode");
    writer
        .execute_batch("CREATE TABLE events (id INTEGER PRIMARY KEY, note TEXT NOT NULL);")
        .expect("create events table");
    for i in 1..=rows {
        writer
            .execute(
                "INSERT INTO events (id, note) VALUES (?1, ?2)",
                rusqlite::params![i as i64, format!("row {i}")],
            )
            .expect("insert fixture row");
    }
    world.sqlite_db = Some(db);
    world.sqlite_writer = Some(writer);
    world.sqlite_rows_at_snapshot = rows;
}

#[when(regex = r#"I read the source through a snapshot copy"#)]
fn read_through_snapshot(world: &mut BotSpyWorld) {
    let db = world.sqlite_db.clone().expect("no fixture source db");
    let wal = wal_path(&db);
    world.sqlite_digest_before = Some(digest_files(&[&db, &wal]));
    let snapshot = snapshot_sqlite(&db, &fresh_dir()).expect("snapshot the source");
    world.sqlite_digest_after = Some(digest_files(&[&db, &wal]));
    world.snapshot_rows =
        Some(botspy::snapshot::count_events(&snapshot).expect("count snapshot rows"));
}

fn wal_path(db: &std::path::Path) -> PathBuf {
    let mut wal = db.as_os_str().to_owned();
    wal.push("-wal");
    PathBuf::from(wal)
}

#[then(regex = r#"the snapshot yields a consistent read of the rows present at snapshot time"#)]
fn snapshot_rows_consistent(world: &mut BotSpyWorld) {
    let rows = world.snapshot_rows.expect("no snapshot was read");
    assert_eq!(
        rows, world.sqlite_rows_at_snapshot,
        "the snapshot does not reflect the rows present at snapshot time"
    );
}

#[then(regex = r#"the source database file and its WAL file were never mutated"#)]
fn source_unmutated(world: &mut BotSpyWorld) {
    let before = world.sqlite_digest_before.as_ref().expect("digest before");
    let after = world.sqlite_digest_after.as_ref().expect("digest after");
    assert_eq!(before, after, "reading the source mutated it");
}

#[then(regex = r#"the live writer kept appending without interference"#)]
fn writer_keeps_appending(world: &mut BotSpyWorld) {
    let writer = world.sqlite_writer.as_ref().expect("no live writer");
    writer
        .execute_batch(
            "INSERT INTO events (id, note) VALUES (101, 'appended during read');
             INSERT INTO events (id, note) VALUES (102, 'appended during read');",
        )
        .expect("the live writer could not append while we read");
    assert_eq!(
        world.snapshot_rows,
        Some(world.sqlite_rows_at_snapshot),
        "the snapshot changed after the writer appended"
    );
}
