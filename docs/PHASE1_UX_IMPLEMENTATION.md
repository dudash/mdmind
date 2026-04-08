# Phase 1 UX Implementation

## Goal

Turn the next `mdmind` UX wave into an implementation sequence that fits the current codebase.

This plan covers the near-term work for:

- command palette
- searchable built-in help
- focused views
- status line refresh
- theme system as the follow-on slice immediately after the first UX landing

## Current Code Shape

Today, the TUI is concentrated in `src/interactive.rs`.

Important existing seams:

- `TuiApp` already owns TUI state and input handling
- overlays already exist for prompt, search, help, and mindmap
- `Editor` already owns structural edits and focus state
- `FilterQuery` already gives us a shared query language
- session and saved views already persist to local sidecar files

This is good enough to extend, but not good enough to keep piling features into one file. Phase 1 should extract pure state and rendering helpers before adding more overlays.

## Constraints

- keep the current TUI behavior working while refactoring
- avoid a full rewrite of `interactive.rs`
- keep new logic testable without a live terminal
- preserve local-first behavior and sidecar persistence

## Delivery Shape

## Slice A

Ship the biggest usability win first:

- command palette
- searchable help
- focused views
- richer status model

Current status:

- focused views are implemented
- command palette is implemented
- searchable built-in help is implemented
- richer status modeling is implemented
- theme system is implemented
- per-map surface settings are implemented with live palette preview, restrained motion toggles, and ASCII accents

## Slice B

Once Slice A is stable:

- theme system
- per-map UI settings persistence
- small surface polish tied to themes

This split keeps the first release centered on navigation and discoverability, while the second pass handles the visual identity layer.

## Proposed Module Layout

Introduce a new `src/tui/` module tree and migrate code into it incrementally.

Recommended layout:

- `src/tui/mod.rs`
- `src/tui/app.rs`
- `src/tui/input.rs`
- `src/tui/render.rs`
- `src/tui/status.rs`
- `src/tui/view_mode.rs`
- `src/tui/palette.rs`
- `src/tui/help.rs`
- `src/tui/theme.rs`
- `src/tui/settings.rs`

Responsibility split:

- `app.rs`: `TuiApp`, overlay enum, orchestration, app-level mutations
- `input.rs`: key routing and mode-specific handlers
- `render.rs`: frame composition and overlay rendering
- `status.rs`: status line model and hint generation
- `view_mode.rs`: visible-row projection rules for full map vs focused views
- `palette.rs`: palette items, scoring, recents, and execution targets
- `help.rs`: help topics, searchable help index, and help result rendering
- `theme.rs`: theme ids, color surfaces, style helpers
- `settings.rs`: read and write per-map UI settings

Do not move everything at once. Start by creating these modules and moving pure data structures and render helpers first.

## State Model

## Core App State

Replace ad hoc booleans with more explicit mode state.

Recommended additions:

```rust
enum Overlay {
    Prompt(PromptState),
    Search(SearchOverlayState),
    Mindmap(MindmapOverlayState),
    Palette(CommandPaletteState),
    Help(HelpOverlayState),
}

enum ViewMode {
    FullMap,
    FocusBranch,
    SubtreeOnly,
    FilteredFocus,
}

struct UiState {
    overlay: Option<Overlay>,
    view_mode: ViewMode,
    status: StatusModel,
    settings: UiSettings,
}
```

This does two things:

- it prevents overlay combinations from becoming accidental state bugs
- it gives focused views and themes a real home instead of more booleans on `TuiApp`

## Status Model

The current status message is one line of free text. Phase 1 needs a structured status surface.

Recommended model:

```rust
struct StatusModel {
    tone: StatusTone,
    message: String,
    mode_label: String,
    focus_id: Option<String>,
    filter_summary: Option<String>,
    dirty: bool,
    autosave: bool,
    view_mode: ViewMode,
}
```

Render this as a richer bottom bar instead of a single transient sentence. The message stays transient, but the rest is stable context.

## Command Palette State

Recommended model:

```rust
enum PaletteItemKind {
    Action,
    NodeJump,
    SavedView,
    Recent,
    HelpTopic,
}

struct PaletteItem {
    kind: PaletteItemKind,
    title: String,
    subtitle: String,
    score: i64,
    target: PaletteTarget,
}

struct CommandPaletteState {
    query: String,
    cursor: usize,
    selected: usize,
    items: Vec<PaletteItem>,
}
```

`PaletteTarget` should be an enum, not a closure, so it is easy to test and serialize for recents.

## Help State

The current help overlay is static. Phase 1 should make it searchable.

Recommended model:

```rust
struct HelpTopic {
    id: &'static str,
    title: &'static str,
    body: &'static [&'static str],
    tags: &'static [&'static str],
}

struct HelpOverlayState {
    query: String,
    cursor: usize,
    selected: usize,
    topics: Vec<HelpTopicMatch>,
}
```

Implementation note:

- keep topics in Rust for Phase 1
- do not build a Markdown parser for help yet
- later, help topics can be generated from docs content if that becomes valuable

## Theme And Settings State

Themes should persist per map in the same spirit as session and saved views.

Recommended model:

```rust
enum ThemeId {
    Workbench,
    Paper,
    Blueprint,
    Calm,
    TerminalNeon,
}

struct UiSettings {
    theme: ThemeId,
    motion_enabled: bool,
    ascii_accents: bool,
}
```

Recommended sidecar:

- `.<map-file>.mdmind-ui.json`

Reasoning:

- theme preference is part of the local working surface
- this matches the existing local-first sidecar pattern
- it avoids inventing global config before the product needs one

## Focused Views

Focused views should be implemented as visible-row projection, not as document mutation.

Recommended rule:

- keep `Editor` unchanged
- keep `expanded` unchanged
- derive visible rows from `document + focus_path + filter + view_mode + expanded`

That means `view_mode.rs` should expose a pure function:

```rust
fn project_visible_rows(
    document: &Document,
    expanded: &HashSet<Vec<usize>>,
    focus_path: &[usize],
    filter: Option<&ActiveFilter>,
    view_mode: ViewMode,
) -> Vec<VisibleRow>
```

Mode rules:

- `FullMap`: current behavior
- `FocusBranch`: show focus, ancestors, siblings, descendants, and dim unrelated rows
- `SubtreeOnly`: show focused node and descendants only
- `FilteredFocus`: start from the active filter scope, then add local focus context

Phase 1 can implement dimming by style and elision labels rather than animation-heavy transitions.

## Keyboard Flows

## Command Palette

Open:

- `:`
- `Ctrl+P`

Flow:

- type to search actions, nodes, views, recents, and help topics
- `↑` / `↓` moves selection
- `Tab` cycles result groups if needed
- `Enter` executes
- `Esc` closes

Ranking rules:

- exact id matches first
- prefix text matches next
- action names boosted when query looks command-like
- help topics boosted for `how`, `help`, `theme`, `view`, `export`

## Searchable Help

Open:

- `?`

Flow:

- `?` opens the help overlay on the search field, not a static card
- typing filters topics live
- `Enter` opens the selected topic or runs the selected guided action
- `Esc` closes

Phase 1 topic groups:

- navigation
- editing
- search and filters
- view modes
- exports
- syntax

## Focused Views

Controls:

- `v` cycles `FullMap -> FocusBranch -> SubtreeOnly -> FilteredFocus`
- `V` opens a small picker if the cycling behavior feels too opaque
- `Esc` returns to `FullMap` when a focused mode is active

Status behavior:

- current view mode is always visible
- switching view mode updates the transient status message

## Themes

Controls:

- palette action: `theme`
- optional direct key later, but not required in Phase 1

Flow:

- open palette
- type `theme`
- choose a theme
- preview applies immediately
- `Enter` commits and persists

## Render Plan

## Header

Keep the header, but reduce it to stable identity:

- app name and version
- map path
- dirty and autosave state

## Status Area

Turn the current status box into the real contextual footer.

Show:

- transient message
- view mode
- focus id
- filter match count
- dirty/autosave state

## Keybar

Shrink the current keybar once the palette exists.

Replace the long list with:

- core movement hints
- palette hint
- help hint
- mode-specific hints when overlays are open

## Help Overlay

Phase 1 help overlay should have two panes:

- search/topic list on the left
- selected topic or recipe on the right

This keeps it consistent with the product’s split-pane mental model.

## Data And Persistence

Keep existing sidecars:

- session
- saved views

Add:

- UI settings sidecar for theme and motion preferences

Do not combine all sidecars in Phase 1. That migration creates more risk than value right now.

## Implementation Sequence

## Step 1: Create New Module Boundaries

- add `src/tui/mod.rs`
- move palette constants and style helpers into `theme.rs`
- move help topic rendering helpers into `help.rs`
- move status rendering into `status.rs`

Goal:

- `interactive.rs` compiles but gets smaller immediately

## Step 2: Introduce `ViewMode`

- add the enum
- thread it through `TuiApp`
- extract visible-row projection
- add `v` cycling

Goal:

- focused views work before the palette exists

## Step 3: Introduce Overlay Enum

- replace separate `prompt`, `search`, `mindmap`, and `help_open` state with `overlay: Option<Overlay>`
- keep behavior the same first

Goal:

- future palette/help interactions do not multiply boolean state

## Step 4: Build Command Palette

- create palette item providers
- implement result ranking
- wire node jump and action execution
- include help topics as palette results

Goal:

- one universal entry point for actions and jumps

## Step 5: Replace Static Help With Searchable Help

- convert `?` to open searchable help
- keep existing help content, but break it into topics
- add guided examples and recipe-style topics

Goal:

- help becomes a working surface, not a dead-end overlay

## Step 6: Refresh Status Bar

- add structured status model
- render stable context plus transient message
- reduce duplicate information between header and footer

Goal:

- the app always explains its current state clearly

## Step 7: Add Theme System

- define built-in themes
- add `UiSettings`
- persist selected theme to sidecar
- thread theme surfaces through header, outline, overlays, and status line

Goal:

- screenshots look intentional and users can keep a preferred surface

## Testing Strategy

## Pure Unit Tests

Add tests for:

- palette ranking
- help topic matching
- visible-row projection by `ViewMode`
- theme settings serialization

These should not require a terminal backend.

## App Interaction Tests

Keep the current style of `interactive.rs` tests for:

- `:` opens palette
- palette can jump to a node
- `?` opens searchable help
- `v` cycles view modes and preserves focus
- selecting a theme persists and reloads

## Regression Tests

Keep coverage for:

- search overlay
- mindmap overlay
- autosave
- delete confirmation
- saved views

The biggest risk is overlay state regression while moving from multiple booleans to `Overlay`.

## Acceptance Criteria

- the palette can run common actions and jump to nodes
- help is searchable and useful without leaving the terminal
- focused view modes reduce noise without breaking navigation
- the status line makes app state obvious
- theme choice persists locally

## Recommended First PR Breakdown

1. `ViewMode + visible row projection + tests`
2. `Overlay enum refactor with behavior preserved`
3. `Command palette core`
4. `Searchable help`
5. `Status line refresh`
6. `Theme system + UI settings sidecar`

This ordering reduces merge risk and keeps each PR reviewable.
