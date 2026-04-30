# Focused Views

Focused views are one of the most useful large-map features in `mdmind`.

Available view modes:

- `Full Map`
- `Focus Branch`
- `Subtree Only`
- `Filtered Focus`

What they are for:

- `Full Map`: broad orientation
- `Focus Branch`: work with context
- `Subtree Only`: treat one branch as a temporary workspace
- `Filtered Focus`: keep a filter active without losing all structure

Important design rule:

- changing view mode never rewrites the map
- it only changes the visible projection

This matters because the app stays calm on large maps without forcing you to manually collapse half the tree every time.

Related docs:

- [FOCUSED_VIEWS.md](../../../FOCUSED_VIEWS.md)
- [search-and-browse.md](search-and-browse.md)
- [visual-mindmap.md](visual-mindmap.md)
