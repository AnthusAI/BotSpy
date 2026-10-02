//! Corpus walking shared by the example consumers: every session the
//! per-agent sources can reach under a fixture/data root, extracted through
//! the public botspy API.

use botspy::adapters::antigravity::AntigravitySource;
use botspy::adapters::claude_code::ClaudeCodeSource;
use botspy::adapters::codex::CodexSource;
use botspy::adapters::cursor::{CursorCliSource, CursorSource};
use botspy::adapters::grok_bot::GrokBotSource;
use botspy::Session;
use std::path::{Path, PathBuf};

/// The committed synthetic corpus, unless a data root is passed in.
pub fn default_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures")
}

/// Extract every session the sources under `root` can reach.
pub fn collect_sessions(root: &Path) -> Vec<Session> {
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
