//! # Chapter 5 — The CLI
//!
//! *Spec layer 5 · mirrors `features/cli/` ·
//! [index](crate) ·
//! [previous: examples](crate::spec::ch04_examples)*
//!
//! The `botspy` command line is a **thin shell over the library**: every
//! verb is one library call plus rendering — no business logic in the
//! binary. The CLI owns flags, exit codes, and formatting; the library
//! owns behavior. Read-only, always: the CLI never writes to the sources
//! it taps.
//!
//! Exit codes: `0` success (including empty results and skipped
//! records), `1` target not found (unknown session id or unknown
//! source), `2` usage error (bad flags), `3` source failure (unreadable
//! root/store, snapshot error).
//!
//! Output modes: `-o human` (default; truncates ids and paths, stays
//! plain and pipe-tolerant), `-o json` (serde of the library's own
//! types), `-o ndjson` (one record per line, streaming). Machine modes
//! never truncate.
//!
//! ## Status: phase 1 (read-only MVP)
//!
//! The scenarios in `features/cli/` cover the five read-only verbs —
//! [`sources`](../../features/cli/10_sources.feature),
//! [`sessions`](../../features/cli/20_sessions.feature),
//! [`show`](../../features/cli/30_show.feature),
//! [`doctor`](../../features/cli/40_doctor.feature), and
//! [`snapshot`](../../features/cli/50_snapshot.feature) — plus the
//! global command contract
//! ([`00_global.feature`](../../features/cli/00_global.feature)):
//! version, usage errors, and the `BOTSPY_HOME` override. Later phases
//! (import, stats, search, watch) extend this chapter; `export` stays
//! out entirely (YAGNI).
//!
#![doc = concat!(
    "## Behavior specification\n\n",
    "### 00_global.feature\n\n```gherkin\n",
    include_str!("../../features/cli/00_global.feature"),
    "\n```\n\n",
    "### 10_sources.feature\n\n```gherkin\n",
    include_str!("../../features/cli/10_sources.feature"),
    "\n```\n\n",
    "### 20_sessions.feature\n\n```gherkin\n",
    include_str!("../../features/cli/20_sessions.feature"),
    "\n```\n\n",
    "### 30_show.feature\n\n```gherkin\n",
    include_str!("../../features/cli/30_show.feature"),
    "\n```\n\n",
    "### 40_doctor.feature\n\n```gherkin\n",
    include_str!("../../features/cli/40_doctor.feature"),
    "\n```\n\n",
    "### 50_snapshot.feature\n\n```gherkin\n",
    include_str!("../../features/cli/50_snapshot.feature"),
    "\n```\n"
)]