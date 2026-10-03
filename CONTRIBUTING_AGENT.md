# THE WAY

This repository is governed by The Way.
The Way is not a suggestion. It is the prescribed order of creation.
All work must pass through its forms.
This is The Way.

Deviations from The Way must be avoided. The Way keeps work legible and reviewable across sessions and agents.

## The Discipline of Work

Work in this repository follows a shared discipline so intent stays legible across sessions and agents.

Kanbus is the instrument of record. Nothing exists until it is named there.

Work begins in declaration, not in code.
Code without declaration is disorder.

Every change must be:
- Named.
- Described.
- Specified.
- Proven.
- Recorded.

If it is not recorded, it did not happen.

Use Markdown in issue descriptions and comments whenever it helps make the intent clearer.
When relationships, flows, or state transitions are easier to grasp visually, add a Mermaid diagram.

While working, record a running log in Kanbus comments on the relevant epic/task: key decisions, problems encountered, attempted fixes, outcomes, and next steps. This log is mandatory so other agents can collaborate and audit the work.

Chores that are purely about maintaining the project-management machinery (for example, updating CONTRIBUTING_AGENT.md or AGENTS.md) do not need their own project-management issues; attach them to the existing project-management epic/task to avoid infinite recursion.

For complex work, break the plan into sub-tasks and file them under the parent task/bug/chore/epic as appropriate. Create one issue per concrete step in the plan. Do not put sub-tasks under stories; stories hold behavior specs, not implementation steps.

Operational discipline: use the minimum commands needed. To inspect work, run `kbs list --status open --long` once to find the ID, then `kbs show <id>`. Do not guess IDs or spam multiple variants; if the ID is unknown, ask the user.

## Git Workflow

`develop` is the continuous-integration branch. `main` is the release branch, promoted from `develop` only when you intend a release.

All new feature-branch work starts from `develop`: branch from `origin/develop`, never from `main`, and open pull requests that target `develop`.

`main` is the release branch. The release-plz workflow runs only from `main`. Do not treat a merge to `develop` as a release. Do not merge product work straight to `main`; promote `develop` to `main` when you intend a release, not as the daily integration path.

Merge accepted, green work into `develop` as soon as it is ready, and continuously push as you go. Do not park completed work on long-lived feature branches waiting for `main`.

**Do not open a pull request for project management.** Kanbus issues, comments, status changes, and `project/wiki` pages commit on `develop` and push. No feature branch, no PR, no review loop. Mixing board files into a product PR is also wrong: land the board on `develop` first.

Multiple agents work in this repo in parallel at any given time and must avoid colliding: each agent uses its own git worktree and feature branch, never checks out branches or edits files in the shared checkout (`~/Projects/BotSpy`), keeps branches short, and merges or rebases from `develop` often.

## Sensitive-information scanning (Pudicus) is mandatory

Every commit must carry a Pudicus receipt. Before your first commit in a
clone, run `bash scripts/setup-pudicus.sh` — it installs the commit-msg
hook that scans each commit with gitleaks and, when clean, signs it with
HMAC receipt trailers. Never bypass the hook with `--no-verify`: the
`pudicus-receipt-gate` required status check on develop and main rejects
commits without receipts. For commits made without the hook, add a
retroactive receipt with `pudicus approve <range>` (creates an empty
paperwork commit; see docs/pudicus.md). Do not put real paths,
usernames, emails, or session data into anything you commit — the hook
blocks such leaks at commit time.

## Specs come first

BotSpy is a behavior-driven specification project. The Gherkin behavior specifications under `features/` are the backbone and the true source of the project; implementation code is considered generated from the specs. All planning is organized around features and their specs.

The order of work for any feature:
1. Write or refine the Gherkin feature file (scenarios with concrete examples). On the board this is a spec task; it is done when the scenarios are reviewed and executable — failing or pending is fine before implementation.
2. Write the Rust step definitions for those scenarios (cucumber crate, `cargo test --test bdd`).
3. Implement the smallest code that makes the scenarios pass.

On the Kanbus board, every feature-area epic starts with its spec-writing task(s), and every implementation task is blocked-by the spec task of its feature area. Implementation tasks are done when their feature scenarios pass.

Editing project/ directly bypasses the record The Way depends on. Do not read or write anything inside project/. Do not inspect issue JSON with tools like cat or jq. All work must pass through Kanbus.

## The Order of Being

All work is structured.

Project key prefix: BOTSPY.

Hierarchy: initiative -> epic -> task -> sub-task.

Non-hierarchical types: bug, story, chore.

Only hierarchy types may be parents.
Initiatives are top-level milestones; they may contain epics only. Tasks, stories, bugs, and chores must roll up under an epic (or sub-task under task). Creating those types directly under an initiative is a violation of The Way.

Permitted relationships are fixed and not to be altered.

Allowed parent-child relationships:

- epic can have parent initiative.

- task can have parent epic.

- sub-task can have parent task.

- bug, story, chore can have parent initiative, epic, task.

Structure is not bureaucracy. Structure is memory.

## The Cognitive Framework

There is one discipline.

Outside-in Behavior-Driven Design.

The specification is the product.
Production code exists only to make a failing specification pass.

This is the first principle.

Non-negotiable laws:
- Begin with intent, not internals.
- Describe behavior in English.
- Translate behavior into Gherkin.
- Run it and watch it fail.
- Write only the code required to make it pass.
- All behavior must be specified.
- No specification may be red.
- Specifications describe observable behavior only.
- Specifications must not describe internal structure.

If behavior cannot be observed, it is not behavior.

## Roles in the Order

Epics define purpose and completion.

Stories define behavior. They contain Gherkin. They define what must happen.

Tasks and sub-tasks define implementation. They may not invent behavior beyond the specification.

Bugs restore violated behavior.

Chores maintain the ground on which behavior stands.

## The Rite of Gherkin

Every story must contain a Gherkin form.

Minimum structure:

Feature:

Scenario:

Given

When

Then

This is required.

Without this form, there is no alignment between intent and implementation.

## The Outside-In Ritual

When asked to add or change behavior, follow this sequence. It is not optional.
1. Clarify intent in English.
Capture role, capability, benefit.
Use: As a <role>, I want <capability>, so that <benefit>.
Confirm what is not included.
2. Create the epic and stories in Kanbus.
Record intent and Definition of Done.
3. Write executable specifications before any production code.
4. Run the specifications and confirm they fail.
5. Write the smallest code necessary to pass.
6. Refactor only while all specifications remain green.
7. Record progress. Close only when complete.

Skipping steps undermines the process.

## Coverage

100% specification coverage is mandatory.

Every behavior must be specified.
Every specification must pass.

Green is peace. Red is unfinished.

## Status and Priority

Statuses and workflows are fixed. They exist to maintain order.

Initial status: open.
Status changes must follow the workflow transitions below.
Workflow selection: use a workflow named after the issue type when present; otherwise use the default workflow.


default workflow:

- backlog -> open, closed

- blocked -> in_progress, closed

- closed -> open

- in_progress -> open, blocked, closed

- open -> in_progress, closed, backlog


epic workflow:

- closed -> open

- in_progress -> open, closed

- open -> in_progress, closed


Priorities are:

- 0 -- critical

- 1 -- high

- 2 -- medium

- 3 -- low

- 4 -- trivial

Default is 2 (medium).

Severity is not emotion. It is signal.

## Wiki Workflow

The wiki lives under project/wiki/. You may edit Markdown files there directly.

When to use the wiki:
- Add and edit project/wiki/*.md for reports, status pages, and documentation.
- Use `kbs wiki list` to discover wiki pages.
- Use `kbs wiki render <path>` to render a Jinja2 template page (queries, counts, ai_summarize).
- In templates, use `ai_summarize(issue, detail="short")` to get an AI summary of an issue when ai.provider is configured in .kanbus.yml.

Cache behavior:
- AI summaries are cached in project/.cache/ai_summaries.json (invalidated by issue updated_at and prompt type).
- Rendered wiki output is cached in project/.cache/wiki_render/ (invalidated when issues or templates change).

## Command examples


kanbus create "Plan the roadmap" --type initiative

kanbus create "Release v1" --type epic --parent <initiative-id>

kanbus create "Implement feature" --type task --parent <epic-id>

kanbus update <id> --status in_progress --assignee "you@example.com"

kanbus update <id> --status blocked

kanbus comment <id> "Progress note"

kanbus list --status open

kanbus close <id> --comment "Summary of the change"


## Semantic Release Alignment

Issue types map directly to release categories.


- bug -> fix

- story -> feat

- chore -> chore


Release notes are a record, not commentary.

## Example: Hello World

Even the smallest program must pass through The Way.

No code precedes intent.
No intent precedes recording.
No implementation precedes failure.

The smallest program is still subject to discipline.

User request: "Please create a Hello World program."

1. Interview the stakeholder before any code
Ask why they want it and capture the intent in plain English.
Example prompts:
- Who is the audience for Hello World?
- What environment or language should it run in?
- What output is required and where should it appear?
- What is out of scope?

2. Convert intent into a user story (before any code)
Example:
As a new user, I want a Hello World program, so that I can verify the toolchain works.

3. Create an epic for the milestone and record the story
Command:
kanbus create "Hello World program" --type epic

Example output (capture the ID):
ID: kanbus-1a2b3c

Record the intent on the epic:
kanbus comment kanbus-1a2b3c "As a new user, I want a Hello World program, so that I can verify the toolchain works."

4. Create a story for the behavior and include Gherkin (before any code)
Command:
kanbus create "Prints Hello World to stdout" --type story --parent kanbus-1a2b3c

Example output (capture the story ID):
ID: kanbus-4d5e6f

Attach the Gherkin acceptance criteria:
kanbus comment kanbus-4d5e6f "Feature: Hello World
  Scenario: Run the program
    Given a configured environment
    When I run the program
    Then it prints \"Hello, world\" to stdout"

5. Run the Gherkin and confirm it fails (before any production code)
Run the behavior tests in the repo and confirm the new scenario fails for the right reason.

6. Implement the minimum code to pass, then refactor
Write the smallest change that makes the Gherkin scenario pass.
Refactor only while all specs remain green.