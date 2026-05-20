---
name: mdmind-map-authoring
description: "Create or revise native mdmind maps: plain-text Markdown outlines, mind maps, knowledge maps, and structured notes that stay useful as durable .md files. Use for turning meeting notes, research, product plans, writing outlines, decision maps, project breakdowns, or agent memory into hierarchy with optional tags, metadata, ids, details, relations, and external Markdown references. Do not use for short prose-only answers, raw transcript cleanup without durable structure, visual-diagram requests, or mdm CLI-first inspection/export tasks."
license: Apache-2.0
metadata:
  author: mdmind
  version: "0.7.0"
---

# mdmind Map Authoring

Create native `mdmind` map output that stays useful for a human, not just syntactically valid text.

## Use For

- Turn messy source material into a clean `mdmind` outline.
- Draft or revise `.md` map files used with `mdmind`.
- Convert notes, plans, research, writing, or strategy material into a durable map without losing the user's intended framing.
- Create decomposable TODO maps for local execution, agent handoff, and persistent project memory.
- Improve an existing map’s structure, labels, ids, metadata, details, or relations.
- Attach supporting local files, URLs, or image artifacts to relevant nodes with normal Markdown references.

## Do Not Use For

- Short prose summaries that do not need durable hierarchy.
- Raw transcript cleanup where structure is still unknown.
- Purely visual diagram requests.
- CLI-first inspection, query, export, or map-audit tasks that are mainly about using `mdm`.
- Forcing every task into ids, metadata, and relations when a simple outline is enough.

## Defaults

- Build a readable tree first.
- Keep labels short.
- Use tags, metadata, ids, details, and relations only when they add clear value.
- Add ids only on durable branches.
- Keep relations sparse.
- Use detail lines only when a branch needs real prose, rationale, quotes, or context.
- Use normal Markdown links or images for external source artifacts; keep the artifact external instead of pasting bulky content into the map.
- Preserve the user's framing; improve the structure, not the underlying taxonomy, unless the user asks for a new framework.

Read only what you need:

- Read `references/mdmind-conventions.md` when deciding how much structure to add and when to use ids, details, or relations.
- Read `references/example-shapes.md` when the task matches a common domain or when you want optional decomposition examples without imposing a canned content model.

## Outline-First Composition Rules

- Treat the tree as an outline before you treat it as a data model.
- Make each node label a scannable thought, not a paragraph.
- Use the tree for ownership and hierarchy.
- Use detail lines for attached prose.
- Use tags and metadata to help grouping and retrieval, not to replace readable labels.
- Use Markdown references for supporting files or URLs that should stay attached to a node.

Good node labels usually work as:

- a section heading
- a work item
- a theme
- a question
- a decision
- a compact entity name with a small amount of state

Bad node labels usually try to do too much at once:

- several sentences in one label
- long rationale mixed into the main tree
- metadata-like content embedded as prose instead of repeated fields

## When To Move Text Into Details

Move content into `| detail` lines when it is useful context but no longer scans well as an outline row.

Use detail lines for:

- rationale
- quotes
- research excerpts
- meeting context
- scene notes
- a sentence or two of explanation attached to one branch

Keep content in the main label when it works as:

- a concise heading
- a short actionable item
- a short named concept
- a compact status-bearing branch

Rule of thumb:

- if a human should skim it quickly in the tree, keep it in the label
- if a human should read it more slowly for nuance, move it to details

## Choosing Structure Deliberately

Choose the lightest structure that preserves meaning:

- Use plain labels for core outline shape.
- Use `#tags` when lightweight grouping or workflow marking helps.
- Use `@key:value` metadata when repeated structured fields help filtering or clarity.
- Use `[id:...]` for durable anchors that a human or tool is likely to revisit, deep-link, export, or reference later.
- Use `[[target/id]]` or `[[rel:kind->target/id]]` only when lateral meaning matters across distant branches.
- Use normal Markdown links or images when a node should point to an external file, URL, or image.

Prefer this escalation order:

1. label
2. label + tag
3. label + repeated metadata
4. durable id
5. relation
6. external reference

Do not jump to ids and relations before the tree shape is already good.

## Preserve User Framing

- Default to the user's own categories, vocabulary, and decomposition.
- Improve readability and structure without replacing the content model with your own preferred framework.
- If the source already implies strong sections, preserve them.
- If the source is messy, infer the smallest useful structure rather than inventing a full taxonomy.
- Only introduce a new organizational framework when the user explicitly asks for one.

## Feature Guide

### Node Labels

- Keep node labels short and readable.
- Use labels for the main outline shape: headings, work items, themes, questions, decisions, and compact named concepts.
- If the label stops scanning cleanly as one outline row, shorten it and move the nuance elsewhere.

### `#tags`

- Use `#tags` only for meaningful grouping or workflow marking.
- Tags are good when they help cluster related branches or make search and filtering easier.
- Do not add tags to every node by default.

### `@key:value` Metadata

- Use a few stable metadata keys when repeated structured fields help.
- Good metadata is small, repeated, and easy to filter later.
- Do not invent lots of one-off keys unless the task clearly needs them.

### `[id:...]`

- Add `[id:...]` only on durable branches.
- Use ids when a branch is likely to be revisited, deep-linked, exported, or referenced by another branch.
- Do not put ids on every node.

### `| detail` Lines

- Use `| detail` lines only when a branch needs real prose.
- Good uses are rationale, quotes, meeting context, research excerpts, scene notes, and attached explanation.
- Keep the main label compact and move slower-reading context into details.

### `[[target]]` And `[[rel:kind->target]]`

- Use `[[target]]` or `[[rel:kind->target]]` only when the cross-link is worth preserving.
- Use plain cross-links when one branch should point to another.
- Use typed cross-links when the relationship itself matters.
- Prefer tree structure first and relations second.

### External Markdown References

- Use normal Markdown links for supporting files, web pages, and source artifacts.
- Use normal Markdown image references for images that belong with the node.
- Prefer paths relative to the map file when the artifact lives in the same project tree.
- Labels and targets may contain spaces; keep them as normal Markdown link text and destinations.
- Do not use external refs as a replacement for ids or relations: refs point outside the map, while ids and relations structure the map itself.

## Workflow

1. Decide whether `mdmind` is the right target.
   If hierarchy, durable branches, lightweight workflow markers, structured fields, deep links, or attached notes matter, use `mdmind`. Otherwise prefer normal Markdown.
2. Sketch the tree before the syntax.
   Find the root and 3-7 major branches first.
3. Write concise labels.
   Each label should still read well in plain text.
4. Add the smallest useful structured layer.
   Use `#tags`, `@key:value` metadata, ids, details, and relations only when they make the map easier to navigate, filter, or understand.
5. Add ids only where future navigation or linking is likely.
   Good id targets are top-level sections, major work items, durable themes, core entities, and reusable anchors.
6. Add details only where prose materially helps.
   Use `| detail` lines for rationale, quotes, research excerpts, scene notes, or meeting context that belongs to one node.
7. Add relations only when the connection should survive outside tree placement.
   Prefer plain `[[target/id]]`. Use typed relations like `[[rel:blocked-by->target/id]]` only when the relation meaning matters.
8. Add external refs only where the source artifact should stay reachable from the node.
   Prefer map-relative Markdown links for local files, normal URLs for web sources, and image refs for visual artifacts.
9. Tighten the map.
   Remove over-structuring, inconsistent metadata, duplicate ids, unnecessary relations, and verbose labels.

## Output Shape

- Use one root branch with clear major children.
- Keep sibling sets coherent: major sections, lenses, entities, questions, actions, stages, or similar.
- Under execution-oriented branches, make leaf nodes concrete enough to act on.
- Under research or writing branches, use details for context instead of turning every label into prose.
- For broad strategy prompts, prefer a few clear lenses over a flat brainstorm.
- Derive the actual top-level branches from the user's framing and source material instead of forcing a canned taxonomy.

Use the reference shapes when the task matches a common pattern:

- Common decomposition patterns and optional domain examples live in `references/example-shapes.md`.

## TODO Map Guidance

Use a TODO map shape when the user wants local task decomposition, agent memory, or a durable handoff file that should stay useful across sessions.

Prefer this structure:

- a short root such as `Project TODO Map #todo-map @status:active [id:todo]`
- `Current Focus`, `Backlog`, `Blocked`, `Decisions`, `Handoff Notes`, and `Done Log`
- concrete leaf tasks marked with `#todo` and `@status:todo|active|blocked|done`
- sparse metadata such as `@owner`, `@priority`, and `@area` only where it helps filtering
- detail lines for acceptance criteria, blockers, or handoff context
- ids on durable branches and major tasks, not every checklist item

Use familiar checkbox markers like `[ ]` and `[x]` when task state should round-trip as plain Markdown. Keep `#todo`, `#done`, and `@status` metadata present enough for `task:open`, `task:blocked`, `task:done`, `mdm find`, and `mdm kv` workflows.

Good TODO map commands to include when useful:

```bash
mdm find TODO.md "#todo @status:active" --plain
mdm find TODO.md "@status:blocked" --plain
mdm kv TODO.md --keys status,owner,priority,area --plain
mdm validate TODO.md
```

## Map Conventions

- Prefer only as much structure as the map needs.
- If you add tags or metadata, keep them small and internally consistent.
- Prefer labels that imply hierarchy naturally instead of encoding too much into tags.
- Do not put ids on every node.
- Do not create relation-heavy graph output unless the source material strongly requires it.
- If the user already has a map style, preserve it instead of imposing a new schema.
- Do not inject a favorite taxonomy just because it is broadly sensible.

## Validation

Before handing back a map or file:

1. Check that the tree shape is coherent and not over-nested.
2. Check that labels are concise and readable.
3. Check that metadata keys are consistent.
4. Check that ids are sparse, stable, and non-duplicated.
5. Check that relations point to real ids and are worth keeping.
6. Check that external refs use normal Markdown link syntax and local refs are intended relative to the map file.
7. If `mdm` is available, run `mdm validate <file>`.
8. If the result is large or deeply linked, use `mdm links <file>` or `mdm relations <file> --plain` as a sanity check.
9. If external refs matter, use `mdm refs <file> --plain` as a sanity check.

If `mdm` is not available, still do the manual checks and say that CLI validation was not run.

If the user’s main goal is to inspect, query, validate, or export an existing map rather than author one, use the companion `mdm-cli-inspection` skill instead.

## Example User Prompts

- “Turn these meeting notes into an mdmind map with actions, decisions, and open questions.”
- “Restructure this project map so the workstreams are clearer and ids only appear on durable branches.”
- “Convert this research dump into a map with themes, evidence, and follow-up tasks.”
- “Brainstorm what it would take to turn this into a widely used product, then turn it into a native mdmind map with short labels, details for rationale, and ids only on durable branches.”

## Example Rewrite Pattern

Prefer this:

```md
- Launch Readiness #risk @status:blocked [id:launch/readiness]
  | Vendor approval is still missing for the final asset bundle and the launch cannot proceed until that dependency clears.
```

Over this:

```md
- Launch readiness is blocked because vendor approval is still missing for the final asset bundle and that dependency must clear before launch can proceed #risk @status:blocked
```

The first version keeps the outline scannable, preserves the key state in the label, and moves the nuance into details.

## Gotchas

- Agents often over-structure. If unsure, remove structure rather than add more.
- Agents also over-impose frameworks. If unsure, preserve the user's framing instead of introducing your own taxonomy.
- Not every node needs an id. Overusing ids hurts readability.
- Relations are for lateral meaning, not for replacing basic hierarchy.
- External refs are for local files, URLs, and images; do not invent custom metadata for attachments when Markdown links will do.
- Detail lines should hold actual prose or attached context, not restate the label.
- A map should still be understandable as plain Markdown without relying on interactive rendering.

## Return Format

- If the user asked for a file edit, write native `.md` map syntax directly into the file.
- If the user asked for map output in chat, return plain `mdmind` syntax unless they explicitly want fences or explanation.
- Keep commentary outside the map brief; the map itself should carry the structure.
