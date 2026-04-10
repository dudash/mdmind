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
- `[[target/id]]` or `[[rel:kind->target/id]]` for cross-branch references

Compatibility notes:

- these files stay valid, readable plain text in normal Markdown tools
- `[id:...]` and `[[...]]` are mdmind conventions, so ordinary Markdown renderers usually show them as literal inline text
- tools that already support wiki-link syntax may also interpret `[[target]]` as a link, which is usually compatible with the intent
- the simplest mental model is:
  - `[id:...]` gives a node a stable address
  - `[[target]]` points at that address
  - `[[rel:kind->target]]` adds meaning to that cross-link

Example:

```text
- Product Idea #idea [id:product]
  - Direction #strategy [id:product/direction]
    - CLI-first MVP
  - Tasks #todo @status:active [id:product/tasks] [[prompts/library]]
    - Build parser
    - Ship tests
  - Prompt Library #prompt [id:prompts/library]
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
cargo run --bin mdmind -- examples/product-status.md
cargo run --bin mdmind -- examples/lantern-studio-map.md
cargo run --bin mdmind -- examples/game-world-moonwake.md
cargo run --bin mdmind -- examples/novel-research-writing-map.md
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
mdm relations roadmap.md
mdm validate roadmap.md
mdm export roadmap.md --format json
mdm export roadmap.md --format mermaid
mdm export roadmap.md#product/tasks --format opml
```

Inspect the bundled example maps from the CLI:

```bash
mdm find examples/lantern-studio-map.md "@owner:mira" --plain
mdm kv examples/game-world-moonwake.md --keys owner,region --plain
mdm tags examples/novel-research-writing-map.md --plain
mdm links examples/lantern-studio-map.md --plain
```

These are useful for learning the map language in read-only mode:

- `find` shows matching labels, ids, tags, and metadata in context
- `kv` is good for auditing shared fields like `owner`, `status`, or `region`
- `tags` gives you the vocabulary and shape of a map quickly
- `links` lists stable ids you can deep-link to from `view`, `open`, or `export`
- `relations` shows outgoing references across the map, or outgoing plus incoming backlinks for a deep-linked node

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
- add/edit prompts now preview parsed `#tags`, `@key:value`, `[id:...]`, and duplicate-id issues before `Enter`
- `x`: delete selected node, confirmed on second press
- `u` / `U`: undo or redo the last structural change

Reshaping:

- `Alt+↑` / `Alt+↓`: move node among siblings
- `Alt+←`: move node out one level
- `Alt+→`: indent node into previous sibling

Search and large-map workflows:

- `:` / `Ctrl+P`: open the command palette
- inside the palette, type `tasks` or another branch name to jump to nodes, `#todo` or `@status:active` to apply inline filters, `product/tasks` or `[id:product/tasks]` to jump by id, relation kinds or target ids like `supports`, `prompts/library`, or `backlink` to jump across cross-links, `review todo` or `work inside branch` to run built-in workflows, `owner` or `status` to surface contextual review recipes when those fields exist in the current map, revisit recent locations and frequent places, type `undo` or `redo` to browse recent actions, `checkpoint` to find manual checkpoints, `safety` to find automatic safety snapshots, `theme` to preview themes like `violet`, `monograph`, or `paper`, `ascii` to toggle terminal-style accents, or `motion` to control attention-guiding focus, filter, and input motion
- `]`: follow the next outgoing relation on the focused node
- `[`: follow the next backlink into the focused node
- type `minimal` in the palette to switch to a quieter pro layout with a condensed shell, no keybar, a wider main tree, and fewer right-side context lanes
- `?`: open searchable built-in help with user guides, command reference, and tips
- `/`: open unified search
- `f`: open unified search on facets
- `F`: open unified search on saved views
- `v` / `V`: cycle focused view modes forward or backward
- `g`: jump to the map root, or back to the subtree root while `Subtree Only` is active
- `m`: open the visual mindmap overlay
- `↑` / `↓` / `←` / `→` inside the overlay: pan the camera
- `0` inside the overlay: recenter on the focused node
- `p` inside the overlay: export the current rendered view to `map-name.mindmap.png`
- when both endpoints are visible, the mindmap also draws cross-link relation edges between related branches
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
- navigation memory: `.<map-file>.mdmind-locations.json`
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

- [docs/USER_GUIDE.md](docs/USER_GUIDE.md)
- [docs/FUTURE_FEATURES.md](docs/FUTURE_FEATURES.md)
- [docs/TERMINAL_EXPERIENCE.md](docs/TERMINAL_EXPERIENCE.md)
- [docs/DOCUMENTATION_STRATEGY.md](docs/DOCUMENTATION_STRATEGY.md)
- [docs/SPATIAL_CANVAS.md](docs/SPATIAL_CANVAS.md)
- [docs/COMMAND_PALETTE.md](docs/COMMAND_PALETTE.md)

Example maps you can open directly in `mdmind`:

- [examples/README.md](examples/README.md)
- [examples/demo.md](examples/demo.md)
- [examples/product-status.md](examples/product-status.md)
- [examples/lantern-studio-map.md](examples/lantern-studio-map.md)
- [examples/game-world-moonwake.md](examples/game-world-moonwake.md)
- [examples/novel-research-writing-map.md](examples/novel-research-writing-map.md)
- [examples/team-project-board.md](examples/team-project-board.md)
- [examples/prompt-ops.md](examples/prompt-ops.md)
- [examples/decision-log.md](examples/decision-log.md)

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
