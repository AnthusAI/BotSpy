# Pudicus: sensitive-information gate

BotSpy uses [Pudicus](https://pypi.org/project/pudicus/) to keep
secrets and other sensitive information out of the source code. It
works in two layers, like an agricultural inspector's produce sticker:

1. **Local hook (pre-merge).** A `commit-msg` hook runs the configured
   scanners against every commit. If the commit is clean, Pudicus mints
   an HMAC receipt and appends it to the commit message as trailers.
   If a scanner finds something, the commit is blocked.
2. **CI gate (pull requests).** The `pudicus-receipt-gate` check
   (required for merging into `develop`; see below) re-verifies the receipts of every commit new
   to the base branch and re-runs the scan, so a commit made with
   `--no-verify` cannot land.

## Setup (once per clone)

```bash
bash scripts/setup-pudicus.sh
```

The script installs a pinned `pudicus` release into a per-user venv
(`~/.pudicus/venv`), writes the `commit-msg` hook into the repository's
common git directory, and creates the shared HMAC secret at
`~/.config/pudicus/secret` on first use. Linked worktrees share hooks
through the common git dir, so one run covers every worktree of a
clone. Re-run it after moving the Python environment or installing a
checker binary in a new location; force a hook reinstall with
`PUDICUS_REINSTALL=1`.

Requirements: `git`, `python3`, and [gitleaks](https://github.com/gitleaks/gitleaks)
(e.g. `brew install gitleaks`).

## Receipts

Every receipt is a block of Git trailers appended to the commit
message:

```text
Inspected-by: pudicus-v1
Inspection-tree: f18588e0c5f5b0a5ba281b5a78242127919d2388
Inspection-result: clean
Inspection-at: 2026-08-31T17:01:34Z
Inspection-sig: hmac-sha256:8cce6e3714782d025eb4ec4aec755...
```

The signature is keyed to the **tree hash** (the code content), not the
commit SHA, so receipts survive history rewrites (such as
`git filter-repo` runs that preserve commit messages). Verification
pools signatures: `pudicus verify <range>` accepts a tree if any commit
in the range carries a valid receipt for it.

Every commit must be created with the hook active. A commit made with
`--no-verify` (or by a tool that skips hooks) has no receipt; the CI
gate will reject it. To receipt existing unsigned commits, add a
retroactive approval commit:

```bash
pudicus approve origin/develop..HEAD   # empty "paperwork" commit holding the receipts
git push
```

`pudicus approve` re-runs the scanners over each commit in the range
before signing it.

## Scope of the CI gate

The gate (`scripts/pudicus_gate.sh`, invoked by
`.github/workflows/pudicus-gate.yml`) verifies **only commits that are
new to the base branch**: `merge-base(origin/<base>, HEAD)..HEAD`.
History that predates the gate is grandfathered and never re-verified —
this keeps old commits (and open PRs created before the gate existed)
from being retroactively blocked; a PR's own new commits are always
checked.

Two kinds of new commits are **exempt** from the receipt requirement:

- **Merge commits.** GitHub's merge button creates them server-side,
  where local hooks cannot run. (A merge commit's tree is also
  derivable from its already-receipted parents' content.)
- **Release-bot commits** authored by `github-actions[bot]` or
  `release-plz[bot]`. Releases no longer produce such commits (version
  bumps are made locally by `scripts/prepare-release.sh` and signed by
  the hook), so this exemption is now vestigial; it is kept in
  `scripts/pudicus_gate.sh` because it is harmless.

The gitleaks re-scan step is *not* exempt for anything: it covers the
entire PR diff.

The gate needs the shared HMAC secret to verify receipts; it is
configured as the repository secret `PUDICUS_SECRET` and restored to
`~/.config/pudicus/secret` by the workflow. Every machine that runs
`pudicus verify` (or mints receipts) must hold the same secret — share
it only with trusted environments.

## Required status check

The check name is `pudicus-receipt-gate`. It runs on every pull request
to `develop` and `main`. It is a **required status check on both `develop` and `main`**,
enforced by repository rulesets: a pull request cannot merge into either
branch until the check passes.

- **Admin bypass.** The rulesets' bypass actors are repository admins
  (bypass mode "always"). AGENTS.md lands board-state commits directly
  on `develop` without a PR; those pushes keep working for admins. The
  local commit-msg hook still scans and signs them.
- **Releases never push to `main`.** `.github/workflows/release.yml`
  only tags, publishes to crates.io, and creates the GitHub release
  (tags are not covered by branch rulesets). The version bump is a
  normal commit made on `develop` with `scripts/prepare-release.sh`, so
  it reaches `main` through the develop to main promotion pull request
  like any other change. GitHub refuses GitHub Actions as a ruleset
  bypass actor on this repository, which is why no automation may push
  to `main`.

## Checker configuration

`.pudicus.yml` at the repository root defines the scanners run by the
hook. Currently one checker: `gitleaks protect --staged --config
.pudicus/gitleaks.toml`. That file is the ruleset shipped with Pudicus
0.2.0, copied in unmodified by `pudicus install`; it extends gitleaks'
default secret rules (AWS, Google/Gemini `AIza...`, PEM private keys,
generic API keys in keyword context, ...) with:

| Rule | Catches |
|---|---|
| `pudicus-personal-email` | Personal email addresses (`git@github.com`, `@users.noreply.github.com`, `@example.*`, `@github.*` allowlisted) |
| `pudicus-home-path-posix` | `/Users/<name>`, `/home/<name>` (mid-path segments like `/tmp/home/...` and `example` placeholders do not match) |
| `pudicus-home-path-windows` | `C:\Users\<name>`, `\Users\<name>` |
| `pudicus-session-uuid` | Bare UUIDs outside structural id fields (Kanbus `id`/`issue_id`/`event_id`-style fields, `BOTSPY-<uuid>` ids, and the `00000000-0000-4000-8000-...` fixture shape are allowlisted; `session_id`-style fields and free text are reported) |
| `pudicus-actor-id` | `actor_id` / `user_id` / `author_id` JSON values other than `example...` placeholders |
| `pudicus-openai-key` | OpenAI `sk-`, `sk-proj-`, `sk-svcacct-` keys |
| `pudicus-anthropic-key` | Anthropic `sk-ant-` keys |
| `pudicus-eth-private-key` | `0x`-prefixed 64-hex private keys |
| `pudicus-wif-key` | Bitcoin WIF private keys |
| `pudicus-bip39-mnemonic` | 12–24 word seed phrases in seed/mnemonic/recovery context |

Scans are **patch-scoped** (staged diffs locally, commit ranges in CI),
so content that predates this configuration is grandfathered and never
re-flagged.

Unlike the hand-written rules this replaced, `project/` (the Kanbus
board) is **not** exempt: the UUID rule allowlists only structural id
fields, and `actor_id` must be the anonymized `example-user`. Run `kbs`
with `KANBUS_USER=example-user` so board events never stamp a machine
username. To upgrade the ruleset, re-copy it from a newer Pudicus
release (`pudicus install` never overwrites an existing file). To add a
BotSpy-only rule, create a config that `[extend]`s
`.pudicus/gitleaks.toml` rather than editing the shipped file.

## TruffleHog: evaluated, not adopted

[TruffleHog](https://github.com/trufflesecurity/trufflehog) (v3.97.9)
was evaluated as a second hook checker against planted values in an
isolated test repository. It was **not** adopted:

- **No allowlist mechanism for custom detectors.** Infrastructure
  identities that must stay scannable-but-allowed under gitleaks
  (`git@github.com` URLs, `@users.noreply.github.com`, example.com
  placeholders, the `/tmp/home` test fixture) all produce findings
  under TruffleHog with no way to exempt them, so it would block
  legitimate commits.
- **Scans outside the patch.** The filesystem source reads `.git`
  objects and gitignored files and re-reports findings per source,
  breaking the grandfathering scope and flooding the hook with
  duplicate findings (43 findings on the same content gitleaks
  reports 6 for).
- **Defaults overlap gitleaks without adding coverage.** Against
  planted fake AWS credentials and a GitHub PAT, TruffleHog's default
  detectors found 0 without network verification (`--no-verification`),
  while gitleaks caught the AWS secret key.

TruffleHog remains useful as a *verified* (network, non-blocking)
secret sweeper run manually or in a scheduled advisory job, but it is
not wired into the commit hook or the PR gate.

## Failure recovery

- **Commit blocked by gitleaks:** remove the sensitive information,
  unstage it, and commit again. If (and only if) a finding is a false
  positive that a human approves, the interactive hook prompts for the
  shared-secret passphrase to record an `override:` receipt.
- **CI gate fails on an unsigned commit:** install the hook, then
  either replay the commits (rebase; the hook signs them) or run
  `pudicus approve origin/develop..HEAD` and push.
- **Secret missing locally:** run `bash scripts/setup-pudicus.sh`
  (it keeps an existing secret) or copy the shared secret to
  `~/.config/pudicus/secret` (or point `PUDICUS_SECRET_PATH` at it).