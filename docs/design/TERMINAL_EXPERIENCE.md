# Terminal Experience

## Goal

Make `mdmind` feel like a best-in-class terminal product:

- fast
- confident
- visually intentional
- discoverable without being noisy

This is not about decorative polish alone. It is about helping a user think faster and stay oriented in large maps.

For the concrete near-term execution sequence, see `docs/PHASE1_UX_IMPLEMENTATION.md`.

## Inspiration Direction

The target feel is informed by strong terminal products such as Gemini CLI, Claude Code, Codex CLI, LazyGit, K9s, Glow, btop, and superfile:

- clear visual hierarchy
- strong status surfaces
- fast command entry
- safe power-user workflows
- tasteful identity and motion

`mdmind` should not copy their exact UI patterns. It should adapt the underlying product lessons to a tree-first mind-mapping workflow.

## Product Principles

- Make state obvious: focus, filter, view mode, dirty state, autosave, and export target should always be legible.
- Make power safe: destructive edits should feel reversible and inspectable.
- Make discovery fast: users should not need to memorize every key.
- Make large maps calmer: focus, dimming, grouping, and local context matter more than raw density.
- Make delight restrained: color, motion, and ASCII flourishes should feel intentional and optional.

## Experience Pillars

### Commandability

The app needs one universal entry point for actions, jumps, and help.

Primary work:

- command palette
- fuzzy node jump
- recent actions and recent locations
- reusable recipes and guided actions

### Focus And Calm

The app should reduce noise when a user is working inside one branch.

Primary work:

- focused branch mode
- subtree isolate mode
- dimmed context
- subtle recentering and transitions

### Visual Identity

The app should look like a designed terminal product, not a default widget demo.

Primary work:

- theme system
- richer status line
- better badges, pills, separators, and borders
- small ASCII moments in help, empty states, and startup

### Confidence

Users will move faster once structural edits feel safe.

Primary work:

- undo and redo
- local checkpoints
- action history
- preview before destructive changes

### Spatial Delight

The existing mindmap overlay should become more legible and more memorable before it becomes more freeform.

Primary work:

- better focus path rendering
- animated open and close
- clearer branch clustering
- better export polish

## Feature Set

## 1. Command Palette 1.0

Open with `:` or `Ctrl+P`.

Capabilities:

- action search
- fuzzy node jump by text, id, tag, or metadata
- recent filters
- recent nodes
- export actions
- built-in help results

Acceptance criteria:

- a user can jump to a branch in a large map in a few keystrokes
- a user can run common actions without remembering dedicated hotkeys
- result groups feel ranked, not random

## 2. Focused Views And Motion

Capabilities:

- full map
- focus branch
- subtree only
- filtered focus
- restrained motion when switching modes or changing focus

Acceptance criteria:

- view mode is always visible in status UI
- switching modes preserves focus path and expanded state
- motion never blocks typing or navigation

## 3. Themes And Surface Design

Capabilities:

- multiple built-in themes
- live palette preview for themes and surface settings
- per-theme status line styling
- improved border and badge language
- optional startup mark, help art, and ASCII chrome accents

Suggested theme set:

- Workbench
- Paper
- Blueprint
- Calm
- Violet
- Amethyst
- Atelier
- Archive
- Signal
- Tokyo Mind
- Monograph
- Terminal Neon

Acceptance criteria:

- the app has a recognizable visual identity in screenshots
- theme switching is immediate and persists locally
- low-contrast themes are rejected in favor of readable defaults

## 4. Safe Power Workflows

Capabilities:

- undo and redo
- automatic checkpoints before large structural edits
- recent action timeline
- preview before delete and major reparenting

Acceptance criteria:

- a user can recover from an accidental structural edit without leaving the app
- the UI distinguishes pending, applied, and reverted actions clearly

## 5. Mindmap Delight Pass

Capabilities:

- focus path highlight
- neighbor emphasis around the selected node
- animated transitions when opening or recentering
- improved export styling for PNG output

Acceptance criteria:

- the overlay is easier to read than today on medium maps
- exported PNGs look intentional and presentation-ready

## 6. Recipes And Guided Actions

Capabilities:

- built-in workflows such as `capture brainstorm`, `review blocked items`, and `export subtree`
- palette integration
- recipe descriptions and preview text

Acceptance criteria:

- a new user can discover useful workflows without reading external docs first
- recipes feel like shortcuts over existing primitives, not a separate mode system

## Delivery Plan

## Phase 1

- command palette
- focused views
- status line refresh
- searchable help entry point

## Phase 2

- theme system
- restrained motion
- richer empty states
- better mindmap overlay polish

## Phase 3

- undo and redo
- checkpoints
- action history
- recipe system

## Phase 4

- stronger canvas behavior
- relation edges
- presentation-grade exports

## Non-Goals

- turning `mdmind` into a general terminal IDE
- forcing motion on users who want a static interface
- replacing the outline with the canvas too early
- adding novelty visuals that hurt map readability

## Product Test

The work succeeds when:

- screenshots of the app feel designed and recognizable
- large maps feel calmer and faster to navigate
- users can discover more of the product without leaving the terminal
- structural editing feels safe enough to use aggressively
