# mdmind Format 1

`mdmind` is a Markdown-compatible profile for tree-structured mind maps and outlines.

The goal is simple: a mind map or outline should stay readable in ordinary Markdown tools, editable in any text editor, and richer when opened with `mdmind`.

This document defines format version 1.

## Normative Words

The words `must`, `must not`, `should`, and `may` are used intentionally.

- `must` means required for a conforming format 1 document.
- `must not` means forbidden in a conforming format 1 document.
- `should` means strongly recommended for durable, portable files.
- `may` means allowed, but not required.

## Design Goals

- use `.md` files by default
- keep the source readable as plain text
- make the tree the primary structure
- treat mind maps and outlines as first-class surfaces over the same file
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

It is a narrow profile for structured thinking surfaces where the same tree can be read as an outline or explored as a mind map.

## File Extension

Normal mdmind files should use `.md`.

A future tool may choose to recognize `.mdmind`, but format 1 does not require a custom extension. The durable source should remain useful when viewed on GitHub, in a text editor, or in a normal Markdown preview.

## Document Shape

An mdmind document is a Markdown unordered-list profile.

Each non-blank source line must be one of:

- a node line
- a detail line

Blank lines are allowed and ignored by the parser.

Other Markdown block forms, such as headings, fenced code blocks, tables, block quotes, and standalone paragraphs, are outside format 1. They may be good Markdown, but they are not valid mdmind format 1 content.

Example:

```text
- Product Plan #project [id:product]
  - Current Work #todo @status:active [id:product/current]
  - Notes
    | Keep this branch short in the visible tree.
```

## Line Grammar

This is the shape of the format, not a full parser grammar:

```text
document    = *( blank-line / node-line / detail-line )
blank-line  = whitespace-only line
node-line   = indent "- " node-content
detail-line = indent "|" [ " " detail-text ]
indent      = zero or more spaces, in multiples of two
```

Tabs must not appear on non-blank lines.

Node content is split into whitespace-delimited tokens. A token is interpreted by its prefix:

- `#` starts a tag token
- `@` starts a metadata token
- `[id:` starts an id token
- `[` or `![` plus `](` starts an external reference token
- `[[` starts a relation token
- any other token is visible label text

There is no escaping in format 1. If visible prose needs to begin with one of the annotation prefixes, reword the label or put the prose in details.

## Indentation

Indentation defines hierarchy.

Rules:

- indentation must use spaces only
- one tree level is two spaces
- indentation must be a multiple of two spaces
- indentation must not jump by more than one level at a time
- the first node must start at level 0
- one document may contain more than one level 0 node

Example:

```text
- Root
  - Child
    - Grandchild
  - Sibling
- Another Root
```

## Nodes

A node line begins with `- ` after indentation.

The rest of the line is the node content:

```text
- API Design #backend @status:active [id:product/api] [[rel:supports->product/mvp]]
```

The visible label is built from the tokens that are not parsed as annotations.

Rules:

- every node must have visible text after annotations are removed
- annotation tokens must be separated by whitespace, except Markdown reference labels and targets may contain spaces inside `[label](target)` or `![label](target)`
- annotation token values must not contain spaces except for Markdown reference labels and targets
- annotation tokens may appear anywhere in the node content
- the preferred order is label, tags, metadata, id, references, relations

Preferred:

```text
- API Design #backend @status:active [id:product/api]
```

Allowed but harder to scan:

```text
- #backend [id:product/api] API Design @status:active
```

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

- a tag token must start with `#`
- a tag token must include at least one character after `#`
- multiple tags are allowed on one node
- tags are stored with the leading `#`
- tools may search tags case-insensitively
- tools should preserve source spelling when displaying tags

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

- a metadata token must start with `@`
- the key and value are separated by the first `:`
- key and value must both be present
- keys should be lowercase
- values must not contain spaces
- repeated keys are allowed

Repeated keys are legal, but mind maps and outlines are easier to query when one key has one value per node.

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
- an id token must start with `[id:` and end with `]`
- an id value must not be empty
- an id value must not contain whitespace
- ids must be unique inside one file
- ids should be stable across edits
- ids should be path-like, lowercase, and readable

Ids are used for deep links, relations, exports, and CLI jumps.

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

- a detail line must begin with `|` after indentation
- a detail line must be indented exactly one level deeper than its owning node
- detail lines must immediately follow their owning node or another detail line for that same node
- detail lines must come before child nodes
- a single optional space after `|` is removed
- a bare `|` stores a blank detail line
- additional spaces after `| ` are preserved as detail text

This is valid:

```text
- Node
  | Detail line one.
  | Detail line two.
  - Child
```

This is invalid because the detail appears after a child:

```text
- Node
  - Child
  | Too late.
```

Use details for rationale, meeting notes, quotes, research context, or explanation that belongs to one branch.

## External References

External references attach supporting local files, URLs, and images to a node while staying readable as ordinary Markdown links.

File or URL reference:

```text
[label](path-or-url)
```

Image reference:

```text
![label](path-or-url)
```

Example:

```text
- Research Packet [brief note](docs/project brief.md) ![flow diagram](assets/flow chart.png)
```

Rules:

- labels and targets must not be empty
- labels and targets may contain spaces
- local targets are resolved relative to the map file for validation
- missing local targets are validation warnings
- URLs are preserved as references but are not fetched during validation
- references are not embedded; binary content stays outside the map
- first-class local picking may browse up to the filesystem root, should mark detected git roots as landmarks, and should store generated paths as portable references relative to the map file's directory
- first-class reference review should preview local `.txt` and `.md` content inline, provide lightweight web-link and PNG previews, show "No preview available" for unsupported types, and keep external open available for every reference

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

- a relation token must start with `[[` and end with `]]`
- plain relations have only a target id
- typed relations must use `rel:kind->target`
- kind and target must both be present in typed relations
- relation kinds must not contain spaces
- relation targets must not contain spaces
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

Warnings do not make a document structurally invalid, but they do mean the file is less portable or less complete.

## Serialization

Tools that rewrite mdmind files should prefer stable, boring output.

Recommended order for a node line:

```text
label #tags @metadata [id:...] [reference](path) [[relations]]
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

They should avoid churn that does not change the tree.

## Compatibility

An mdmind file should render as a normal Markdown list.

Normal Markdown tools will usually show mdmind annotations as literal text. That is acceptable. The file should still be readable even when the richer structure is ignored.

Because mdmind format 1 is a profile, not all Markdown is valid mdmind. A normal Markdown document with headings and paragraphs may be good Markdown but not a valid mdmind mind map or outline.

## Sidecar Files

Sidecar files are not part of the mdmind document format.

Tools may store UI state, session state, locations, checkpoints, navigation memory, or preferences next to a map. Those files must not be required to parse the `.md` source.

The `.md` file is the durable source.

## Versioning

Format 1 is the current stable contract.

Future format versions should:

- preserve readable `.md` files
- remain compatible with existing format 1 files when practical
- add syntax only when the gain is worth the extra surface area
- prefer optional conventions over required ceremony

If a future version needs explicit file-level metadata, it should be added in a way that does not break the plain Markdown reading experience.

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
