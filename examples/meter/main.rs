//! The coding-session thanks-vs-F-bombs meter: an example consumer of the
//! BotSpy public API. It walks a fixture root (default: the committed
//! synthetic corpus under `tests/fixtures/`), extracts every session it can
//! reach through the per-agent sources, and scores the conversation.
//!
//! ```text
//! cargo run --example meter [path-to-agent-data-root]
//! ```
//!
//! This example lives outside the core dependency graph: it imports only
//! `botspy` and the standard library.

#[path = "../common/mod.rs"]
mod common;
mod scoring;

use common::{collect_sessions, default_root};
use scoring::{favors_thanks, score_history, score_session};
use std::path::PathBuf;

fn main() {
    let root = std::env::args()
        .nth(1)
        .map(PathBuf::from)
        .unwrap_or_else(default_root);
    let sessions = collect_sessions(&root);
    let report = score_history(&sessions);
    println!(
        "Scoring {} sessions under {}:",
        sessions.len(),
        root.display()
    );
    for session in &sessions {
        let (thanks, f_bombs) = score_session(session);
        println!(
            "  {:?} {}: {thanks} thanks, {f_bombs} F-bombs",
            session.agent, session.id
        );
    }
    println!(
        "Meter: {} thanks vs {} F-bombs across {} sessions — the ratio {} thanks.",
        report.thanks,
        report.f_bombs,
        report.sessions,
        if favors_thanks(&report) {
            "favors"
        } else {
            "does not favor"
        }
    );
}
