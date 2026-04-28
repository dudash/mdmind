# Query And Export Notes

## Query Syntax

The shared query language works on:

- plain text
- `#tags`
- `@key:value` metadata
- combinations of the above

Examples:

- `rate limit`
- `#todo`
- `@status:active`
- `#todo @owner:jason`

Start broad, then narrow.

## Plain Versus JSON

Use `--plain` when:

- the result is for a human
- you want grep-friendly output
- you are quickly exploring a map

Use `--json` when:

- another tool needs structured output
- the user wants machine-readable results

Do not combine `--plain` and `--json` on commands that support both.

## Deep Links

Use deep links when one branch matters more than the full map:

- `map.md#product/tasks`
- `map.md#research/themes/ownership`

Prefer id-based deep links over label-path fallbacks when possible.

## Export Scope

Whole tree:

```bash
mdm export map.md --format json
```

One subtree:

```bash
mdm export map.md#product/tasks --format mermaid
```

Filtered subset:

```bash
mdm export map.md --query "#todo @status:active" --format json
```

If a filtered export returns no nodes, the query is too narrow or wrong.

## Good Answering Pattern

When helping a user:

1. choose the smallest command that answers the question
2. run `validate` first if the file may be malformed
3. use deep links to reduce noise
4. summarize the result in prose instead of dumping raw command output without context
