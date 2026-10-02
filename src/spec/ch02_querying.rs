//! # Chapter 2 — Querying
//!
//! *Spec layer 2 · mirrors `features/02_querying/` ·
//! [index](crate) ·
//! [previous: structure](crate::spec::ch01_structure) ·
//! [next: importers / CDC](crate::spec::ch03_importers)*
//!
//! Chapter 1 gives every agent's history the same shape. Chapter 2 is how
//! you search it: **one query interface, backed by SQLite and the
//! sqlite-vec extension, with the engine completely invisible to
//! callers**.
//!
//! ## One query interface
//!
//! Query functionality lives on a class or type — one entry point, with
//! different query functions on it. Callers never see the engine: there is
//! no SQLite type, no SQL string, and no vector index leaking through the
//! API. Underneath, the store is a single SQLite file (WAL mode) with the
//! sqlite-vec extension for similarity search over session embeddings —
//! brute-force cosine similarity, which is fine for thousands of session
//! embeddings.
//!
//! The design goal is transparency in both directions: a caller asks
//! "find sessions about X" (or filters by agent, project, or time) without
//! knowing or caring that SQLite is executing it, and the store can change
//! engines or indexing strategy later without changing the query surface.
//!
//! ## What it replaces
//!
//! This chapter **supersedes** the old "query and export" approach of
//! property-based filtering over in-memory objects. Holding every session
//! in memory to filter properties does not scale to the 1.4 GB Claude
//! trees and 6 GB Cursor stores the storage survey measured; the query
//! engine must be the one doing the work, invisibly.
//!
//! ## Implementation
//!
//! The implementation of this chapter is the **Local Store Initiative**
//! on the BotSpy Kanbus board
//! ([`BOTSPY-c7795b`](https://github.com/AnthusAI/BotSpy)): SQLite +
//! sqlite-vec + all-MiniLM-L6-v2 embeddings, embedded via tract (ONNX) on
//! CPU. See that initiative for the storage, vector-search, model, and
//! performance decisions.
//!
//! ## Status: specified, awaiting the local store
//!
//! The query scenarios live in `features/02_querying/` and are embedded
//! below, tagged `@wip` until the Local Store implementation lands. They
//! pin down the query surface the engine will be built to satisfy:
//! iteration over sessions, messages, and parts
//! ([`01_iteration.feature`](../../features/02_querying/01_iteration.feature)),
//! filters by source, project, part kind, and time window
//! ([`02_filters.feature`](../../features/02_querying/02_filters.feature)),
//! and laziness
//! ([`03_laziness.feature`](../../features/02_querying/03_laziness.feature)).
//!
#![doc = concat!(
    "## Behavior specification\n\n",
    "### 01_iteration.feature\n\n```gherkin\n",
    include_str!("../../features/02_querying/01_iteration.feature"),
    "\n```\n\n",
    "### 02_filters.feature\n\n```gherkin\n",
    include_str!("../../features/02_querying/02_filters.feature"),
    "\n```\n\n",
    "### 03_laziness.feature\n\n```gherkin\n",
    include_str!("../../features/02_querying/03_laziness.feature"),
    "\n```\n"
)]
