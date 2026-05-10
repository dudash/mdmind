# Checklist Semantics

`mdmind` already handles task-like work well through tags and metadata.

The question is whether it should also feel more like an outline-based checklist tool.

The right direction is probably:

- start with metadata-backed task state
- add TUI rendering and toggles
- avoid inventing heavy new raw syntax too early

Good first shape:

- toggle a done state on the current node
- render that state as an optional checkbox-style marker in the TUI
- let search, browse, and saved views work with it naturally

Current first slice:

- explicit `[ ]` and `[x]` markers parse and serialize as task state
- `Space` toggles a focused task item in the TUI
- parent progress rollups are derived and displayed without rewriting parent rows
- `t` / `T` starts a new TODO child or sibling prompt with `[ ] `
- `task:open`, `task:blocked`, `task:done`, and `task:any` make filters task-aware across checkbox, tag, and status conventions
- `mdm validate` warns on conflicting task state such as `[x] @status:active` or `#todo #done`

This should probably build on existing structure such as:

- `#todo`
- `#done`
- `@status:active`
- `@done:true`

What it should not do:

- turn the file format into a second task-markdown dialect
- replace the more general metadata model
- make normal non-task maps feel task-centric

Complexity guardrail:

If checklist behavior lands well, it should feel like a small, obvious extension of the current map language rather than a new system users need to learn.
