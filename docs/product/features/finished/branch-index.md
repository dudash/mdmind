# Branch Index

`mdmind` now has a lightweight section list for large maps without adding another permanent pane or view mode.

What ships today:

- a `Sections` shelf at the top of the empty-query command palette
- searchable section jumps when you type branch or section names in the palette
- major branches first, then meaningful second-level sections when you narrow by query
- a `Browse Sections` palette action for explicit discovery

Why it matters:

- large maps are easier to scan and jump through
- outliner-first users get a familiar “section list” behavior
- the feature composes cleanly with recent places, saved views, and relations instead of competing with them

Important design rule:

- this is a navigation surface, not a view mode

Related docs:

- [PALETTE_AND_HELP.md](../../../PALETTE_AND_HELP.md)
- [USING_MDMIND_AS_OUTLINER.md](../../../USING_MDMIND_AS_OUTLINER.md)
