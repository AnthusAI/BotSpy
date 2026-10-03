# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

## [0.1.0](https://github.com/AnthusAI/BotSpy/releases/tag/v0.1.0) - 2026-10-03

### Added

- *(cli)* botspy snapshot — a WAL-safe copy with proof of no mutation
- *(cli)* botspy doctor — per-source diagnostics, reports not gates
- *(cli)* botspy show — open one session and walk its messages
- *(cli)* botspy sessions — one unified listing across every source
- *(cli)* botspy sources — the source registry as a command
- *(cli)* bin target, arg parsing, store builder, executable spec steps (red)
- *(session)* unified history dedup plus executable cross-adapter spec
- *(examples)* metrics stub with docs reference
- *(examples)* local sentiment analysis with VADER-style lexicon backend
- *(examples)* thanks-vs-F-bombs meter as a public-API example consumer
- *(adapters)* Antigravity importer over transcripts DB and summaries index
- *(adapters)* Grok Bot importer over sand-client-persistence blobs
- *(adapters)* Cursor importer over snapshot-copied KV store and CLI transcripts
- *(adapters)* Codex importer with ordinal-ordered rollout extraction
- *(adapters)* Claude Code importer with discovery, extraction, and doctor
- *(spec)* unified history and example-consumer Gherkin specs (Chapter 4)
- *(spec)* per-agent importer Gherkin specs for all five agents
- *(snapshot)* read-only SQLite/WAL sources via safe snapshot copies
- *(importer)* source registry with per-source root and home overrides
- *(importer)* typed adapter protocol with streaming extraction and skip counters
- *(spec)* query API and adapter contract Gherkin specs (Chapter 2 + 3)
- *(schema)* optional message timestamps and file mtime provenance (R9)
- *(schema)* turn grouping with timing and per-message turn position (R4)
- *(schema)* raw blob escape hatch and partial-cache provenance (R12)
- *(schema)* optional session metadata on sessions and summaries (R11)
- *(schema)* role mapping conventions for native message shades (R10)
- *(schema)* inline data parts with content hashing and blob refs (R8)
- *(schema)* raw-string and structured tool-call arguments (R7)
- *(schema)* provenance widened with record id, type, ordinal, parent (R6)
- *(schema)* first-class compaction and summaries (R5)
- *(schema)* session graph — parents, roots, sub-agents, forks, battles, peers (R3)
- *(schema)* error and status flags on tool results, calls, messages, turns (R2)
- *(schema)* usage and cost metrics on messages and sessions (R1)
- add normalized schema driven by a behave feature spec
- add python package skeleton with pytest and ruff

### Fixed

- *(test)* give the discover-summaries regression test its own temp root (BOTSPY-4c8521a3 follow-up)
- *(cli)* keep the sessions table aligned in both truncation modes (BOTSPY-bc3ae3db)
- *(session)* key dedupe on (agent, id, project_id) (BOTSPY-17e4aad9)
- *(session)* pin most-recent-first listing order (BOTSPY-3ccc4fc9)
- *(cli)* validate --since/--until as RFC 3339 (BOTSPY-3f3ec558)
- *(adapters)* claude_code discover() populates summaries from transcripts (BOTSPY-4c8521a3)
- *(ci)* drive the bdd runner on a large-stack thread
- *(ci)* use setup-uv managed venv and create one for release

### Other

- milestone comments for BOTSPY-72dc40, BOTSPY-060f40, BOTSPY-e2f3fb, BOTSPY-e5be2f (filters, laziness proof, search spec, FTS5)
- *(kanbus)* BOTSPY-526284 README flip progress log
- *(readme)* replace README with a real user-facing README
- *(kanbus)* BOTSPY-ae38ee dry-run verified, stale branches cleaned
- *(kanbus)* BOTSPY-ae38ee pre-release prep progress log
- *(changelog)* drop stale semantic-release v1.0.0 section
- *(kanbus)* BOTSPY-ae38ee back to in_progress (wrongly moved to backlog)
- *(release)* commit computed bump locally in dry run
- *(board)* move unstarted open issues to Backlog
- *(kanbus)* epic BOTSPY-a6b3ed closed — all five sessions-listing bugs fixed
- *(release)* publish to crates.io on each release
- *(kanbus)* BOTSPY-4c8521a3 flake follow-up comment (temp-root collision)
- *(kanbus)* epic BOTSPY-a6b3ed wrap-up log — all five sessions-listing bugs closed
- *(kanbus)* BOTSPY-bc3ae3db closed (sessions table alignment)
- *(kanbus)* BOTSPY-17e4aad9 closed (dedupe keyed on agent+id+project)
- *(kanbus)* BOTSPY-3ccc4fc9 closed (most-recent-first order pinned)
- *(kanbus)* BOTSPY-3f3ec558 closed (--since/--until window + validation)
- *(kanbus)* BOTSPY-4c8521a3 closed (claude_code discover summaries)
- *(release)* direct releases — bump, commit, tag, GitHub release, no PR
- *(kanbus)* file sessions-listing bug epic BOTSPY-a6b3ed with five child bugs (first real-data CLI run)
- *(kanbus)* BOTSPY-5c1c7d milestone (PR #4), BOTSPY-2c0dc3 in progress
- *(kanbus)* file BOTSPY-ae38ee direct-release workflow rework (attempt A)
- *(kanbus)* BOTSPY-4df7c3 closed (spec red, @wip kept), BOTSPY-5c1c7d in progress
- *(kanbus)* close BOTSPY-493cee as superseded — direct release rework (attempt A)
- Merge pull request #3 from AnthusAI/fix/release-plz-b
- *(kanbus)* BOTSPY-4df7c3 in progress
- *(kanbus)* record Ryan's MiniLM decision, close BOTSPY-bb60bb, unblock E6
- *(kanbus)* close BOTSPY-3d9af8 — release-plz fix landed on develop
- *(kanbus)* file local-store implementation plan under BOTSPY-c7795b (7 epics, 18 tasks)
- *(release)* make release-plz work under GitFlow
- *(kanbus)* file BOTSPY-3d9af8 release-plz 403 fix (attempt A)
- *(kanbus)* file BOTSPY-493cee — release-plz PR-permission diagnosis (attempt B)
- Merge pull request #1 from AnthusAI/feat/cli-phase1
- *(kanbus)* close spec task BOTSPY-fd3c5d and log phase-1 progress
- *(kanbus)* close BOTSPY-713087 (botspy snapshot)
- *(kanbus)* close BOTSPY-727bb6 (botspy doctor)
- *(kanbus)* close BOTSPY-185f83 (botspy show)
- *(kanbus)* close BOTSPY-541f5a (botspy sessions)
- *(cli)* portable fixture homes (copy, not symlink) for the CI matrix
- *(kanbus)* close BOTSPY-62676b (botspy sources)
- *(kanbus)* spec task BOTSPY-fd3c5d in progress
- *(cli)* Gherkin specs for the five phase-1 verbs (features/cli)
- *(kanbus)* reopen CLI epic and file phase-1 tasks
- *(readme)* rewrite README as working-backwards PR FAQ with Mermaid diagrams
- *(kanbus)* sync board cache
- *(kanbus)* commit board state (issues)
- *(kanbus)* commit board state (issues)
- *(kanbus)* commit board state (issues)
- *(kanbus)* commit board state (issues)
- *(fixtures)* synthetic fixture corpus for all five agents with generator and secret guards
- *(kanbus)* commit board state (issues)
- *(readme)* Mermaid diagrams for system flow, adapter layer, and query path
- *(kanbus)* commit board state (issues)
- *(kanbus)* commit board state (issues)
- *(kanbus)* commit board state (issues)
- *(kanbus)* commit board state (issues)
- *(kanbus)* commit board state (issues)
- *(kanbus)* commit board state (issues)
- *(kanbus)* commit board state (issues)
- *(kanbus)* commit board state (issues)
- drop unused test import
- *(schema)* golden round-trip tests for part kinds, raw passthrough, and sessions
- *(kanbus)* commit board state (issues)
- *(kanbus)* commit board state (issues)
- *(kanbus)* commit board state (issues)
- *(kanbus)* commit board state (issues)
- *(kanbus)* commit board state (issues)
- *(kanbus)* commit board state (issues)
- *(kanbus)* commit board state (issues)
- *(kanbus)* commit board state (issues)
- *(kanbus)* commit board state (issues)
- *(kanbus)* commit board state (issues)
- *(kanbus)* commit board state (issues)
- *(kanbus)* commit board state (issues)
- *(kanbus)* commit board state (issues)
- *(kanbus)* commit board state (issues)
- *(kanbus)* commit board state (issues)
- *(kanbus)* commit board state (issues)
- *(kanbus)* commit board state (issues)
- *(kanbus)* sync board cache and event log for chapter restructure
- *(kanbus)* commit board state (issues)
- restructure chapters into Structure, Querying, Importers/CDC
- *(kanbus)* file 12 schema recommendation tasks with spec/impl subtasks
- *(kanbus)* commit board state (issues)
- *(kanbus)* decide fetch-on-first-launch for Local Store Initiative
- *(kanbus)* commit board state (issues)
- *(kanbus)* finish tract-only wording in Local Store Initiative
- *(kanbus)* commit board state (issues)
- *(kanbus)* drop MLX from Local Store Initiative, tract-only engine
- *(kanbus)* commit board state (issues)
- *(kanbus)* record Local Store Initiative
- *(kanbus)* commit board state (issues)
- layer-2 schema extensions from the agent storage survey
- run x86_64 macOS tests on macos-15-intel (macos-13 runners retired)
- point repository metadata at AnthusAI/BotSpy
- *(kanbus)* refresh board cache and usage events
- *(kanbus)* commit board state (issues)
- drop leftover CONTRIBUTING_AGENT template
- rustdoc chapters organized around the gherkin specs
- *(kanbus)* rework board for the Rust library pivot
- *(kanbus)* commit board state (issues)
- add cross-platform rust matrix and release-plz workflow
- [**breaking**] pivot BotSpy from Python to Rust
- *(release)* v1.0.0 [skip ci]
- *(kanbus)* restructure board specs-first with feature-area epics
- run behave in CI and simplify release workflow
- make gherkin specs the source of truth
- *(kanbus)* commit board state (issues)
- add GitHub CI workflows for ruff, pytest, and semantic-release
- initialize BotSpy project with Kanbus board and agent docs
# CHANGELOG

<!-- version list -->
