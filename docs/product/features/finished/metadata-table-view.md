# Metadata Table View

The metadata table view is a read-only structured lens over the current outline scope.

It is useful when repeated metadata matters more than hierarchy for a moment: project status maps, research matrices, model comparisons, task maps, and source review maps.

Current behavior:

- `C` or `Open Table View` opens a table over the current visible scope.
- Rows follow the same focus, filter, expansion, and view-mode model as the outline.
- `c` opens the column picker.
- `Node` is always present so every row keeps a clear path back to the outline.
- Metadata columns are discovered from the current visible rows.
- Missing values render as `-`.
- Column choices persist per map in the local `.mdmind-views.json` sidecar.
- `↑` / `↓`, `j` / `k`, `PgUp` / `PgDn`, `Home`, and `End` move through rows.
- `←` / `→` collapse or expand the selected table branch.
- `v` / `V` cycle view modes without leaving the table.
- `Enter` focuses the selected row back in the outline for editing.

The table intentionally does not edit metadata directly yet. The plain-text map remains the source of truth, and normal outline/detail editing remains the editing path.
