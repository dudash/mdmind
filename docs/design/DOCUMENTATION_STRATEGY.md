# Documentation Strategy

## Goal

Make mdmind easy to understand, adopt, and contribute to without making every
repo document look like a user manual.

The docs now use separate lanes:

- user guides
- embedded help source
- exact reference
- agent guidance
- design notes
- product docs

## Why This Changed

The old flat `docs/` folder mixed day-to-day guides, design notes, shipped
feature summaries, future feature shelves, and agent material. That made it hard
to tell whether a file was current user guidance, product rationale, or an old
planning note.

Feature task tracking now lives in Linear, so repo docs should not pretend to be
the active backlog.

## Current Structure

- `docs/manual/`: task-focused user guides. These are the best source material
  for a future public docs site.
- `docs/help/`: source companion material for embedded TUI help.
- `docs/reference/`: exact behavior and durable contracts.
- `docs/agents/`: agent usage, CLI contract, skills, snippets, and eval docs.
- `docs/design/`: design rationale and historical product thinking.
- `docs/product/shipped/`: concise summaries of current product capabilities.
- `docs/product/prds/`: PRDs and product strategy docs worth reviewing in repo.
- `docs/product/_archive/`: old feature shelves kept for context only.
- `docs/_archive/`: older historical planning documents.

## Ownership Rules

- If a user should read it to learn the product, put it in `docs/manual/`.
- If it is embedded-help source or a help companion, put it in `docs/help/`.
- If it describes exact syntax, query behavior, ids, relations, or contracts,
  put it in `docs/reference/`.
- If it is about coding-agent use, put it in `docs/agents/`.
- If it explains why the product or UX was shaped a certain way, put it in
  `docs/design/`.
- If it is a shipped capability summary or PRD, put it in `docs/product/`.
- If it is an implementation task, priority decision, or future backlog item,
  track it in Linear instead of adding another roadmap doc.

## Built-In Help

The TUI help is currently code-defined in `src/interactive.rs`. The map-shaped
guide in `docs/help/USER_GUIDE.md` should stay aligned with those topics, but it
is not loaded by path at runtime.

This means the embedded help keeps working when repo docs move, as long as the
TUI help tests keep passing.

## Public Docs Site

A future public docs site should draw primarily from:

- `docs/manual/`
- `docs/reference/`
- `docs/agents/`
- `docs/product/shipped/`

The site should not expose design notes and archived shelves as the main reader
path. Those can stay linked for contributors and maintainers.

## Acceptance Criteria

- A first-time reader can tell where to start.
- A maintainer can tell whether a file is manual, reference, design, product, or
  agent guidance from its path.
- Active roadmap and implementation work is not duplicated between Linear and
  repo docs.
- The embedded TUI help tests pass after docs moves.
- The doc link checker passes for real docs links.
