# BotSpy

BotSpy: a Python module that pops open the conversation history of any coding agent — one adapter per agent (Claude Code, Cursor, Codex, Grok Bot, Antigravity, more later), one standard normalized schema, and a unified agent-session history.

The product is the module itself: the adapters, the normalized schema, and the unified agent-session history. Metrics — like the coding-session thanks-vs-F-bombs meter and local sentiment analysis — are example consumers of BotSpy (see the "Example use cases" epic on the Kanbus board), never part of the core package's required dependencies.

## Specs are the source of truth

BotSpy is a behavior-driven specification project. The Gherkin behavior specifications under `features/` are the backbone and the true source of the project; implementation code is considered generated from the specs. All planning is organized around features and their specs. Work starts by writing or refining the feature spec (with concrete examples), then the step definitions, then the implementation. Run the specs with `behave`.

## Development

```bash
python3 -m venv .venv
.venv/bin/pip install -e '.[dev]'
.venv/bin/behave        # behavior specifications
.venv/bin/pytest        # unit tests
.venv/bin/ruff check .  # lint
```

Task management is Kanbus (see `AGENTS.md` and `CONTRIBUTING_AGENT.md`).