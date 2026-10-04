#!/usr/bin/env bash
# Install the Pudicus inspection gate for this repository.
#
# Installs a pinned pudicus release into a per-user venv, writes the
# commit-msg hook into the repository's common git directory (shared by
# all linked worktrees of the clone), and creates the shared HMAC secret
# on first use. Run once per clone:
#
#   bash scripts/setup-pudicus.sh
#
# Re-run whenever the Python environment or a checker binary (gitleaks)
# moves, or force a hook reinstall with PUDICUS_REINSTALL=1. See
# docs/pudicus.md for how the hook, receipts, and CI gate fit together.
#
# Tool: https://pypi.org/project/pudicus/
set -euo pipefail

PUDICUS_VERSION="${PUDICUS_VERSION:-0.2.0}"
PUDICUS_VENV="${PUDICUS_VENV:-$HOME/.pudicus/venv}"

if ! command -v git >/dev/null 2>&1; then
  echo "error: git is required" >&2
  exit 1
fi
if ! command -v python3 >/dev/null 2>&1; then
  echo "error: python3 is required" >&2
  exit 1
fi

if [ ! -x "$PUDICUS_VENV/bin/pudicus" ]; then
  echo "Installing pudicus $PUDICUS_VERSION into $PUDICUS_VENV ..."
  python3 -m venv "$PUDICUS_VENV"
  "$PUDICUS_VENV/bin/pip" install --quiet "pudicus==$PUDICUS_VERSION"
fi

if ! command -v gitleaks >/dev/null 2>&1; then
  echo "warning: gitleaks was not found on PATH; the commit hook will fail" >&2
  echo "until you install it (e.g. brew install gitleaks)" >&2
fi

# `pudicus install` writes hooks/commit-msg into the common git dir and
# generates the shared secret at ~/.config/pudicus/secret when absent.
# It prompts before overwriting an existing hook, so only invoke it when
# the hook is missing or a reinstall is forced.
common_dir="$(git rev-parse --path-format=absolute --git-common-dir)"
hook_path="$common_dir/hooks/commit-msg"

if [ -f "$hook_path" ] && ! grep -q "pudicus" "$hook_path"; then
  echo "error: $hook_path already exists and is not a Pudicus hook;" >&2
  echo "refusing to overwrite it. Remove or rename it first." >&2
  exit 1
fi

if [ ! -f "$hook_path" ] || [ "${PUDICUS_REINSTALL:-0}" = "1" ]; then
  "$PUDICUS_VENV/bin/pudicus" install
else
  echo "Pudicus commit-msg hook already installed at $hook_path"
  echo "(set PUDICUS_REINSTALL=1 to reinstall it with the current environment)"
fi

echo "Done. Commits in this clone (including all linked worktrees) are now"
echo "scanned by gitleaks (rules in .pudicus/gitleaks.toml) and signed with a"
echo "Pudicus receipt trailer."