# Design

This document defines the CLI shape, command rationale, and key user flows for the app

The redesign is guided by:
- clig.dev principles (https://clig.dev/)
- composable output
- local-first files
- excellent defaults
- human-first errors
- clear separation of CLI vs TUI responsibilities

## 1. Product surfaces

### `mdm`
The command-line interface.

Use it for:
- quick inspection
- search
- filtering
- stats
- validation
- exports
- initializing files from templates
- opening a file or deep link

### `mdmind`
The full-screen terminal UI.

Use it for:
- immersive navigation
- folding
- editing larger trees
- focused exploration

## 2. Command design principles

- Subcommands, not feature flags
- Pretty output by default
- `--json` for machine-readable output
- `--plain` where line-oriented output is useful
- stdout contains command data
- stderr contains diagnostics, warnings, and errors
- exit code `0` = success
- exit code `1` = runtime / user error
- exit code `2` = invalid usage

## 3. Primary commands

## `mdm open`

Purpose:
- Open a file, optionally with a deep link

Examples:

```bash
mdm open ideas.md
mdm open ideas.md#product/api-design
```

Rationale:
- Explicit command improves discoverability
- Can later dispatch to quick mode or TUI depending on flags

Notes:
- A shortcut form may also be supported:
  `mdm ideas.md#product/api-design`

---
## `mdm view`

Purpose:
- Show a tree view of the map in the terminal without entering interactive mode

Examples:

```bash
mdm view ideas.md
mdm view ideas.md#product
mdm view ideas.md --max-depth 2
mdm view ideas.md --json
```

Rationale:
- The fastest way to inspect structure
- Good for README-style previews and shell usage
- Better than forcing users into interactive mode just to inspect a map

Expected default output:
- ASCII tree with indentation and box drawing characters

---
## `mdm find`

Purpose:
- Search across node labels, tags, ids, and metadata

Examples:

```bash
mdm find ideas.md "rate limit"
mdm find ideas.md "#todo"
mdm find ideas.md "@status:blocked"
mdm find ideas.md "api-design"
mdm find ideas.md "#prompt" --json
```

Rationale:
- Search is a core workflow
- One flexible command is better DX than forcing separate micro-commands for every query type

Behavior:
- Should return matching nodes with enough context to identify location
- May include node id and breadcrumb path in output

---
## `mdm tags`

Purpose:
- Inspect tags in a file

Examples:

```bash
mdm tags ideas.md
mdm tags ideas.md --json
mdm tags ideas.md --plain
```

Rationale:
- Tags are cross-cutting structure
- Users want both visibility and metrics
- Keeps tag operations discoverable and separate from general search

Default output ideas:
- table of tag counts
- optionally sorted descending by count

Future subcommands:
- `mdm tags list`
- `mdm tags stats`
- `mdm tags rename`

MVP preference:
- start with a simple `tags` command that prints counts

---
## `mdm kv`

Purpose:
- Inspect key:value metadata from nodes

Examples:

```bash
mdm kv ideas.md
mdm kv ideas.md --json
mdm kv ideas.md --plain
mdm kv ideas.md --keys status,owner
```

Rationale:
- Metadata drives TODO views, ownership, priority, and prompt refinement
- Needs to be easy to pipe into shell tools

Default output ideas:
- table by node:
  path | key | value
- or grouped node detail view if more readable

Future options:
- `--by-node`
- `--by-key`
- `--flat`

---
## `mdm links`

Purpose:
- List all node ids and deep-linkable references

Examples:

```bash
mdm links ideas.md
mdm links ideas.md --json
mdm links ideas.md --plain
```

Rationale:
- Deep links are a key differentiator
- Users need a way to discover available anchors

Default output:
- id
- node label
- breadcrumb path

---
## `mdm validate`

Purpose:
- Check file integrity and structural correctness

Examples:

```bash
mdm validate ideas.md
mdm validate ideas.md --json
```

Rationale:
- Needed for confidence, automation, and future CI use
- Helps explain parser issues without entering interactive mode

Validation checks:
- malformed structure
- duplicate ids
- invalid metadata tokens
- invalid indentation patterns

---
## `mdm export`

Purpose:
- Export normalized representations

Examples:

```bash
mdm export ideas.md --format json
mdm export ideas.md --format json > ideas.json
```

Rationale:
- Makes the tool interoperable with scripts, AI workflows, and future integrations

MVP:
- JSON export first

Later:
- markdown normalize
- mermaid
- opml

---
## `mdm init`

Purpose:
- Create a new map from a template

Examples:

```bash
mdm init feature.md --template feature
mdm init prompt-library.md --template prompts
mdm init product-idea.md --template product
```

Rationale:
- Templates make the tool immediately useful
- Strong fit for idea exploration and requirements definition

---
## `mdm help`

Purpose:
- Show command usage and examples

Rationale:
- Help text is part of the product
- Every command should include examples

## 4. Main user flows

## Flow A — idea exploration

Goal:
- Explore a new concept quickly and shape it into structure

Example:

```bash
mdm init ai-feature.md --template product
mdm view ai-feature.md
mdm find ai-feature.md "#idea"
```

Why this matters:
- This is the core “thinking tool” workflow
- It should feel faster than starting a document

---
## Flow B — requirements definition

Goal:
- Turn a vague feature idea into scoped requirements

Example:

```bash
mdm init feature-auth.md --template feature
mdm view feature-auth.md
mdm kv feature-auth.md --keys status,owner
```

Why this matters:
- The map becomes the pre-spec structure
- Easy to track status and ownership without losing context

---
## Flow C — TODO and status tracking

Goal:
- Use nodes and metadata as a lightweight structured backlog

Example:

```bash
mdm find backlog.md "@status:todo"
mdm kv backlog.md --plain | grep high
mdm tags backlog.md
```

Why this matters:
- Makes metadata operational, not decorative

---
## Flow D — prompt refinement

Goal:
- Build and organize prompt experiments and refinements

Example:

```bash
mdm init prompts.md --template prompts
mdm find prompts.md "#prompt"
mdm open prompts.md#prompts/system-auth
```

Why this matters:
- Good fit for AI-native workflows
- Nodes become reusable units of prompt context

---
## Flow E — deep-link recall

Goal:
- Re-open exactly the node you care about

Example:

```bash
mdm open ideas.md#product/api-design
mdm links ideas.md
```

Why this matters:
- This is one of the strongest differentiators versus plain markdown

## 5. Error handling philosophy

Errors should:
- be explicit
- explain the problem
- suggest the next action

Good:
- `File not found: ideas.md`
- `Duplicate id "product/api-design" found in ideas.md`
- `Could not resolve deep link "product/auth". Run "mdm links ideas.md" to inspect available ids.`

Bad:
- stack traces by default
- internal exception names
- silent failures

## 6. Output modes

### Pretty
Default human-readable output.

### JSON
Structured output for automation.

### Plain
Minimal line-oriented output for shell tools.

## 7. Quiet / verbosity

Planned:
- `--quiet` suppresses non-essential stderr chatter
- errors still go to stderr

## 8. Color behavior

Planned:
- color on TTY by default
- respect `NO_COLOR`
- support `--no-color`

## 9. TUI boundary

The TUI should not be required for:
- search
- stats
- validation
- export
- structure inspection

It exists for:
- immersive editing
- navigation
- larger-map interaction

That keeps the CLI useful even before the TUI is fully mature.
