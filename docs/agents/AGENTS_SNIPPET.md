# mdmind AGENTS.md Snippet

Paste this compact note into a project `AGENTS.md` when agents should always know
the basic mdmind shape. It complements the deeper mdmind skills; it does not
replace them.

```md
## mdmind Maps

Use mdmind when the user wants durable hierarchy, structured notes, deep links,
relations, or agent-readable project memory. Prefer normal Markdown for short
summaries or prose-only answers.

Native mdmind syntax is Markdown outline text:

- concise node labels form the tree
- optional `#tags` mark lightweight categories or workflow state
- optional `@key:value` metadata supports repeated structured fields
- optional `[id:stable/id]` anchors durable branches for links and CLI lookup
- optional `| detail` lines hold prose, rationale, quotes, or context
- optional `[[target/id]]` or `[[rel:kind->target/id]]` links distant branches

Default authoring rule: readable tree first, then add the smallest useful layer
of tags, metadata, ids, details, or relations. Do not put ids on every node.

Validation loop:

1. Generate or edit the map.
2. Run `mdm validate <file>` when `mdm` is available.
3. Inspect with the smallest useful command: `mdm view`, `mdm find`, `mdm links`,
   `mdm relations`, or `mdm export`.
4. Revise labels, ids, metadata, and relations until the map is readable and
   passes validation.

Use deeper skills when available:

- `mdmind-map-authoring` for turning notes, plans, meetings, research, or writing
  into native mdmind maps.
- `mdm-cli-inspection` for validating, querying, deep-linking, or exporting
  existing maps with the `mdm` CLI.
```
