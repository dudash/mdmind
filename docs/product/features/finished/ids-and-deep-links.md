# Ids And Deep Links

Ids are the stable reference system for `mdmind`.

Use them when a branch should be:

- reopened reliably
- exported directly
- referenced from another branch
- linked from docs, scripts, or notes

Syntax:

```text
- Tasks [id:product/tasks]
```

You can then open:

- `mdmind map.md#product/tasks`
- `mdm view map.md#product/tasks`
- `mdm export map.md#product/tasks --format json`

Recent improvement:

- if no explicit id matches, `mdmind` and `mdm` now fall back to label paths like `map.md#Product Idea/Tasks`

That fallback is great for quick exploration, but ids are still the better durable target.

Related docs:

- [IDS_AND_DEEP_LINKS.md](../../../IDS_AND_DEEP_LINKS.md)
