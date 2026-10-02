//! Stub showing where further session metrics would live. Every metric is an
//! example consumer like the meter and sentiment examples: it imports only
//! the public botspy API and the standard library, reads sessions through the
//! per-agent sources, and never joins the core dependency graph.
//!
//! ```text
//! cargo run --example metrics_stub [path-to-agent-data-root]
//! ```
//!
//! Candidate metrics for this spot: code-time share per session,
//! interruption count (user → assistant handbacks), topic drift across a
//! session, turn latency distribution. None are implemented yet — this stub
//! is the extension point and the pattern to copy.

#[path = "../common/mod.rs"]
mod common;

use common::{collect_sessions, default_root};
use std::path::PathBuf;

/// The extension point a real metric implements: inspect one normalized
/// session through the public API and report whatever it measures.
trait SessionMetric {
    /// The metric's display name.
    fn name(&self) -> &'static str;
    /// One-line report for one session.
    fn score(&self, session: &botspy::Session) -> String;
}

/// Placeholder implementation: no metric yet, the session just counts.
struct StubMetric;

impl SessionMetric for StubMetric {
    fn name(&self) -> &'static str {
        "stub (no metric implemented yet)"
    }

    fn score(&self, session: &botspy::Session) -> String {
        format!(
            "{} messages available to a future metric",
            session.messages.len()
        )
    }
}

fn main() {
    let root = std::env::args()
        .nth(1)
        .map(PathBuf::from)
        .unwrap_or_else(default_root);
    let sessions = collect_sessions(&root);
    let metric = StubMetric;
    println!(
        "Session-metric stub ({}) over {} sessions under {}:",
        metric.name(),
        sessions.len(),
        root.display()
    );
    for session in &sessions {
        println!(
            "  {:?} {}: {}",
            session.agent,
            session.id,
            metric.score(session)
        );
    }
    println!(
        "This is a placeholder: real metrics would live beside the meter \
         and sentiment examples, as public-API-only consumers."
    );
}
