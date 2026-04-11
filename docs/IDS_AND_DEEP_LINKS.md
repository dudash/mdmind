# Ids And Deep Links

Ids are how a branch becomes reliably addressable in `mdmind`.

If you add `[id:product/api-design]` to a node, that branch now has a stable target that the TUI, the CLI, exports, and cross-links can all agree on. Without an id, you can still browse by visible label and tree position, but the branch is not as durable as a long-term reference point.

## What An Id Is

An id is an inline token on a node line:

```text
- API Design #backend [id:product/api-design]
```

That token does three useful things:

- it gives the branch a stable address
- it makes palette and prompt jumps more reliable
- it gives other features a durable target to point at

## When To Add Ids

You do not need ids on every node.

Good candidates:

- major branches you revisit often
- durable work items
- sections you want to deep-link from the CLI
- branches you plan to export by subtree
- branches other nodes will reference with `[[target]]`

Poor candidates:

- tiny throwaway leaves
- branches whose names and role are still changing constantly
- every single node in a small one-off map

## How Ids Show Up In The Product

In the TUI:

- `o` opens the jump-to-id prompt
- the palette understands both `product/api-design` and `[id:product/api-design]`
- you can open `mdmind` directly to a deep link with `mdmind map.md#product/api-design`
- help, saved views, history, and relation jumps all work better when important branches have ids

In the CLI:

```bash
mdm links map.md
mdm view map.md#product/api-design
mdm export map.md#product/api-design --format mermaid
```

`mdm links` is the best inspection command when you are unsure what ids a file currently exposes.

## Naming Advice

Good id paths are:

- short
- stable
- lower-friction to type
- descriptive enough to recognize later

Examples:

- `product/api-design`
- `team/sprint-review`
- `chapters/chapter-08`
- `quests/main/moonlit-embassy`

Try to avoid ids that depend too heavily on the current visible wording of the label. The label can evolve. The id should stay dependable.

## Deep Links

Once a node has an id, you can deep-link to it from supported commands:

```bash
mdmind roadmap.md#product/api-design
mdm view roadmap.md#product/api-design
mdm open roadmap.md#product/api-design
mdm export roadmap.md#product/api-design --format json
```

This is especially useful in docs, scripts, or notes where you want to point at one exact branch instead of a whole map.

If no explicit id matches, `mdm` and `mdmind` can also fall back to a label path:

```bash
mdmind roadmap.md#Product Idea/Tasks
mdm view roadmap.md#Product Idea/Tasks
```

That fallback is useful for quick exploration and early maps. For durable references, ids are still the better choice because label paths can become ambiguous or drift when labels change.

## Troubleshooting

If a deep link fails:

1. run `mdm links map.md`
2. confirm the exact id path
3. check whether the node still carries `[id:...]`

If you expect a branch to be cross-linkable later, give it an id first. Relations and backlinks depend on stable targets.
