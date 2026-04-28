# mdmind Conventions

Use these defaults unless the user’s source material or existing map style clearly suggests something else.

## When mdmind Is The Right Target

Choose `mdmind` when the output needs:

- real hierarchy
- durable branches a human will revisit
- lightweight workflow markers like `#todo` or `#risk`
- structured fields like `@owner:mira` or `@status:active`
- stable deep-link anchors with `[id:...]`
- lateral links between distant branches
- attached notes without turning the whole file into prose

Prefer ordinary Markdown when the user only needs a short one-off answer.

## Good Default Syntax

- visible label first
- `#tag` for grouping or workflow markers
- `@key:value` for repeated structured fields
- `[id:path/to/node]` only on durable branches
- `| detail` lines for real prose
- `[[target/id]]` or `[[rel:kind->target/id]]` only when lateral structure matters

## Recommended Tag And Metadata Defaults

These are defaults, not mandatory schema:

- workflow: `#todo`, `#blocked`, `#done`, `#idea`, `#decision`, `#question`, `#theme`, `#risk`
- metadata: `@status`, `@owner`, `@priority`, `@area`, `@source`, `@region`, `@section`

Prefer 2-5 stable keys over a large taxonomy.

## When To Add Ids

Add ids to:

- the root when the map is durable
- top-level sections
- major workstreams or reusable anchors
- durable themes, chapters, risks, entities, or decisions

Avoid ids on:

- transient leaves
- every checklist item
- branches that are unlikely to be linked or revisited

Good ids are short, stable, and slash-based:

- `product/api-design`
- `research/themes/ownership`
- `story/plot/act-1`

## When To Use Details

Use detail lines for:

- rationale
- source quotes
- meeting context
- scene notes
- research excerpts
- a sentence or two of explanation that belongs to one node

Do not use detail lines for every branch. If most nodes need paragraphs, the map structure is probably wrong or the result should be prose.

## When To Use Relations

Use relations when the tree alone cannot express the meaning cleanly.

Good cases:

- a risk blocks a delivery branch
- a research note supports a thesis branch
- a chapter links to a character, place, or theme
- an action answers an open question

Prefer plain `[[target/id]]` first.
Use typed relations like `[[rel:blocked-by->target/id]]` only when preserving the relation meaning is valuable.

## What Good Map Output Looks Like

- root and major branches are obvious
- labels scan cleanly in plain text
- metadata is repeated enough to support search
- ids are present where deep links will help later
- details hold the prose, not the labels
- relations are sparse and meaningful

## Validation Shortlist

- parses cleanly
- `mdm validate` passes when available
- no duplicate ids
- metadata keys are consistent
- relation targets exist
- labels are readable without the TUI
