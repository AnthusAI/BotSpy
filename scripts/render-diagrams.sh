#!/usr/bin/env bash
#
# Re-render every BotSpy diagram from its D2 source.
#
# One command produces, for every docs/diagrams/<name>.d2 (except the
# shared theme file):
#   <name>.svg        - one SVG with a light and a dark variant; the file
#                       switches by itself when the viewer prefers a dark
#                       color scheme
#   <name>.light.png  - PNG rendered with the light theme
#   <name>.dark.png   - PNG rendered with the dark theme
#
# The docs embed the two PNG variants with a <picture> element, so GitHub
# shows the variant that matches the reader's color scheme.
#
# Theme and styling (single source of truth):
#   - docs/diagrams/_shared.d2 holds the project-wide visual language:
#     the sensitive-data marker (a 3px amber border), the shape classes
#     (sensitive, tool, store, boundary), and the legend that explains
#     the marker. This script concatenates _shared.d2 with each diagram
#     source before rendering, so every diagram gets exactly these
#     definitions. D2 v0.9 has no working root-level import, so the
#     concatenation takes the place of an import statement.
#   - Colors come from the two pinned built-in d2 themes listed below.
#     Only the sensitive-data marker carries its own fixed color, set in
#     _shared.d2, and that color is legible on both themes.
#   - Fonts, stroke widths, border radius, and text sizes come from the
#     pinned d2 version's theme defaults. No diagram overrides them, so
#     every diagram renders with the same typeface and the same line
#     weights in both modes.
#   - Layout engine: TALA (bundled with d2). Every diagram uses TALA, so
#     spacing and alignment are consistent project-wide.
#
# Pinned tool versions:
#   - d2 CLI v0.9.0 (installed with `brew install d2`). PNG export uses
#     Playwright and downloads a headless Chromium into
#     ~/Library/Caches/ms-playwright on first use. This is the only
#     large download the pipeline needs.
#
# Usage:
#   scripts/render-diagrams.sh
#
# Override the d2 binary if it is not on PATH:
#   D2_BIN=/path/to/d2 scripts/render-diagrams.sh

set -euo pipefail

REPO_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
SRC_DIR="$REPO_ROOT/docs/diagrams"
SHARED_FILE="$SRC_DIR/_shared.d2"

D2_BIN="${D2_BIN:-d2}"
LAYOUT="tala"
LIGHT_THEME=0    # d2 built-in theme 0: "Neutral Default"
DARK_THEME=200   # d2 built-in theme 200: "Dark Mauve"
PAD=24

if ! command -v "$D2_BIN" >/dev/null 2>&1; then
  echo "error: d2 not found. Install it with 'brew install d2' or set D2_BIN." >&2
  exit 1
fi

if [[ ! -f "$SHARED_FILE" ]]; then
  echo "error: shared theme file not found: $SHARED_FILE" >&2
  exit 1
fi

shopt -s nullglob
sources=("$SRC_DIR"/*.d2)
shopt -u nullglob

if [[ ${#sources[@]} -eq 0 ]]; then
  echo "error: no diagram sources found in $SRC_DIR" >&2
  exit 1
fi

rendered=0
for src in "${sources[@]}"; do
  name="$(basename "$src" .d2)"
  # The shared theme file is concatenated into every diagram; it is not
  # a diagram of its own.
  if [[ "$name" == _* ]]; then
    continue
  fi

  merged="$(mktemp -t botspy-d2).d2"
  # Concatenate instead of import: d2 v0.9 has no working root-level
  # import, so the shared file becomes part of each diagram's source.
  # The blank line keeps the two files from merging into one line.
  cat "$SHARED_FILE" <(echo) "$src" > "$merged"

  # SVG with both themes: the file carries a light and a dark variant and
  # follows the viewer's prefers-color-scheme.
  "$D2_BIN" "$merged" "$SRC_DIR/$name.svg" \
    --layout "$LAYOUT" --theme "$LIGHT_THEME" --dark-theme "$DARK_THEME" --pad "$PAD"

  # PNG variants: one per theme, for the <picture> elements in the docs.
  "$D2_BIN" "$merged" "$SRC_DIR/$name.light.png" \
    --layout "$LAYOUT" --theme "$LIGHT_THEME" --pad "$PAD"
  "$D2_BIN" "$merged" "$SRC_DIR/$name.dark.png" \
    --layout "$LAYOUT" --theme "$DARK_THEME" --pad "$PAD"

  rm -f "$merged"
  echo "rendered $name (svg, light png, dark png)"
  rendered=$((rendered + 1))
done

echo "done: $rendered diagram(s) rendered into $SRC_DIR"