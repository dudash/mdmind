# Mindspace Boundaries

Mindspace is optional. It should not make the base mdmind experience heavier.

## Product Boundary

Single-file maps remain first-class:

- `mdmind file.md` opens a map without requiring a mindspace.
- `mdm view`, `find`, `links`, `relations`, `validate`, and `export` keep their
  single-file behavior.
- Map syntax remains documented in `docs/reference/`, not in Mindspace docs.
- Users can delete `.mdmind/` and still keep useful maps and Markdown files.

Mindspace adds folder-level coordination only when it earns its keep:

- workspace inventory
- path-qualified cross-file references
- deterministic scan and lint
- context bundles with provenance
- scoped agent sessions and review items
- checkpoints across related workspace changes

## CLI Boundary

Mindspace commands live under `mdm mindspace ...`.

Do not add top-level commands for workspace-only behavior unless the same
behavior is useful for a single map. Keep existing single-file commands calm and
unchanged.

Good:

```bash
mdm mindspace scan .
mdm mindspace setup . --preview
mdm mindspace context maps/tasks.md#todo/focus
```

Avoid:

```bash
mdm scan .
mdm session ... for Mindspace sessions
mdm init --workspace ...
```

`mdm init <path>` remains single-map creation. Future native workspace creation
belongs under `mdm mindspace new <path>`.

## TUI Boundary

`mdmind .` may open a workspace surface, but `mdmind file.md` should still feel
like the core editor.

Mindspace TUI work should add:

- workspace landing
- map and id switching
- recent map stack
- pinned working set
- cross-map backlinks when scan data exists
- review surfaces for proposed workspace changes

Mindspace TUI work should not add permanent chrome to the single-map editor or
turn the app into a general file manager. See [NAVIGATION.md](NAVIGATION.md)
for the multi-file navigation model.

## Docs Boundary

Use this section for Mindspace docs by default:

- workspace reference
- manifest schema
- command contracts
- safety model
- agent session and review design
- workflow tips
- future Mindspace-specific examples

Use core docs only for behavior that applies without Mindspace:

- map syntax
- single-map query behavior
- same-file ids and relations
- normal TUI editing
- normal CLI inspection and export

When in doubt, link from core docs to Mindspace instead of copying Mindspace
concepts into core docs.
