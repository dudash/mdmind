# Export Targets

## Goal

Let users move maps into other tools without losing too much structure.

## Problem

JSON is useful for machines, but users also need formats that work in documentation, diagramming, and outlining tools.

## Candidate Targets

- Mermaid: for quick diagrams inside docs and repos
- OPML: for interchange with outliners
- Markdown report views: flattened task or requirements summaries

## UX

Extend `mdm export` with a format flag:

- `mdm export map.md --format json`
- `mdm export map.md --format mermaid`
- `mdm export map.md --format opml`
- `mdm export map.md#product/tasks --format mermaid`
- `mdm export map.md --query "#todo @status:active" --format json`

For diagram-like targets, expose a few view choices:

- full tree
- current subtree
- filtered subtree

Today:

- full tree works by default
- current subtree works by exporting a deep link target such as `map.md#node/id`
- filtered subtree works with `--query`

## Mapping Rules

Mermaid:

- prioritize flowchart or mindmap-compatible output
- ids become stable node references
- tags and metadata should be selective to avoid clutter

OPML:

- preserve labels and hierarchy
- ids and metadata become attributes when possible

## Architecture

- exporter trait or module boundary per target
- one normalized export tree as source

## Delivery

Phase 1:

- Mermaid
- OPML
- done

Phase 2:

- better formatting options

## Risks

- format-specific quirks leaking into the core model
- exported diagrams becoming unreadable on large maps

## Product Test

A user should be able to take a map into documentation or another outliner without doing manual restructuring first.
