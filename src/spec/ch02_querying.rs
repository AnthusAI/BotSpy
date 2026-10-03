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
//! (`BOTSPY-c7795b` on the BotSpy Kanbus board): SQLite +
//! sqlite-vec + all-MiniLM-L6-v2 embeddings, embedded via tract (ONNX) on
//! CPU. See that initiative for the storage, vector-search, model, and
//! performance decisions.
//!
//! ## Status: chapter 2 is green through the semantic model
//!
//! The query scenarios live in `features/02_querying/` and are embedded
//! below. Executable and green: the local store itself
//! ([`04_store.feature`](../../features/02_querying/04_store.feature)), the
//! query iteration surface
//! ([`01_iteration.feature`](../../features/02_querying/01_iteration.feature)),
//! adapter-driven ingestion
//! ([`05_ingest.feature`](../../features/02_querying/05_ingest.feature)),
//! engine-pushed-down query filters
//! ([`02_filters.feature`](../../features/02_querying/02_filters.feature)),
//! incremental refresh
//! ([`06_refresh.feature`](../../features/02_querying/06_refresh.feature)),
//! text search
//! ([`07_text_search.feature`](../../features/02_querying/07_text_search.feature)),
//! query laziness
//! ([`03_laziness.feature`](../../features/02_querying/03_laziness.feature)),
//! semantic and hybrid vector search
//! ([`08_vector_search.feature`](../../features/02_querying/08_vector_search.feature)),
//! and the semantic model's identity and paraphrase behavior
//! ([`09_semantic_model.feature`](../../features/02_querying/09_semantic_model.feature))
//! — the last two run the real MiniLM model, ungated per the
//! BOTSPY-bb60bb decision.
//!
//! ## Model assets: fetched once at setup, never at runtime
//!
//! The embedding model is not committed to the repository, and the
//! runtime library makes no network calls — ever. A one-time setup step
//! (`cargo run --example setup_models`, or the equivalent
//! [`fetch_assets`](crate::store::assets::fetch_assets) call) downloads
//! the quantized all-MiniLM-L6-v2 ONNX weights and its tokenizer exactly
//! once, verifies each file against a pinned SHA-256 digest (a
//! truncated or tampered download never lands in the cache), and caches
//! it under the models dir (`BOTSPY_MODELS` or `$HOME/.botspy/models`).
//! Setup is idempotent: already-verified files are left untouched.
//! After setup the store is fully offline — nothing leaves the machine,
//! and a missing model is a clean typed error, never a silent download.
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
    "\n```\n\n",
    "### 04_store.feature\n\n```gherkin\n",
    include_str!("../../features/02_querying/04_store.feature"),
    "\n```\n\n",
    "### 05_ingest.feature\n\n```gherkin\n",
    include_str!("../../features/02_querying/05_ingest.feature"),
    "\n```\n\n",
    "### 06_refresh.feature\n\n```gherkin\n",
    include_str!("../../features/02_querying/06_refresh.feature"),
    "\n```\n\n",
    "### 07_text_search.feature\n\n```gherkin\n",
    include_str!("../../features/02_querying/07_text_search.feature"),
    "\n```\n\n",
    "### 08_vector_search.feature\n\n```gherkin\n",
    include_str!("../../features/02_querying/08_vector_search.feature"),
    "\n```\n\n",
    "### 09_semantic_model.feature\n\n```gherkin\n",
    include_str!("../../features/02_querying/09_semantic_model.feature"),
    "\n```\n"
)]
