# Codex Kickoff Brief

You are starting a clean-slate start for `mdmind`.

Read these files first:
- `VISION.md`
- `DESIGN.md`
- `MVP.md`
- `FORMAT.md`
- `DECISIONS.md`
- `ROADMAP.md`


## Goals

Build a clean architecture around:
- a human-readable tree format
- inline tags
- inline metadata
- node ids
- deep links
- strong CLI ergonomics based on clig.dev

## Core product surfaces

- `mdm`: CLI
- `mdmind`: TUI

## Primary use cases

- idea exploration
- requirements definition
- TODO/status breakdowns
- prompt refinement
- project templates

## Constraints

- no backwards compatibility requirement
- local-first
- files remain readable
- avoid plugin/service/AI complexity early
- core library must not depend on UI layer

## What to produce first

1. Proposed directory structure
2. Core data model
3. Parser / serializer plan
4. CLI command scaffold
5. Test strategy
6. Initial implementation plan with milestones

## Important design rules

- Do not start with the TUI
- Do not recreate a monolithic quick mode file
- Do not mix parser logic with rendering
- Make stdout/stderr separation explicit
- Make command help excellent
- Keep file format simple and durable

## First implementation target

Build the core parser/model/search layer and the following commands:
- `view`
- `find`
- `tags`
- `kv`
- `links`
- `validate`
- `export`
- `init`

## Success criteria for early milestone

A user can:
- create a map from a template
- inspect it as a tree
- search it
- list tags
- list metadata
- list ids
- validate it
- export it as JSON
