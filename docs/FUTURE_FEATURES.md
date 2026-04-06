# Future Features

This file is the roadmap index for features that matter after the current CLI and TUI baseline.

Each item below has its own design note so future work can start from a concrete product brief instead of a loose idea.

## Navigation And Large-Map UX

- Deep-link path fallback: let `file.md#parent/child` resolve by labels when no explicit id exists. See `docs/DEEP_LINK_FALLBACK.md`.
- Focused TUI views and motion: add tighter navigation modes, branch isolation, and restrained transitions so large maps feel easier to parse. See `docs/FOCUSED_VIEWS.md`.
- Command palette and fuzzy search: make branch jumping, action discovery, and large-map navigation feel instant. See `docs/COMMAND_PALETTE.md`.
- Saved filters and recurring views: preserve working sets like `#todo`, `@status:blocked`, or `@owner:jason` as first-class views. See `docs/SAVED_FILTERS.md`.

## Visual Mind-Mapping

- Mindmap overlay: render the current expanded working set as a visual bubble map with pan and PNG export. See `docs/MINDMAP_VIEW.md`.
- Spatial canvas mode: grow the overlay into a more ambitious visual navigation surface with stronger clustering and layout freedom. See `docs/SPATIAL_CANVAS.md`.
- Relations and backlinks: show non-tree connections between nodes and let users navigate them directly. See `docs/RELATIONS_AND_BACKLINKS.md`.

## Templates, Packaging, And Export

- Template variables: allow `mdm init` templates to prompt for values such as owner, project name, and default status. See `docs/TEMPLATE_VARIABLES.md`.
- CLI completions and manual pages: generate shell completions and man pages from the command definition so the tool is easier to adopt and package. See `docs/CLI_HELP_AND_COMPLETIONS.md`.
- Richer export targets: add Mermaid, OPML, and other structure-preserving output formats beyond JSON. See `docs/EXPORT_TARGETS.md`.
