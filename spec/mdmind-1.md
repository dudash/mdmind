# mdmind Format 1

`mdmind` is a CommonMark-friendly Markdown profile for tree-structured mind maps and outlines.

The goal is simple: a mind map or outline should stay readable in ordinary Markdown tools, editable in any text editor, and richer when opened with `mdmind`.

This document defines format version 1.

## Design Goals

- use `.md` files by default
- keep the source readable as plain text
- make the tree the primary structure
- keep annotations small and inline
- support durable links without requiring every node to have an id
- let richer tools exist without trapping the file inside one app

## Non-Goals

`mdmind` format 1 is not:

- a complete Markdown replacement
- a general-purpose Markdown parser
- a wiki syntax
- a graph database
- a rich document format

It is a narrow profile for structured thinking surfaces where mind maps and outlines are both first-class.

## File Extension

Use `.md` for normal mdmind mind maps and outlines.

The file should remain useful when viewed on GitHub, in a text editor, or in a normal Markdown preview. A future tool may choose to recognize `.mdmind`, but format 1 does not require it.

## Document Shape

An mdmind document is a Markdown unordered list.

Each non-empty source line is one of:

- a node line beginning with `- `
- a detail line beginning with `|`

Blank lines are allowed and ignored by the parser.

Other Markdown block forms, such as headings, fenced code blocks, tables, and block quotes, are not part of mdmind format 1.

Example:

```text
- Product Plan #project [id:product]
  - Current Work #todo @status:active [id:product/current]
  - Notes
    | Keep this branch short in the visible tree.
```

## Indentation

Indentation defines hierarchy.

Rules:

- indentation uses spaces only
- one tree level is two spaces
- tabs are not valid indentation
- indentation must not jump by more than one level at a time
- the first node starts at level 0

Example:

```text
- Root
  - Child
    - Grandchild
  - Sibling
```

## Nodes

A node line begins with `- ` and then contains a visible label plus optional annotation tokens.

```text
- API Design #backend @status:active [id:product/api] [[rel:supports->product/mvp]]
```

The visible label is the line content after annotation tokens are removed.

Rules:

- every node should have visible text
- annotation tokens are separated by whitespace
- annotation tokens may appear after, before, or between label words, but the preferred style is label first, annotations after
- annotation token values do not contain spaces in format 1

Preferred:

```text
- API Design #backend @status:active [id:product/api]
```

Avoid:

```text
- #backend [id:product/api] API Design @status:active
```

The second form is parseable, but harder to scan.

## Tags

Tags mark topics, types, or workflow states.

Syntax:

```text
#tag
```

Examples:

```text
- MVP Scope #todo
- Customer Quotes #research
- Launch Risk #blocked
```

Rules:

- a tag token starts with `#`
- a tag must include at least one character after `#`
- multiple tags are allowed
- tags are stored with the leading `#`
- tools may search tags case-insensitively, but should preserve source spelling when displaying them

Use tags for grouping. Use metadata when the value matters.

## Metadata

Metadata adds structured key-value fields to a node.

Syntax:

```text
@key:value
```

Examples:

```text
- Draft Release Note #todo @owner:jason @status:active
- Glass Marsh Quest #quest @region:glass-marsh @status:todo
```

Rules:

- a metadata token starts with `@`
- the key and value are separated by the first `:`
- key and value must both be present
- keys should be lowercase
- values do not contain spaces in format 1
- repeated keys are allowed, but maps and outlines are easier to query when one key has one value per node

Good common keys:

- `@status:active`
- `@owner:jason`
- `@priority:high`
- `@source:interview`
- `@region:glass-marsh`

## Ids

Ids give durable addresses to important nodes.

Syntax:

```text
[id:path/to/node]
```

Example:

```text
- API Design #backend [id:product/api-design]
```

Rules:

- a node may have zero or one id
- ids must be unique inside one file
- ids should be stable across edits
- ids should be path-like, lowercase, and readable
- ids are used for deep links, relations, exports, and CLI jumps

Not every node needs an id. Add ids to branches you expect to revisit, cite, export, or link to.

Deep-link form:

```text
roadmap.md#product/api-design
```

The fragment resolves against node ids.

## Details

Details attach longer prose to one node without making the visible tree noisy.

Syntax:

```text
- API Design [id:product/api]
  | We need one stable auth flow before launch.
  |
  | Open question: should refresh tokens be scoped by workspace?
  - Token Lifecycle
```

Rules:

- detail lines begin with `|`
- a single optional space after `|` is removed
- a bare `|` creates a blank detail line
- detail lines must appear directly under the node they belong to
- detail lines come before child nodes
- details are attached prose, not child structure

Use details for rationale, meeting notes, quotes, research context, or explanation that belongs to one branch.

## Relations

Relations connect nodes across the tree without changing parent-child structure.

Plain reference:

```text
[[target/id]]
```

Typed relation:

```text
[[rel:kind->target/id]]
```

Example:

```text
- MVP Scope #todo [id:product/mvp] [[rel:supports->product/requirements]]
- Requirements #spec [id:product/requirements]
```

Rules:

- relation tokens use double brackets
- plain relations have only a target id
- typed relations use `rel:kind->target`
- kind and target must both be present in typed relations
- relation targets should point at ids in the same file
- unresolved relation targets are validation warnings
- backlinks are derived from incoming relations and are not stored in the file

Relations should be used when the connection is meaningful. They are not a substitute for putting the main structure in the tree.

## Validation

A valid mdmind format 1 document:

- uses only node lines, detail lines, and blank lines
- uses spaces, not tabs
- indents in multiples of two spaces
- starts with a level 0 node
- does not skip indentation levels
- gives each node visible text
- uses valid annotation tokens
- does not repeat ids inside the same file

Validation warnings may include:

- metadata keys that are not lowercase
- relations that point to missing ids

## Serialization

Tools that rewrite mdmind files should prefer stable, boring output.

Recommended order for a node line:

```text
label #tags @metadata [id:...] [[relations]]
```

Recommended detail placement:

```text
- Node label [id:node]
  | Detail line one.
  | Detail line two.
  - Child node
```

Serializers should preserve:

- hierarchy
- visible labels
- detail text
- tags
- metadata
- ids
- relations

They should avoid churn that does not change the map.

## Compatibility

An mdmind file should render as a normal Markdown list.

Normal Markdown tools will usually show mdmind annotations as literal text. That is acceptable. The file should still be readable even when the richer structure is ignored.

Because mdmind format 1 is a profile, not all Markdown is valid mdmind. A normal Markdown document with headings and paragraphs may be good Markdown but not a valid mdmind mind map or outline.

## Sidecar Files

Sidecar files are not part of the mdmind document format.

Tools may store UI state, session state, locations, checkpoints, or preferences next to a map, but those files must not be required to parse the map itself.

The `.md` file is the durable source.

## Versioning

Format 1 is the current stable contract.

Future format versions should:

- preserve readable `.md` files
- remain compatible with existing format 1 maps when practical
- add syntax only when the gain is worth the extra surface area
- prefer optional conventions over required ceremony

If a future version needs explicit file-level metadata, that should be added in a way that does not break the plain Markdown reading experience.

## Complete Example

```text
- Onboarding Research #project @status:active [id:onboarding]
  | Turn scattered notes, interviews, and generated research into a decision map.
  - Core Question #question [id:onboarding/question]
    - Where do new users lose momentum first?
  - Evidence #source [id:onboarding/evidence] [[rel:informs->onboarding/decision]]
    - Interview notes mention setup vocabulary friction
    - Support tickets cluster around first-map examples
  - Decision #decision @owner:jason [id:onboarding/decision]
    - Ship a guided starter map before adding more settings
  - Follow-ups #todo @status:active [id:onboarding/follow-ups] [[onboarding/evidence]]
    - Review five more sessions
    - Draft release note
```
