#!/usr/bin/env bash
# Pudicus receipt gate for pull requests.
#
# Fails unless every commit that is new to the base branch carries a
# valid Pudicus receipt (HMAC trailer), and re-runs the gitleaks scan
# over the PR's commits as defense in depth.
#
# Scope: only commits in merge-base(origin/<base>, HEAD)..HEAD are
# verified. History that predates the gate is never re-verified.
#
# Exemptions (see docs/pudicus.md):
#   - merge commits: created by the GitHub merge button, where local
#     hooks cannot run;
#   - commits authored by release bots (github-actions[bot],
#     release-plz[bot]): created inside CI, where local hooks do not
#     run; the release workflow's own push carries the exemption.
#
# Usage: scripts/pudicus_gate.sh <base-ref>   (e.g. develop)
set -euo pipefail

base_ref="${1:?usage: pudicus_gate.sh <base-ref>}"
base_sha="$(git merge-base "origin/$base_ref" HEAD)"

echo "Pudicus receipt gate: verifying commits new to origin/$base_ref (merge base $base_sha)"

exempt_shas=" "
verify_needed=0
for sha in $(git rev-list "$base_sha..HEAD"); do
  parents="$(git rev-list --parents -n 1 "$sha" | awk '{print NF-1}')"
  author="$(git log -1 --format=%an "$sha")"
  if [ "$parents" -gt 1 ]; then
    echo "EXEMPT (merge commit): $(git log -1 --format='%h %s' "$sha")"
    exempt_shas="$exempt_shas${sha:0:7} "
  elif [ "$author" = "github-actions[bot]" ] || [ "$author" = "release-plz[bot]" ]; then
    echo "EXEMPT (release bot $author): $(git log -1 --format='%h %s' "$sha")"
    exempt_shas="$exempt_shas${sha:0:7} "
  else
    verify_needed=$((verify_needed + 1))
  fi
done

if [ "$verify_needed" -gt 0 ]; then
  # `pudicus verify` pools valid receipts from the whole range and then
  # flags every commit whose tree has no pooled signature. Failures for
  # exempt commits are expected and filtered out below.
  set +e
  verify_output="$(pudicus verify "$base_sha..HEAD" 2>&1)"
  verify_status=$?
  set -e
  echo "$verify_output"

  if [ "$verify_status" -ne 0 ]; then
    gate_failed=0
    while IFS= read -r short; do
      exempt=0
      case " $exempt_shas " in *" $short "*) exempt=1 ;; esac
      if [ "$exempt" -eq 0 ]; then
        gate_failed=1
        echo "::error::Commit ${short} has no valid Pudicus receipt. Commit with the hook installed by scripts/setup-pudicus.sh, or add retroactive receipts with 'pudicus approve origin/${base_ref}..HEAD' followed by a push."
      fi
    done < <(printf '%s\n' "$verify_output" | sed -n 's/.*\([0-9a-f]\{7\}\): No valid signature found.*/\1/p')
    if [ "$gate_failed" -ne 0 ]; then
      echo "Pudicus receipt gate FAILED: unsigned commit(s) found." >&2
      exit 1
    fi
    echo "All receipt failures belong to exempt (merge/bot) commits."
  fi
else
  echo "All new commits are exempt (merge/bot); nothing to receipt-verify."
fi

# Defense in depth: scan the PR's commits for sensitive information,
# independent of any receipts. Same rule set as the hook.
gitleaks detect --log-opts="$base_sha..HEAD" --config .gitleaks.toml --no-banner --redact

echo "Pudicus receipt gate passed."