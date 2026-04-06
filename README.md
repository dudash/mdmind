# mdmind

A terminal-native thinking tool for fast idea exploration, structured planning, and lightweight project design.

`mdm` is the CLI.
`mdmind` is the interactive navigator and editing flow on the path to the full TUI.

## Purpose

`mdmind` is for:
- idea exploration
- TODO and requirement breakdowns
- prompt refinement
- project templates
- keyboard-first thinking

It is not trying to replace a full document editor, whiteboard, or team collaboration platform.

## Core product shape

- Plain-text markdown-like format
- Human-readable tree structure
- Inline structure with:
  - `#tags`
  - `@key:value`
  - `[id:path/to/node]`
- Deep links like `ideas.md#product/api-design`
- Local-first workflow
- Composable CLI output

## Repo docs

- `VISION.md`
- `DESIGN.md`
- `MVP.md`
- `FORMAT.md`
- `DECISIONS.md`
- `ROADMAP.md`
- `CODEX_KICKOFF.md`

## Status

Current capabilities:
- Parse, validate, search, and export maps from the CLI
- Create starter maps from templates
- Open a deep link in a full-screen interactive TUI
- Restore the last focused node with a local hidden session file
- Navigate with arrow keys through a colorized map outline and focus panes
- Add or edit nodes from in-app modal prompts without leaving the map

From the repo root, run:

```bash
cargo run --bin mdm -- --help
cargo run --bin mdm -- view examples/demo.md
cargo run --bin mdm -- init my-map.md --template product
cargo run --bin mdm -- open my-map.md
cargo run --bin mdmind -- examples/demo.md#demo/direction
```

Inside `mdmind`, use commands like:
- Arrow keys to move and expand branches
- `a` to add a child, `A` to add a sibling, `Shift+R` to add a root
- `e` to edit the selected node
- `/` to jump to a node id
- `s` to save, `?` for the keymap, `q` to quit

To install binaries instead of using `cargo run`:

```bash
cargo install --path .
```
