# Node Details

Node details solve the “one line is not enough” problem without turning the tree into a document editor.

Raw format:

```text
- API Design [id:product/api]
  | Longer rationale lives here.
  | Quotes, notes, and meeting context work well too.
```

In the TUI:

- `d` opens the detail editor
- `Enter` adds a new line
- `Ctrl+S` saves
- `Esc` cancels

Why this feature matters:

- the tree stays compact and scannable
- longer notes still live close to the right branch
- writing maps, research maps, and product maps all benefit from it

Good uses:

- rationale
- quotes
- research notes
- scene notes
- meeting context

Related docs:

- [NODE_DETAILS.md](../../../NODE_DETAILS.md)
