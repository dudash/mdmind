# Node Details

`mdmind` keeps the main tree line intentionally short.

That works well for structure, but some branches need more than a short title. A feature branch might need rationale. A meeting branch might need a few notes. A writing branch might need a quote or a scene beat. Node details are the attached long-form layer for that.

## What Details Are For

Use node details when the content belongs to one branch, but would make the main outline harder to scan if it lived in the visible node label.

Good fits:

- rationale for a decision
- meeting notes tied to one branch
- a quote, scene note, or character note
- a small research excerpt
- context for a blocker or dependency

Bad fits:

- child structure that should really be separate branches
- metadata you expect to filter repeatedly
- labels that should just be rewritten to be clearer

## Raw File Format

Details are stored on indented `| ...` lines directly under a node:

```text
- API Design #backend [id:product/api-design]
  | We need one stable auth flow before launch.
  |
  | Open question:
  | Should refresh tokens be scoped per workspace or per user?
  - Auth Flow
  - Token Lifecycle
```

Rules:

- `- ...` is still the node line
- `| ...` attaches detail text to the node above it
- a bare `|` creates a blank line in the detail body
- detail lines come before child branches

## In The TUI

Use:

- `d` to open the detail editor for the focused node
- `Enter` to add a new line
- `Ctrl+S` to save
- `Esc` to cancel

The focus card also shows a short detail preview, so you can read attached notes without opening the editor every time.

## How To Think About It

The clean mental model is:

- node label = fast, scannable structure
- tags and metadata = structured filters
- ids = stable addresses
- relations = lateral links
- details = attached prose

That separation keeps maps readable even when they start holding richer information.

## Related Docs

- [USER_GUIDE.md](USER_GUIDE.md)
- [TUI_WORKFLOWS.md](TUI_WORKFLOWS.md)
- [QUERY_LANGUAGE.md](QUERY_LANGUAGE.md)
