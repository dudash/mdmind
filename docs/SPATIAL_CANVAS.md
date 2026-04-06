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

## Navigation

- arrows or `hjkl`: move camera
- `Tab`: cycle visible nearby nodes
- `Enter`: focus the highlighted bubble
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
- separate spatial navigation from outline navigation logic

## Delivery

Phase 1:

- visual navigation only
- deterministic layout
- camera pan and recenter

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
