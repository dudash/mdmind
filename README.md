# mdmind

`mdmind` is a local-first mind mapping tool for structured thinking in plain text.

It gives you two interfaces over the same map format:

- `mdm`: a CLI for viewing, searching, validating, and exporting maps
- `mdmind`: a full-screen TUI for navigating, filtering, editing, and reshaping maps

The goal is not to mimic a whiteboard app. The goal is to make large idea trees feel fast, searchable, and safe to edit with a keyboard.

License: Apache-2.0

## What A Map Looks Like

Maps are plain-text tree files with lightweight inline structure:

- `#tag` for grouping and workflow markers
- `@key:value` for structured metadata
- `[id:path/to/node]` for stable deep links

Example:

```text
- Product Idea #idea [id:product]
  - Direction #strategy [id:product/direction]
    - CLI-first MVP
  - Tasks #todo @status:active [id:product/tasks]
    - Build parser
    - Ship tests
```

That gives you:

- human-readable files
- deep links like `ideas.md#product/tasks`
- filterable tags and metadata
- editable maps that still work well in git

## What mdmind Is Good For

- product and feature planning
- project breakdowns
- prompt libraries
- backlog shaping
- idea exploration
- keyboard-first personal planning

It is intentionally not a rich document editor, team wiki, or freeform diagramming canvas.

## Install

For local development from this repo:

```bash
cargo run --bin mdm -- version
cargo run --bin mdm -- --help
cargo run --bin mdmind -- examples/demo.md
```

To install the binaries locally:

```bash
cargo install --path .
```

That installs:

- `mdm`
- `mdmind`

## CLI Quick Start

Create a new map from a starter template:

```bash
mdm init roadmap.md --template product
```

View a map:

```bash
mdm view roadmap.md
mdm view roadmap.md#product/tasks
```

Search by text, tag, or metadata:

```bash
mdm find roadmap.md "rate limit"
mdm find roadmap.md "#todo"
mdm find roadmap.md "@status:blocked"
mdm find roadmap.md "#todo @owner:jason"
```

Inspect structure:

```bash
mdm tags roadmap.md
mdm kv roadmap.md --keys status,owner
mdm links roadmap.md
mdm validate roadmap.md
mdm export roadmap.md --format json
mdm export roadmap.md --format mermaid
mdm export roadmap.md#product/tasks --format opml
```

Open the interactive TUI:

```bash
mdm open roadmap.md
mdm open roadmap.md#product/tasks --autosave
```

## TUI Quick Start

Run:

```bash
mdmind roadmap.md
```

Core navigation:

- `↑` / `↓`: move through visible nodes
- `←` / `→`: collapse/expand or move between parent and child
- `Enter`: toggle branch expansion
- `g`: jump to root

Editing:

- `a`: add child
- `A`: add sibling
- `Shift+R`: add root
- `e`: edit selected node
- `x`: delete selected node, confirmed on second press
- `u` / `U`: undo or redo the last structural change

Reshaping:

- `Alt+↑` / `Alt+↓`: move node among siblings
- `Alt+←`: move node out one level
- `Alt+→`: indent node into previous sibling

Search and large-map workflows:

- `:` / `Ctrl+P`: open the command palette
- inside the palette, type `undo` or `redo` to browse recent actions, `checkpoint` to find manual checkpoints, `safety` to find automatic safety snapshots, `theme` to preview themes, `ascii` to toggle terminal-style accents, or `motion` to control attention-guiding focus, filter, and input motion
- `?`: open searchable built-in help
- `/`: open unified search
- `f`: open unified search on facets
- `F`: open unified search on saved views
- `v` / `V`: cycle focused view modes forward or backward
- `g`: jump to the map root, or back to the subtree root while `Subtree Only` is active
- `m`: open the visual mindmap overlay
- `↑` / `↓` / `←` / `→` inside the overlay: pan the camera
- `0` inside the overlay: recenter on the focused node
- `p` inside the overlay: export the current rendered view to `map-name.mindmap.png`
- `Tab`: switch between `Query`, `Facets`, and `Saved Views`
- `←` / `→` inside facets: switch `Tags`, `Keys`, `Values`
- `n` / `N`: move to next or previous match
- `c`: clear active filter

Saving:

- `s`: save now
- `S`: toggle autosave
- `r`: reload from disk and discard unsaved in-memory changes
- `q`: quit
- `?`: help

## Local-First Behavior

`mdmind` writes your map back to the original file. It also keeps small hidden sidecar files next to the map:

- session restore: `.<map-file>.mdmind-session.json`
- saved views: `.<map-file>.mdmind-views.json`
- checkpoints: `.<map-file>.mdmind-checkpoints.json`
- UI settings: `.<map-file>.mdmind-ui.json`

These keep editor state local without changing the map format itself. The UI settings sidecar stores the active theme plus per-map surface preferences such as attention-guiding motion and ASCII accents.

## Project Templates

Starter templates live in `templates/` and include:

- `product`
- `feature`
- `prompts`
- `backlog`

Example:

```bash
mdm init prompts.md --template prompts
```

## Repo Docs

User-facing and product docs:

- [docs/FUTURE_FEATURES.md](docs/FUTURE_FEATURES.md)
- [docs/TERMINAL_EXPERIENCE.md](docs/TERMINAL_EXPERIENCE.md)
- [docs/DOCUMENTATION_STRATEGY.md](docs/DOCUMENTATION_STRATEGY.md)
- [docs/SPATIAL_CANVAS.md](docs/SPATIAL_CANVAS.md)
- [docs/COMMAND_PALETTE.md](docs/COMMAND_PALETTE.md)

Developer workflow, testing, CI, and release notes:

- [DEVELOPER.md](DEVELOPER.md)

## Current Status

The app is already useful for:

- authoring and editing structured map files
- deep-linking into a map by node id
- filtering large maps by text, tags, and metadata
- calming large maps with full-map, focus-branch, subtree-only, and filtered-focus views
- exporting full maps or deep-linked subtrees as JSON, Mermaid, and OPML
- saving and reopening named filtered views
- opening a visual bubble-style mindmap overlay from the current working set
- exporting that visual mindmap view as a PNG
- keyboard-first restructuring of nodes in the tree

The next major UX leap is a stronger command palette and then a more ambitious spatial canvas beyond the current overlay.
