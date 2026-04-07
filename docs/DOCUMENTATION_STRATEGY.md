# Documentation Strategy

## Goal

Make the project easy to understand, adopt, and contribute to.

Documentation should work at three levels:

- quick orientation for first-time visitors
- task-focused guidance for daily users
- precise reference for power users and contributors

## Problems To Solve

- the repo already has useful feature notes, but they are product briefs more than end-user docs
- the CLI and TUI are growing faster than the current help surfaces
- users should not need to read source code to learn the map format, query language, or workflows
- a public docs site will eventually be better than a repo-only reading experience

## Audience Layers

### Curious Visitor

Needs:

- what the tool is
- why it is different
- how it looks
- how to try it in five minutes

### Daily User

Needs:

- keybindings
- workflows
- query patterns
- export behavior
- troubleshooting

### Contributor

Needs:

- codebase map
- testing workflow
- release flow
- architecture notes

## Documentation System

## Layer 1: Repo Docs

The repository should remain the source of truth for versioned docs tied to code changes.

Recommended structure:

- `README.md`: product pitch, install, quickstart, screenshots, and primary links
- `docs/format.md`: file format, ids, tags, metadata, and deep links
- `docs/query_language.md`: search grammar and examples
- `docs/tui.md`: navigation, editing, filters, views, and mindmap mode
- `docs/export.md`: formats, subtree export, and future import behavior
- `docs/recipes.md`: common real tasks
- `DEVELOPER.md`: setup, testing, release, and contributor workflow

The current design notes in `docs/` should stay, but they should be clearly separated from end-user guides.

## Layer 2: Built-In Docs

The terminal product should teach itself.

Recommended surfaces:

- `?` opens searchable help inside `mdmind`
- `mdm help <topic>` for focused CLI guidance
- built-in examples for map syntax and query patterns
- recipe-oriented answers such as `how do I export one branch?`
- keyboard cheatsheet and mode-specific help

This layer matters because many users will never open a browser while actively working in the tool.

The first implementation slice for searchable help and guided in-app documentation is outlined in `docs/PHASE1_UX_IMPLEMENTATION.md`.

## Layer 3: Public Docs Site

Once the product surface is broader, add a polished documentation site.

Recommendation:

- use Starlight for a branded docs experience with room for tutorials, guides, and screenshots

Alternative:

- use Material for MkDocs if the priority is a faster Markdown-first setup with less design work

Suggested site sections:

- Quickstart
- Why mdmind
- Map Format
- Query Language
- CLI
- TUI
- Mindmap Mode
- Export And Import
- Templates
- Recipes
- Screenshots And Demos
- FAQ
- Contributing

## Information Architecture Principles

- task-first before reference-first for the main docs path
- every advanced feature should have a small example
- every mode should explain what problem it solves
- screenshots and short terminal captures should support the docs
- keep contributor docs separate from end-user docs

## Delivery Plan

## Phase 1: Repo Cleanup

- tighten `README.md`
- create missing end-user docs for format, query language, TUI, and export
- keep feature briefs in `docs/` but link them as design notes

## Phase 2: Built-In Help

- searchable help overlay in `mdmind`
- richer `mdm --help`
- topic-based CLI help such as `mdm help export`
- examples and recipes embedded in help output

## Phase 3: Public Docs Site

- launch docs site
- add screenshots, GIFs, and copy that explains the product feel
- publish versioned docs if the release process justifies it

## Content Backlog

- install paths and platform notes
- map format tutorial
- query cookbook
- export cookbook
- TUI workflows for planning, backlog shaping, and prompt libraries
- animated demos of the mindmap overlay
- architecture overview for contributors

## Acceptance Criteria

- a first-time visitor can understand the product and run it quickly from `README.md`
- a user can learn core TUI workflows from built-in help without external docs
- the public docs site can become the canonical browseable manual without diverging from repo docs

## Product Test

The docs succeed when the most common usage and contribution questions can be answered without opening source files or reading archived planning notes.
