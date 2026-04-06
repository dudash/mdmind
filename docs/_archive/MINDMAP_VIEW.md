# Mindmap View

This document captures the design for a true visual mindmap mode layered on top of the current TUI.

## Goal

Let a user press a key inside `mdmind` and see the currently visible map as a real mindmap:

- centered around the current focus
- respecting the current expanded/collapsed branch state
- readable as bubbles and connector lines rather than only as an outline
- pannable when the layout is larger than the terminal viewport
- exportable as a PNG

This should feel like a visual “reveal” of the structure the user is already shaping in the TUI, not a separate disconnected mode.

## Core UX

Inside `mdmind`:

- Press a key such as `m` to open the mindmap overlay.
- The overlay uses the current focus as the visual center.
- Only expanded branches from the outline are rendered.
- Collapsed branches appear as compact closed bubbles with a child count.
- The user can pan with arrow keys or `hjkl`.
- `Enter` or `Esc` closes the overlay and returns to the normal TUI.
- `p` exports the current rendered mindmap view as PNG.

## Visual Model

The rendered view should not look like a generic org chart.

Preferred style:

- center bubble for the focused node
- parent chain flowing to the left or upper-left
- children fanning to the right in clustered arcs
- peers arranged near the focused node rather than flattened into a single list
- rounded bubble blocks with different colors for:
  - focused node
  - ancestors
  - peers
  - descendants
- connector lines that feel light and intentional
- tags and `@key:value` metadata shown selectively, not all at once

Important:

- text-first clarity is still more important than decorative layout
- large maps should degrade gracefully into scrollable space, not visual noise

## Data Source

The overlay should render from the same in-memory editor state already used by the outline view.

Specifically:

- current document tree
- current focus path
- current expanded/collapsed state

This is important because the overlay should be a visual rendering of the user’s current mental working set, not the entire file by default.

## Layout Rules

First implementation should use a deterministic tree layout, not a force-directed graph.

Recommended approach:

1. Build a “visible subtree” from the expanded state.
2. Treat the focused node as layout origin.
3. Layout ancestors in a compact chain to the left.
4. Layout descendants in columns or radial bands to the right.
5. Preserve sibling grouping so branches feel coherent.
6. Compute bubble bounds from wrapped text content.
7. Route connectors after node boxes are placed.

Good enough first version:

- horizontal layered layout
- branch spacing based on rendered subtree height
- viewport scrolling if bounds exceed terminal size

Later versions can add:

- animated opening
- smarter branch balancing
- radial or hybrid layouts
- relation edges for backlinks

## Viewport Behavior

The overlay needs a camera model.

Minimum behavior:

- track `offset_x` and `offset_y`
- if content fits, center it automatically
- if content exceeds viewport, allow panning
- optional shortcut to recenter on focused node

Good shortcuts:

- arrow keys: pan
- `0`: recenter
- `+` / `-`: optional future zoom controls

## PNG Export

Export should produce an image of the current rendered mindmap view, not a separate textual export.

Initial requirement:

- `p` inside the overlay exports a PNG
- output path can default to something like:
  - `map.mindmap.png`
  - or `exports/<map-name>-mindmap.png`

Rendering pipeline recommendation:

1. Convert the computed layout into a render scene.
2. Render to an offscreen canvas.
3. Save as PNG.

Strong implementation path in Rust:

- define a renderer-agnostic scene model first
- render to terminal cells for the overlay
- render the same scene to raster for PNG export

This avoids maintaining two unrelated layout systems.

## Architecture Suggestion

Add a dedicated module set instead of folding this into the main TUI file:

- `src/mindmap/layout.rs`
- `src/mindmap/scene.rs`
- `src/mindmap/tui_render.rs`
- `src/mindmap/png_render.rs`

Responsibilities:

- layout computes bubble positions and connector geometry
- scene is the normalized render model
- tui renderer paints scene into the overlay viewport
- png renderer paints the same scene into an image

## Phased Delivery

### Phase 1

- overlay opens from current TUI
- focused node plus visible descendants render as bubbles
- panning works
- closed branches show counts

### Phase 2

- ancestors and peers gain richer placement
- styling becomes more expressive
- export to PNG works

### Phase 3

- animated transitions
- zoom
- optional full-screen visual navigation mode
- relation edges and backlinks

## Risks

- trying to make the first version too graph-like will slow delivery
- rendering all metadata inside bubbles will create clutter
- separate TUI and PNG layout implementations will drift
- naive large-tree rendering can become unusable without viewport constraints

## Product Test

This feature is successful when:

- the overlay feels delightful on a medium-sized real map
- the user immediately understands branch shape and hierarchy
- the PNG export is presentation-worthy without manual cleanup
- the visual view feels like an extension of the editing workflow, not a toy demo
