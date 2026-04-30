# Relations And Backlinks

Relations let one branch point to another without forcing the tree itself to carry every connection.

Supported syntax:

- `[[target/id]]`
- `[[rel:kind->target/id]]`

Examples:

```text
- Launch Readiness [[rel:blocked-by->product/api-design]]
- Chapter 8 Reunion [[characters/mara]] [[locations/observatory]]
```

What ships today:

- relation parsing and validation
- CLI inspection with `mdm relations`
- backlinks derived automatically from incoming references
- palette relation jumps
- `]` for outgoing relations
- `[` for backlinks
- picker overlay when there is more than one target
- relation edges in the mindmap when both endpoints are visible

This is valuable because many real maps have lateral structure:

- requirements to tasks
- prompts to product work
- characters to themes
- quests to places and factions

Related docs:

- [CROSS_LINKS_AND_BACKLINKS.md](../../../CROSS_LINKS_AND_BACKLINKS.md)
