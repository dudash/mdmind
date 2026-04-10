# Working In mdmind

This page is about the day-to-day feel of using `mdmind`.

It is not a full feature reference. It is the practical workflow guide for how someone actually works in the TUI once a map exists.

## The Core Loop

Most work in `mdmind` follows the same pattern:

1. move to the part of the tree you care about
2. add, edit, or reshape a few nodes
3. narrow the visible surface if the map gets noisy
4. save, checkpoint, or undo when needed

That is the product.

Everything else, including ids, cross-links, themes, and the visual mindmap, exists to make that loop clearer, faster, or safer.

## First Session Workflow

If you are new:

1. open a small map in `mdmind`
2. move with `↑` and `↓`
3. use `→` to expand or enter a branch
4. press `a` to add a child
5. press `e` to edit a line
6. press `/` to search
7. press `:` or `Ctrl+P` to open the palette
8. press `?` to open built-in help

That is enough to become productive.

## Browsing The Tree

The outline is still the main working surface.

Use it when:

- you are learning a map
- you are reviewing structure
- you are making local edits
- you are scanning nearby context

Important movement keys:

- `↑ / ↓` move through visible nodes
- `←` collapses a branch or moves to the parent
- `→` expands a branch or enters the first child
- `Enter` or `Space` toggles expansion
- `g` jumps to the root

If the tree starts feeling noisy, do not keep scrolling blindly. Change the visible working set first.

## Editing Without Losing Your Place

`mdmind` works best when editing feels local.

Use:

- `a` to add a child
- `A` to add a sibling
- `Shift+R` to add a root branch
- `e` to edit the current node
- `d` to edit longer details for the current node
- `x` to delete after confirmation
- `Alt+↑ / Alt+↓` to reorder among siblings
- `Alt+← / Alt+→` to move out or indent into the previous sibling

The important part is that edits happen relative to the current focus. You are shaping a branch, not switching into a different editing application.

### When To Use Details

Node labels should stay short enough to scan quickly in the tree.

Use details when one branch needs:

- a paragraph of rationale
- a quote
- meeting notes
- a scene note
- a short research excerpt

In the raw file, details are stored as `| ...` lines directly under the node. In the TUI, press `d` to open the detail editor, use `Enter` for new lines, and `Ctrl+S` to save.

## Narrowing The Working Set

When the map gets large, reduce the visible surface instead of relying on memory.

You have three main tools:

### Search

Use `/` for queries, `b` for browse, and `w` for saved views.

Good beginner progression:

1. plain text
2. `#tags`
3. `@key:value`
4. combined queries

Examples:

```text
rate limit
#todo
@status:active
#todo @owner:jason
```

### View Modes

Use `v` or `V` to change projection.

- `Full Map`: broad orientation
- `Focus Branch`: local context plus nearby structure
- `Subtree Only`: one branch becomes a temporary workspace
- `Filtered Focus`: search-driven work with enough context to stay oriented

The key mental model is: view modes change what you see, not the underlying document.

### Palette

Use `:` or `Ctrl+P` when you already know what you want:

- a branch
- an id
- a saved view
- a recent location
- a workflow recipe
- a relation jump
- a help topic

The palette is the fastest high-level jump surface in the product.

## A Good “Work Inside One Branch” Flow

This is one of the most useful everyday patterns:

1. move focus to the branch you care about
2. switch to `Subtree Only`
3. do your add, edit, and reorder work locally
4. use `g` to return to the subtree root if you drift
5. press `Esc` when you want the broader map back

This is better than manually collapsing unrelated branches because the mode is explicit and reversible.

## A Good “Review Work” Flow

For task-heavy maps:

1. open search
2. query `#todo`, `@status:active`, or a combined filter
3. move through results with `n` and `N`
4. save the filter if it is recurring
5. switch to `Filtered Focus` if you want the filtered working set to dominate the screen

If the map has stable `@owner` or `@status` values, the palette can also surface contextual recipes so you do not need to remember every exact filter string.

## Relations In Daily Use

Relations are useful when the tree is not enough on its own.

Use them for:

- dependencies
- support links
- chapter-to-character references
- quest-to-region or quest-to-faction links

Daily interaction model:

- use `]` to follow outgoing relations
- use `[` to follow backlinks
- if there is more than one possible target, `mdmind` opens a small picker
- use the palette when you want a more explicit relation jump surface

Relations should add clarity, not replace basic structure.

## The Visual Mindmap

The mindmap is best used after you already have the right working set.

Good times to open it:

- after isolating a branch
- after applying a filter
- when presenting or reviewing branch shape
- when exporting a PNG

Poor times to open it:

- before you know what scope you care about
- when the full map is still too noisy to read in the tree

The tree should usually lead. The mindmap is the second lens.

## Safety Workflow

The safety layer should make you faster, not slower.

Use:

- `u` to undo
- `U` to redo
- checkpoints before bigger structural changes
- recent history in the palette when you want to jump back more than one step

If a change feels risky, take a manual checkpoint first instead of editing timidly.

## The Calm Setup

If you want a quieter pro surface:

1. open the palette
2. choose the `Monograph` theme
3. turn on `minimal mode`

That setup gives you:

- less shell chrome
- more room for the main outline
- quieter overlays
- a cleaner working feel on large maps

## Practical Advice

- use the tree until the tree stops being enough
- use search when you know what kind of thing you want
- use view modes when noise is the real problem
- use the palette when you already know the target
- use ids for durable branches
- use relations only where lateral structure is genuinely helpful
- use the mindmap after you have already chosen the right scope

## Related Docs

- [USER_GUIDE.md](USER_GUIDE.md)
- [NODE_DETAILS.md](NODE_DETAILS.md)
- [QUERY_LANGUAGE.md](QUERY_LANGUAGE.md)
- [IDS_AND_DEEP_LINKS.md](IDS_AND_DEEP_LINKS.md)
- [CROSS_LINKS_AND_BACKLINKS.md](CROSS_LINKS_AND_BACKLINKS.md)
