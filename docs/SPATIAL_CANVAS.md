# Spatial Canvas Mode

## Goal

Evolve `mdmind` beyond an outline-with-panels into a more spatial visual exploration mode for medium and large maps.

This is the ambitious “mind map” direction after the simpler overlay is proven.

## Relationship To `MINDMAP_VIEW.md`

The overlay in `docs/MINDMAP_VIEW.md` is the pragmatic first step.

Spatial canvas mode is the follow-on product:

- richer layout freedom
- stronger clustering
- more direct visual navigation
- a bigger emphasis on delight

## UX

The user enters a full-screen canvas mode where:

- the focused node is centered
- child branches fan outward in clusters
- peers orbit nearby instead of reading like a flat list
- ancestors form a readable chain or spine
- the camera can pan and possibly zoom

This mode should feel like a working surface, not just a screenshot of the tree.

Current Phase 1 behavior:

- `m` opens the full-screen spatial canvas from the TUI
- `M` opens the older fixed visual mindmap overlay when needed
- the same scene model as the visual mindmap is projected into a spatial layout
- the focused node is centered
- ancestors sit left of the focus
- descendants fan right from the focus
- peers cluster near the focus
- siblings stay in document order; focus highlighting moves instead of reordering the map
- expanded branches stay visible until the user explicitly collapses them, matching Full Map behavior
- the camera fits the focused neighborhood when possible, then centers on the active focus when it cannot fit
- connectors to off-screen bubbles are hidden rather than drawn as dangling lines
- the selected bubble is highlighted independently when using Tab-based jumps
- normal arrows / `hjkl` navigate and expand/collapse like the outline
- `Shift` + arrows pan the camera
- `+` / `=` zoom in and `-` / `_` zoom out
- `Tab` / `Shift+Tab` optionally cycle visible bubbles
- `Enter` toggles the current branch, or focuses a Tab-selected bubble

## Navigation

- arrows or `hjkl`: navigate focus, expand, and collapse like the normal outline
- `Shift` + arrows: move camera
- `+` / `=` and `-` / `_`: zoom in and out
- `Tab`: optionally cycle visible nearby nodes for non-linear jumps
- `Enter`: toggle the focused branch, or focus a Tab-selected bubble
- `0`: recenter on the focused node
- `Esc`: return to outline mode

## Editing

Editing in canvas mode can start narrow:

- create child
- create sibling
- rename focused node
- delete focused node with confirmation

Heavy structural editing can still route back to the outline until the canvas interaction proves itself.

## Layout Direction

Prefer a deterministic hybrid layout:

- focused node at center
- descendants fanned right or around the center
- ancestors on the left or upper-left
- peers clustered near the focus

Do not start with force-directed physics.

## Architecture

- share one scene model with the mindmap overlay
- add a camera layer for pan and optional zoom
- reuse the outline navigation logic for focus, expand, and collapse

## Delivery

Phase 1:

- visual navigation only - shipped
- deterministic layout - shipped
- camera pan and recenter - shipped

Phase 2:

- lightweight editing
- better clustering and collision handling

Phase 3:

- animations
- relation edges
- polished export

## Risks

- trying to replace the outline too early
- visual density becoming unreadable on large trees

## Product Test

The mode succeeds when a user can understand the structure of a medium-sized map faster in canvas mode than in the plain outline, without feeling lost.
