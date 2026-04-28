---
name: mdmind-map-authoring
description: Use whenever the user wants source material turned into a native mdmind map or wants an existing mdmind map restructured, cleaned up, or normalized. Trigger for meeting notes, research synthesis, product plans, writing outlines, decision maps, project breakdowns, and similar work where hierarchy, tags, metadata, ids, details, or relations should survive in a durable .md file. Do not use for short prose-only answers, raw transcript cleanup without durable structure, visual-diagram requests, or mdm CLI-first inspection/export tasks.
---

# mdmind Map Authoring

Create native `mdmind` map output that stays useful for a human, not just syntactically valid text.

## Use For

- Turn messy notes into a clean `mdmind` outline.
- Draft or revise `.md` map files used with `mdmind`.
- Convert meeting notes into actions, decisions, risks, and open questions.
- Convert research material into themes, evidence, questions, and follow-up work.
- Convert product inputs into goals, requirements, workstreams, dependencies, and risks.
- Convert writing notes into characters, places, plotlines, themes, chapters, and revisions.
- Improve an existing map’s structure, labels, ids, metadata, details, or relations.

## Do Not Use For

- Short prose summaries that do not need durable hierarchy.
- Raw transcript cleanup where structure is still unknown.
- Purely visual diagram requests.
- CLI-first inspection, query, export, or map-audit tasks that are mainly about using `mdm`.
- Forcing every task into ids, metadata, and relations when a simple outline is enough.

## Defaults

- Build a readable tree first.
- Keep labels short.
- Use a few repeated tags or metadata keys, not a taxonomy.
- Add ids only on durable branches.
- Keep relations sparse.
- Use detail lines only when a branch needs real prose, rationale, quotes, or context.

Read only what you need:

- Read `references/mdmind-conventions.md` when deciding how much structure to add and when to use ids, details, or relations.
- Read `references/example-shapes.md` when the task looks like meeting notes, research synthesis, product planning, writing, decision work, or general planning.

## Outline-First Composition Rules

- Treat the tree as an outline before you treat it as a data model.
- Make each node label a scannable thought, not a paragraph.
- Use the tree for ownership and hierarchy.
- Use detail lines for attached prose.
- Use tags and metadata to help grouping and retrieval, not to replace readable labels.

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
- Use `#tags` for lightweight grouping and workflow markers such as `#todo`, `#risk`, `#decision`, or `#theme`.
- Use `@key:value` metadata for repeated structured fields such as `@status:active`, `@owner:jason`, or `@priority:high`.
- Use `[id:...]` for durable anchors that a human or tool is likely to revisit, deep-link, export, or reference later.
- Use `[[target/id]]` or `[[rel:kind->target/id]]` only when lateral meaning matters across distant branches.

Prefer this escalation order:

1. label
2. label + tag
3. label + repeated metadata
4. durable id
5. relation

Do not jump to ids and relations before the tree shape is already good.

## Workflow

1. Decide whether `mdmind` is the right target.
   If hierarchy, durable branches, lightweight workflow markers, structured fields, deep links, or attached notes matter, use `mdmind`. Otherwise prefer normal Markdown.
2. Sketch the tree before the syntax.
   Find the root and 3-7 major branches first.
3. Write concise labels.
   Each label should still read well in plain text.
4. Add the smallest useful structured layer.
   Start with `#tags` and repeated metadata keys such as `@status`, `@owner`, `@priority`, `@source`, or `@area` only when the source supports them.
5. Add ids only where future navigation or linking is likely.
   Good id targets are top-level sections, major work items, durable themes, core entities, and reusable anchors.
6. Add details only where prose materially helps.
   Use `| detail` lines for rationale, quotes, research excerpts, scene notes, or meeting context that belongs to one node.
7. Add relations only when the connection should survive outside tree placement.
   Prefer plain `[[target/id]]`. Use typed relations like `[[rel:blocked-by->target/id]]` only when the relation meaning matters.
8. Tighten the map.
   Remove over-structuring, inconsistent metadata, duplicate ids, unnecessary relations, and verbose labels.

## Output Shape

- Use one root branch with clear major children.
- Keep sibling sets coherent: sections, workstreams, themes, acts, decisions, questions, actions, or similar.
- Under execution-oriented branches, make leaf nodes concrete enough to act on.
- Under research or writing branches, use details for context instead of turning every label into prose.

Use the reference shapes when the task matches a common pattern:

- Meeting, research, product, writing, decision, or general planning shapes live in `references/example-shapes.md`.

## Map Conventions

- Prefer a small, repeated metadata vocabulary over many one-off keys.
- Prefer labels that imply hierarchy naturally instead of encoding too much into tags.
- Do not put ids on every node.
- Do not create relation-heavy graph output unless the source material strongly requires it.
- If the user already has a map style, preserve it instead of imposing a new schema.

## Validation

Before handing back a map or file:

1. Check that the tree shape is coherent and not over-nested.
2. Check that labels are concise and readable.
3. Check that metadata keys are consistent.
4. Check that ids are sparse, stable, and non-duplicated.
5. Check that relations point to real ids and are worth keeping.
6. If `mdm` is available, run `mdm validate <file>`.
7. If the result is large or deeply linked, use `mdm links <file>` or `mdm relations <file> --plain` as a sanity check.

If the user’s main goal is to inspect, query, validate, or export an existing map rather than author one, use the companion `mdm-cli-inspection` skill instead.

## Example User Prompts

- “Turn these meeting notes into an mdmind map with actions, decisions, and open questions.”
- “Restructure this project map so the workstreams are clearer and ids only appear on durable branches.”
- “Convert this research dump into a map with themes, evidence, and follow-up tasks.”

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
- Not every node needs an id. Overusing ids hurts readability.
- Relations are for lateral meaning, not for replacing basic hierarchy.
- Detail lines should hold actual prose or attached context, not restate the label.
- A map should still be understandable in plain Markdown without the TUI.

## Return Format

- If the user asked for a file edit, write native `.md` map syntax directly into the file.
- If the user asked for map output in chat, return plain `mdmind` syntax unless they explicitly want fences or explanation.
- Keep commentary outside the map brief; the map itself should carry the structure.
