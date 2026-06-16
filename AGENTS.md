# AGENTS.md

## Project Shape

`mdmind` is a local-first thinking tool for structured maps in plain text.

The repo ships two main surfaces over the same format:

- `mdm`: CLI for inspection, validation, import/export, examples, changelog, and agent-friendly output.
- `mdmind`: keyboard-first TUI for navigating, editing, filtering, reading, and reshaping maps.

Prioritize human UX over feature accumulation. The best changes make large maps feel calmer, safer, and easier to understand.

## Product Principles

Keep the core mental model intact:

- one line is one node
- the tree is the primary structure
- plain text remains the source of truth
- tags, metadata, ids, relations, and details are lightweight additions
- view modes change projection, not the document
- palette and help reduce memory load
- local files and sidecars stay inspectable

Before adding a new surface or syntax, ask whether it fits the existing model. Prefer rendering, workflow, metadata, or help improvements before inventing new file syntax.

## UX Surfaces To Keep Aligned

When changing TUI behavior, check related surfaces:

- command palette: the universal "take me there" surface
- built-in help: searchable guidance, not just hotkeys
- view modes: `Full Map`, `Focus Branch`, `Subtree Only`, `Filtered Focus`
- minimal mode: shell density
- reading mode: long-form detail emphasis
- table view and mindmap: lenses over the current scope, not alternate sources of truth
- safety layer: undo, redo, checkpoints, autosave/manual save, restore previews

If a feature adds a shortcut, mode, status message, setting, or workflow, update help and docs in the same change when practical.

## TUI UX Guidance

The TUI should feel keyboard-first, calm, and local. Do not add permanent chrome for every new capability.

Prefer:

- contextual status over noisy always-on labels
- palette entries over new hotkeys when intent can be named
- help and search updates over hidden behavior
- stable focus, scroll, filter, and view-mode state across transitions
- clear escape paths from overlays and modes
- compact layouts that still work in small terminals

Every mode should have one plain-English job. View modes control scope, minimal mode controls chrome density, and reading mode emphasizes long-form details. Do not blur those roles.

Motion should guide attention, not decorate the interface. Prefer short, optional cues for focus changes, filter landings, scope changes, active inputs, and table selection. Motion must never block typing, shift layout unpredictably, or become required to understand state. Respect the persisted motion setting everywhere a cue or animation is introduced.

Contextual help should live in the surfaces users already trust: status messages, keybar hints, palette results, searchable help topics, and mode-specific empty states. Prefer concise, state-aware guidance over popups or permanent instructional chrome. If a user enters a mode, changes scope, or hits an empty/error state, explain the next useful action without stealing focus.

## mdm CLI Guidance

Treat `mdm` as both a human CLI and an agent-facing contract.

Prefer commands that are:

- non-interactive by default
- scriptable and deterministic
- clear in stdout/stderr behavior
- useful with deep links and filtered scopes
- friendly in human output and stable in JSON/plain output
- explicit about lossy imports, validation warnings, and unsafe assumptions

When adding or changing commands, update command help, `mdm commands` metadata if applicable, CLI tests, and related agent docs.

## Planning And Docs

Active implementation priorities live in Linear. Do not create new roadmap docs as a substitute for Linear issues.

Use repo docs by lane:

- `README.md`: user-facing overview
- `DEVELOPER.md`: setup, tests, CI, release workflow
- `docs/manual/`: user workflows
- `docs/help/`: embedded help companions
- `docs/reference/`: exact syntax and contracts
- `docs/design/`: rationale and historical product thinking
- `docs/product/shipped/`: current shipped capabilities
- `docs/agents/`: agent usage, skills, CLI contract, evals
- `CHANGELOG.md`: curated user-facing release notes

Avoid editing archived docs unless preserving history intentionally.

## Development Workflow

Use existing Rust patterns and keep changes scoped.

Before finishing code changes, run the smallest useful verification:

- `cargo fmt`
- `cargo test`
- `cargo clippy --all-targets -- -D warnings`

For CLI changes, add or update tests in `tests/cli.rs` and smoke with `cargo run --bin mdm -- ...`.

For TUI changes, add focused state/render tests where possible and smoke with `cargo run --bin mdmind -- examples/demo.md`.

For map format, parser, query, export, import, or relation changes, update fixtures/reference docs as needed and run validation-oriented tests.

## Release And Version Notes

User-facing changes should be reflected in `CHANGELOG.md` when they matter for release notes. Keep `mdm changelog`, `mdm version`, `mdmind --version`, help copy, and release docs aligned with the versioning model in `DEVELOPER.md`.

## Agent Behavior

Inspect before editing. Prefer narrow changes. Preserve existing user work. Do not rewrite broad docs, examples, or generated assets unless the task requires it.

When creating or editing mdmind maps, keep labels human-readable first. Add tags, metadata, ids, details, and relations only when they make the map more useful.
