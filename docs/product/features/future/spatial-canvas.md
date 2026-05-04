# Spatial Canvas

The current visual mindmap overlay is shipped, and the first spatial-canvas navigation slice now exists.

What exists now:

- full-screen spatial canvas from the TUI with `m`
- legacy fixed visual mindmap remains available with `M`
- deterministic focus-centered layout
- focused-neighborhood camera framing with focus-centered fallback
- off-screen connector clipping to avoid dangling lines over the canvas edge
- Full Map-like expanded branch visibility: branches stay visible until explicitly collapsed
- stable sibling order while focus highlighting moves through the map
- ancestors to the left, descendants fanned right, peers near the focus
- outline-like arrow / `hjkl` navigation and expand/collapse
- `Shift` + arrows camera pan
- `+` / `=` zoom in and `-` / `_` zoom out
- `0` recenter
- optional `Tab` / `Shift+Tab` bubble selection for non-linear jumps

The remaining future canvas work is the larger next step:

- stronger clustering
- more ambitious layout behavior
- richer editing inside the canvas
- export and relation-edge polish for the spatial layout

This is no longer purely future product behavior, but the shipped slice is intentionally navigation-first.

Related docs:

- [SPATIAL_CANVAS.md](../../../SPATIAL_CANVAS.md)
