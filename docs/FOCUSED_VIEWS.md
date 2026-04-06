# Focused Views

## Goal

Give large maps calmer reading modes so users can isolate the part of the tree they are actively shaping.

This is about reducing noise without destroying context.

## Problem

Even with folding and filtering, large trees can feel visually busy:

- peers compete with the current branch
- navigation feels jumpy when many rows are visible
- users want to “zoom in” on one area without losing orientation

## UX

Add view modes inside `mdmind`:

- Full map: current default tree view
- Focus branch: show the current node, ancestors, siblings, and descendants, while dimming unrelated branches
- Subtree only: temporarily isolate the current node and its descendants
- Filtered focus: combine current filter results with a tighter local context

Small motion can help when entering or exiting these modes:

- short branch reveal transitions
- subtle focus recentering
- no animation that blocks typing or navigation

## Controls

Suggested keys:

- `v`: cycle view modes
- `V`: open a small “view mode” picker
- `Esc`: exit focused mode back to full map

## Rendering Rules

- Preserve the same focus path and expanded state when switching modes.
- Unrelated rows should dim or collapse, not vanish without explanation.
- Status and focus panels should clearly state the active view mode.

## Architecture

- Introduce a view-state enum separate from filter state.
- Rendering should consume `document + expanded state + filter state + view mode`.
- Animation, if added, should be optional and easy to disable.

## Delivery

Phase 1:

- implement view-state enum
- subtree-only and focus-branch modes

Phase 2:

- restrained transitions and recentering
- better visual separation for dimmed branches

## Risks

- too many view modes can become confusing
- motion can feel gimmicky if it delays interaction

## Product Test

When a map gets large, a user should be able to isolate the relevant branch in one keystroke and feel immediate relief from visual noise.
