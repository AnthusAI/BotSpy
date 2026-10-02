# BotSpy

BotSpy: a Python module that pops open the conversation history of any coding agent — one adapter per agent (Claude Code, Cursor, Codex, Grok Bot, Antigravity, more later), one standard normalized schema, and a unified agent-session history.

The product is the module itself: the adapters, the normalized schema, and the unified agent-session history. Metrics — like the coding-session thanks-vs-F-bombs meter and local sentiment analysis — are example consumers of BotSpy (see the `examples/` epic on the Kanbus board), never part of the core package's required dependencies.