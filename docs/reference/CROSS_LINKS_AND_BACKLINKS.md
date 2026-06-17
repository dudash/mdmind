# Cross-Links And Backlinks

The tree is still the primary structure in `mdmind`, but sometimes two branches are meaningfully connected even though they should not live under the same parent.

That is what cross-links are for.

## The Simple Form

Use a plain cross-link when you want to point at another branch by id:

```text
- Launch Readiness [[product/api-design]]
```

This says: this node is connected to `product/api-design`.

It does not say why. It just preserves the connection.

Same-file targets are file-scoped. `product/api-design` means an id in the
current map, not a global id across every file.

For a branch in another map, include the path plus `#` plus the branch id:

```text
- Launch Readiness [[maps/decisions.md#product/api-design]]
```

That path is resolved relative to the current map. `mdm validate` checks the
target file and branch id when it can read the referenced Markdown map.

## The Typed Form

Use a typed relation when the meaning matters:

```text
- Launch Readiness [[rel:blocked-by->product/api-design]]
- Launch Readiness [[rel:blocked-by->maps/decisions.md#product/api-design]]
```

This says more than “these are connected.” It says the current branch is blocked by the target branch.

Typed relations are useful for:

- `blocked-by`
- `supports`
- `depends-on`
- `informs`
- `set-in`
- `involves`

The plain form should still be your default. Reach for the typed form when the relation meaning is worth keeping around in search, review, or export.

## Backlinks

Backlinks are derived from incoming cross-links.

That means:

- you never write backlinks by hand
- if one branch points at another, the target can surface that incoming reference
- the target stays clean because backlinks are not stored as extra tree structure

## In The TUI

You can work with relations in a few ways:

- type a target id, a relation kind, or `backlink` in the command palette
- use `]` to follow outgoing relations
- use `[` to follow backlinks into the current node

If there is only one possible target, the jump happens immediately.

If there are multiple outgoing relations or multiple backlinks, `mdmind` opens a small picker instead of guessing.

## In The CLI

Use:

```bash
mdm relations map.md
mdm relations map.md#product/api-design
mdm validate map.md
```

The deep-linked form is the most useful when you want to inspect both:

- outgoing relations from one node
- incoming backlinks to that same node

`mdm relations --json` and `--plain` include a target kind so agents and humans
can tell same-file ids from path-qualified branch refs, external files, and
URLs.

`mdm validate` warns when:

- a same-file relation target does not match an id in the current map
- a path-qualified target file is missing
- a path-qualified target file exists but the requested branch id is missing or
  ambiguous
- a path-qualified branch points at a non-Markdown file

## When Relations Add Real Value

Good uses:

- a release checklist blocked by a technical branch
- a prompt library supporting a feature branch
- a chapter scene referencing a character and a location
- a quest node pointing at the region and faction it involves

Bad uses:

- replacing clear tree structure with lots of arbitrary links
- linking almost every node to every other node
- adding typed relation kinds that no one will search for or reuse

If everything cross-links everywhere, the map gets harder to read.

## Markdown Compatibility

These tokens live inline in ordinary Markdown text files.

In most Markdown renderers:

- `[id:...]` shows up as literal text
- `[[target]]` shows up as literal text

In wiki-link-aware tools:

- `[[target]]` may also be interpreted as a link

That overlap is usually compatible with the intent rather than destructive.
