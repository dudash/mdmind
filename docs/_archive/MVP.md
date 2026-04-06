# MVP

Goal: ship a clean, useful app that is clearly better than a prototype in architecture and CLI ergonomics.

## Must-have outcomes

1. Clean parser + serializer
2. Useful CLI commands
3. Human-readable files
4. Deep links
5. Searchable tags and metadata
6. Strong automated tests

## MVP scope

### Core library
- Parse tree structure from file
- Extract and serialize:
  - tags
  - kv metadata
  - ids
- Search node text
- Search tags
- Search metadata
- Resolve deep links
- Export normalized JSON

### CLI
- `view`
- `find`
- `tags`
- `kv`
- `links`
- `open`
- `validate`
- `init`

### Output behavior
- pretty output by default
- `--json` on data commands
- `--plain` on data commands where useful
- stdout for data
- stderr for errors and status

### File/template support
- starter templates for:
  - product idea
  - feature requirements
  - prompt exploration
  - backlog / todo map

## Deferred from MVP

- advanced TUI
- Mermaid export
- service mode
- collaboration
- AI actions
- plugin system
- diagram nodes
