# BotSpy

BotSpy pops open the conversation history of any coding agent: one adapter per agent (Claude Code, Cursor, Codex, Grok Bot, Antigravity, more later), one standard normalized schema, and one unified agent-session history.

The product is the `botspy` Rust **library crate** — library only for now: no binary, no executable, no server, no C API/FFI, no CLI. Metrics — like the coding-session thanks-vs-F-bombs meter and local sentiment analysis — are example consumers of the library (see the "Example use cases" epic on the Kanbus board), never part of the library itself.

## Specs are the source of truth

BotSpy is a behavior-driven specification project. The Gherkin behavior specifications under `features/` are the backbone and the true source of the project; implementation code is considered generated from the specs. All planning is organized around features and their specs. Work starts by writing or refining the feature spec (with concrete examples), then the step definitions (Rust, `cucumber` crate), then the implementation. Run the specs with `cargo test --test bdd`.

## Development

```bash
cargo test              # unit tests + the bdd (Gherkin) suite
cargo test --test bdd   # only the Gherkin specs
cargo clippy --all-targets -- -D warnings
cargo fmt --all --check
```

Task management is Kanbus (see `AGENTS.md` and `CONTRIBUTING_AGENT.md`), driven by the `kbs` CLI.