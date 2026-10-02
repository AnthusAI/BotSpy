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

mod scoring;

use botspy::adapters::antigravity::AntigravitySource;
use botspy::adapters::claude_code::ClaudeCodeSource;
use botspy::adapters::codex::CodexSource;
use botspy::adapters::cursor::{CursorCliSource, CursorSource};
use botspy::adapters::grok_bot::GrokBotSource;
use botspy::Session;
use scoring::{favors_thanks, score_history, score_session};
use std::path::{Path, PathBuf};

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

fn default_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures")
}

fn collect_sessions(root: &Path) -> Vec<Session> {
    let mut sessions = Vec::new();
    let claude = root.join("claude_code/projects");
    if claude.is_dir() {
        let source = ClaudeCodeSource::new(&claude);
        for summary in source.discover().0 {
            if let Some(extraction) = source.extract(&summary.id) {
                sessions.push(extraction.session);
            }
        }
    }
    let codex = root.join("codex");
    if codex.is_dir() {
        let source = CodexSource::new(&codex);
        for summary in source.discover().0 {
            if let Some(extraction) = source.extract(&summary.id) {
                sessions.push(extraction.session);
            }
        }
    }
    let cursor_store = root.join("cursor/store/state.vscdb");
    if cursor_store.is_file() {
        let source = CursorSource::new(&cursor_store);
        for summary in source.discover().0 {
            if let Some(extraction) = source.extract(&summary.id) {
                sessions.push(extraction.session);
            }
        }
    }
    let cursor_cli = root.join("cursor/cli/projects");
    if cursor_cli.is_dir() {
        let source = CursorCliSource::new(&cursor_cli);
        for summary in source.discover().0 {
            if let Some(extraction) = source.extract(&summary.id) {
                sessions.push(extraction.session);
            }
        }
    }
    let grok = root.join("grok_bot/sand-client-persistence");
    if grok.is_dir() {
        let source = GrokBotSource::new(&grok);
        for summary in source.discover().0 {
            if let Some(extraction) = source.extract(&summary.id) {
                sessions.push(extraction.session);
            }
        }
    }
    let antigravity = root.join("antigravity");
    if antigravity.is_dir() {
        let source = AntigravitySource::new(&antigravity);
        for summary in source.discover().0 {
            if let Some(extraction) = source.extract(&summary.id) {
                sessions.push(extraction.session);
            }
        }
    }
    sessions
}
