# The diagrams: how to update and re-render

All diagrams are [D2](https://d2lang.com) sources in
[`diagrams/`](diagrams/). One command re-renders every diagram into
three files each: an SVG and two PNGs (light and dark).

```bash
scripts/render-diagrams.sh
```

Run it from the repository root. It writes next to the sources:

- `<name>.svg` — carries a light and a dark variant; the file follows
  the viewer's `prefers-color-scheme`.
- `<name>.light.png` — the light-mode PNG.
- `<name>.dark.png` — the dark-mode PNG.

The docs embed the two PNGs with a `<picture>` element, so GitHub
picks the variant that matches the reader's color scheme:

```html
<picture>
  <source media="(prefers-color-scheme: dark)" srcset="diagrams/<name>.dark.png">
  <img src="diagrams/<name>.light.png" alt="Description">
</picture>
```

(From `docs/sources/*.md`, prefix the paths with `../diagrams/`.)

## How to change a diagram

1. Edit `diagrams/<name>.d2`. Do not define colors, classes, or the
   legend there — the shared file already provides them (below).
2. Mark every shape that reads, copies, or stores session content
   with `class: sensitive` and put 🔒 in its label. Every diagram must
   carry the sensitive marker.
3. Run `scripts/render-diagrams.sh`.
4. Commit the source **and** the three rendered files together, so
   the checked-in renders always match the sources.
5. For a new diagram, add `diagrams/<new>.d2`, re-run the script, and
   embed the `<picture>` block in the relevant document. Names
   starting with `_` are skipped by the script.

## The theme and styling files

Two files define the entire visual language. Nothing else sets colors
or styling.

### `diagrams/_shared.d2` — the shared theme

This file is the single source of truth. `scripts/render-diagrams.sh`
concatenates it in front of every diagram source before rendering, so
every diagram gets exactly these definitions (D2 v0.9 has no working
root-level import, so the script concatenates instead — do not try
`#: import` or `@import`):

- `vars.sensitive-color` — the one marker color, `#d97706` (amber
  600). It is legible on both the light and the dark background. The
  marker is a 3px stroke; the shape fill still comes from the theme,
  so it adapts to both modes.
- The shape classes:
  - `sensitive` — the 3px amber border. Apply with
    `class: sensitive`.
  - `tool` — the default look for tools and containers that only move
    data; a 7px border radius.
  - `store` — `shape: cylinder`, for BotSpy's own storage shapes.
  - `boundary` — a dashed border, for trust zones and file-system
    areas.
- The **legend** — a small group that explains the sensitive marker
  and the default look. It lives in the shared file, so it renders on
  every diagram automatically. Do not add a legend to an individual
  diagram.

### `scripts/render-diagrams.sh` — the pipeline settings

The script pins everything that must stay consistent project-wide:

| Setting | Value |
| --- | --- |
| d2 version | v0.9.0 (`brew install d2`) |
| Layout engine | `--layout tala` (bundled with d2), on every diagram |
| Light theme | d2 built-in theme 0, "Neutral Default" (`--theme 0`) |
| Dark theme | d2 built-in theme 200, "Dark Mauve" (`--dark-theme 200` for the SVG; `--theme 200` for the dark PNG) |
| Fonts, stroke widths, border radius, text sizes | The pinned d2 version's theme defaults — no diagram overrides them, so every diagram renders with the same typeface and line weights in both modes |

The sensitive-data marker is the only fixed color; everything else
adapts to the two themes. That is why the amber stroke stays legible
in both modes while fills change.

## The PNG export and Playwright

D2 renders PNGs by screenshotting with a headless Chromium. On the
first PNG render, d2 downloads that Chromium through Playwright into
`~/Library/Caches/ms-playwright`. This is the one large download the
pipeline needs; the repository needs no other large tooling.

## The diagrams in one list

| File | Shows |
| --- | --- |
| `context.d2` | C4 level 1: the user, BotSpy, and the agents on the machine |
| `containers.d2` | C4 level 2: the CLI, the files, the snapshots, the store, the model cache |
| `components.d2` | C4 level 3: adapters, importer, schema, store components |
| `dataflow.d2` | The top-level data flow with every source and every exact path |
| `claude-code.d2`, `cursor.d2`, `codex.d2`, `grok-bot.d2`, `antigravity.d2` | One per source adapter |
| `store.d2` | The store: open, ingest, refresh, query, tables, WAL sidecars |
| `search.d2` | Text, semantic, and hybrid search |
| `model-assets.d2` | The one-time model download and the cache |