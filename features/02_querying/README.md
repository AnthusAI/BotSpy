# Chapter 2 — Querying

Specs for the transparent query interface over SQLite + sqlite-vec: one
query interface, with the engine (SQLite underneath) invisible to callers.
Implementation is tracked by the Local Store Initiative on the Kanbus
board (BOTSPY-c7795b); see `src/spec/ch02_querying.rs` for the embedded
specs and status.

Spec files (all `@wip` until the Local Store implementation lands):

- `01_iteration.feature` — iteration over sessions, messages, and parts
  in order.
- `02_filters.feature` — filters by source, project, part kind, and time
  window, pushed down to the engine.
- `03_laziness.feature` — iteration reads only what the caller consumes.
- `04_store.feature` — the local store: opening at a caller-given path,
  persistence across reopens, a reader querying while an ingest commits,
  clean typed errors for files that are not a store, and the
  BOTSPY_HOME-derived default path.