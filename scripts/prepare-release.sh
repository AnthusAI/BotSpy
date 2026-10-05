#!/usr/bin/env bash
# Prepare a release: bump the version and changelog on develop.
#
# Runs `release-plz update` (next version from conventional commits;
# edits Cargo.toml, Cargo.lock, CHANGELOG.md) and commits the result
# locally as "chore: release botspy v<version>". The Pudicus commit-msg
# hook scans and signs that commit like any other. This script never
# pushes. Run it on an up-to-date develop (or a feature branch cut from
# it) before promoting develop to main:
#
#   bash scripts/prepare-release.sh
#
# The release workflow on main only tags, publishes, and creates the
# GitHub release; it never commits or pushes. See docs/pudicus.md for
# why no automation may push to main.
#
# Tool: https://release-plz.dev/docs/install
set -euo pipefail

if ! command -v git >/dev/null 2>&1; then
  echo "error: git is required" >&2
  exit 1
fi

if ! command -v release-plz >/dev/null 2>&1; then
  cat >&2 <<'MSG'
error: release-plz is not installed.

Install it with one of:
  cargo binstall release-plz   # prebuilt binary (needs cargo-binstall)
  cargo install release-plz --locked
  brew install release-plz

See https://release-plz.dev/docs/install
MSG
  exit 1
fi

if [ -n "$(git status --porcelain)" ]; then
  echo "error: working tree is not clean; commit or stash changes first" >&2
  exit 1
fi

branch="$(git rev-parse --abbrev-ref HEAD)"
if [ "$branch" = "main" ]; then
  echo "error: run this on develop or a feature branch, never on main" >&2
  exit 1
fi

echo "Fetching origin..."
git fetch origin develop
if [ "$branch" = "develop" ]; then
  if [ "$(git rev-parse HEAD)" != "$(git rev-parse origin/develop)" ]; then
    echo "error: develop is not up to date with origin/develop; pull or push first" >&2
    exit 1
  fi
else
  if ! git merge-base --is-ancestor origin/develop HEAD; then
    echo "error: $branch does not contain the latest origin/develop; rebase or merge it first" >&2
    exit 1
  fi
fi

# release-plz checks out the branch named by the current branch's upstream
# (e.g. origin/develop) in a temporary worktree. On a feature branch that
# tracks origin/develop (the default for `git worktree add -b <branch>
# origin/develop`) that fails with "'develop' is already checked out"
# whenever develop is checked out elsewhere. Drop the tracking config on
# feature branches; `git push -u` sets the right upstream later.
if [ "$branch" != "develop" ] && git rev-parse --abbrev-ref "@{upstream}" >/dev/null 2>&1; then
  git branch --unset-upstream
fi

release-plz update

if [ -z "$(git status --porcelain)" ]; then
  echo "Nothing to release: release-plz found no version bump."
  exit 0
fi

version="$(cargo metadata --no-deps --format-version 1 \
  | python3 -c 'import json,sys; print(next(p["version"] for p in json.load(sys.stdin)["packages"] if p["name"] == "botspy"))')"

git add -A
git commit -m "chore: release botspy v${version}"

cat <<MSG

Committed "chore: release botspy v${version}" locally. Nothing was pushed.

Next steps:
  1. Push this commit (or a PR branch containing it) and get it into
     develop. On a feature branch, open a pull request targeting develop.
  2. Open the develop -> main promotion pull request.
  3. Merging that PR triggers the release workflow on main, which tags,
     publishes to crates.io, and creates the GitHub release.
MSG
