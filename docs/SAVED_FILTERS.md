# Saved Filters And Views

## Goal

Turn recurring filter workflows into named, reusable views.

## Problem

Once a map gets large, users naturally revisit the same working sets:

- `#todo`
- `@status:blocked`
- `#prompt @owner:jason`

Retyping them is friction, and remembering the exact syntax breaks flow.

## UX

Saved views should feel lightweight.

Examples:

- save the current filter as `blocked`
- reopen `blocked` from a palette or dedicated shortcut
- mark one view as the default opening lens for a file if desired

Potential TUI actions:

- `save current filter as view`
- `open saved view`
- `rename view`
- `delete view`

## Scope

Saved views should store:

- raw filter query
- optional sort or grouping preference later
- optional “scope to subtree” behavior later

They should not mutate the map itself.

## Storage

Store views in a sidecar session-like file first, not inline in the document.

That keeps the map plain-text format stable while the feature matures.

Current implementation:

- one JSON sidecar per map
- file name pattern: `.<map-file-name>.mdmind-views.json`
- stored next to the source map

## Delivery

Phase 1:

- save and reopen named filters
- per-map storage

Phase 2:

- recents and favorites
- saved grouped views

## Risks

- users may expect shared, portable views if they are not stored in the document
- too many saved views can become their own clutter problem

## Product Test

A user who repeatedly works the same slices of a large map should feel that the tool remembers their workflow.
