//! Local sentiment analysis over BotSpy sessions: an example consumer of the
//! public API. The default backend is the VADER-style valence lexicon in
//! `scoring.rs`, fully in-process — message text never leaves the machine.
//!
//! ```text
//! cargo run --example sentiment [path-to-agent-data-root]
//! ```
//!
//! The optional GoEmotions transformer backend is planned behind an optional
//! `botspy[metrics]` extra: it would run locally (e.g. via a bundled ONNX
//! model) and stay out of the core dependency graph. This binary documents
//! the seam and keeps the lexicon backend as the always-available default.
//!
//! Like every example, this imports only `botspy` and the standard library.

#[path = "../common/mod.rs"]
mod common;
mod scoring;

use common::{collect_sessions, default_root};
use scoring::{score_history, score_session};
use std::path::PathBuf;

fn main() {
    let root = std::env::args()
        .nth(1)
        .map(PathBuf::from)
        .unwrap_or_else(default_root);
    let sessions = collect_sessions(&root);
    let report = score_history(&sessions);
    println!(
        "Sentiment (VADER, local) over {} sessions under {}:",
        sessions.len(),
        root.display()
    );
    for session in &sessions {
        let session_report = score_session(session);
        println!(
            "  {:?} {}: {p} positive, {n} negative, {ne} neutral",
            session.agent,
            session.id,
            p = session_report.positive,
            n = session_report.negative,
            ne = session_report.neutral
        );
    }
    println!(
        "Totals: {} positive, {} negative, {} neutral messages. \
         Everything scored in-process; no text left the machine.",
        report.positive, report.negative, report.neutral
    );
    println!(
        "Note: the GoEmotions transformer backend is planned behind an \
         optional botspy[metrics] extra; the local VADER lexicon is the default."
    );
}
