# Command Palette And Help

The command palette and built-in help are two of the most important comfort features in `mdmind`.

They exist for a simple reason: once a map gets real, you should not have to remember exactly where every command, branch, view, recipe, or relation lives.

## The Command Palette

Open it with:

- `:`
- `Ctrl+P`

The palette is best when you already know your intent.

Good palette use cases:

- jump to a branch by name
- jump to an id
- open a saved view
- revisit a recent location
- restore a checkpoint
- browse undo or redo history
- run a workflow recipe
- jump across a relation or backlink
- open a help topic

The palette is not just an action menu. It is the universal “take me there” surface.

## What To Type

You do not need special syntax for most palette use.

Examples:

```text
tasks
product/api-design
#todo
@status:active
review todo
owner
checkpoint
undo
backlink
theme
help
```

That range is the point. One surface can handle navigation, workflows, recovery, appearance, and help.

## When The Palette Is Better Than Search

Use search when you want to narrow the visible working set.

Use the palette when you already know the target or intent.

Examples:

- “show me all blocked items” is usually search
- “take me to the blocked launch branch” is usually palette
- “open the subtree export flow” is palette
- “bring back that branch I was editing earlier” is palette

## Built-In Help

Open it with:

- `?`

Help should feel like a guide, not a dead end.

The current built-in help is designed as one surface called `Help`, but internally it mixes:

- short user-guide explanations
- command reference
- practical tips

That keeps it unified in the product while still letting different topics teach in different ways.

## The Best Way To Use Help

If you are new:

- start with `Start Here`
- then read `Using mdmind As An Outliner` if that framing fits how you work
- read `Using mdmind With Agents` if you expect an agent to generate maps or outlines for you
- then read `Navigation`, `Editing`, and `Search And Filters`
- treat `Ids And Deep Links` and `Relations And Backlinks` as power-feature guides you can adopt later

If you are experienced:

- search for the exact topic or phrase you want
- use help as a fast refresher instead of browsing the whole tree of docs

## Why Palette And Help Belong Together

These two surfaces solve a similar problem:

- the palette helps you act
- help helps you understand

Keeping them close makes the app easier to learn because the same user who types `review todo` in the palette can also type `deep link` or `subtree` in help and get an explanation right away.

## Practical Advice

- use the palette when you know the target
- use help when you know the question
- use search when you need to reduce the working set
- do not force yourself to memorize every key if a named surface is faster

## Related Docs

- [USING_MDMIND_AS_OUTLINER.md](USING_MDMIND_AS_OUTLINER.md)
- [TUI_WORKFLOWS.md](TUI_WORKFLOWS.md)
- [QUERY_LANGUAGE.md](QUERY_LANGUAGE.md)
- [IDS_AND_DEEP_LINKS.md](IDS_AND_DEEP_LINKS.md)
- [CROSS_LINKS_AND_BACKLINKS.md](CROSS_LINKS_AND_BACKLINKS.md)
- [AGENT_USAGE.md](AGENT_USAGE.md)
