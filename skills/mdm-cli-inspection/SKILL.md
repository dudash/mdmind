---
name: mdm-cli-inspection
description: Use whenever the user wants to inspect, validate, query, audit, deep-link, or export mdmind map files with the mdm CLI. Trigger for requests about tags, metadata, ids, relations, subtree inspection, filtered search, plain/json output, or export formats like json, mermaid, and opml. Do not use for drafting new map content from source material when the main task is authoring structure.
license: Apache-2.0
metadata:
  author: mdmind
  version: "0.4.0"
---

# mdm CLI Inspection

Use `mdm` as the read, query, validate, and export surface for existing map files.

## Use For

- Validate a map file before handoff.
- Inspect a map without opening the full TUI.
- Answer questions about tags, metadata, ids, relations, or branches in a map.
- Find items with text, `#tags`, or `@key:value` filters.
- Audit structured fields like owner, status, region, priority, or source.
- List deep-link ids.
- Check incoming and outgoing relations.
- Export a full tree, a subtree, or a filtered working set.

## Do Not Use For

- Drafting a new map from notes when the main work is authoring structure.
- Rewriting large parts of a map unless the user is explicitly asking for file edits.
- Purely visual diagram design.

If the task is mainly about creating or reshaping map content, use `mdmind-map-authoring` instead.

## Defaults

- Start with the smallest command that answers the question.
- Prefer `--plain` for grep-friendly inspection and quick human scanning.
- Prefer deep links like `map.md#product/tasks` when the user cares about one branch.
- Prefer `--query` on export when the user wants a filtered machine-readable subset.
- Check that `mdm` is available before relying on CLI output.

Read only what you need:

- Read `references/command-patterns.md` when choosing the right mdm subcommand.
- Read `references/query-and-export.md` when query syntax, deep links, or export scope matter.

## Requirement

This skill expects `mdm` on `PATH`.

If availability is unknown, run:

```bash
command -v mdm
```

If `mdm` is missing, do not invent command results. Tell the user to install `mdmind` first or use the map-authoring skill for syntax-only guidance.

## Command Selection

- Use `mdm validate <file>` for parser, id, and relation checks.
- Use `mdm view <file>` for a readable tree.
- Use `mdm find <file> "<query>"` for text, tags, and metadata search.
- Use `mdm kv <file> --keys key1,key2` to audit metadata fields.
- Use `mdm tags <file>` to summarize vocabulary.
- Use `mdm links <file>` to list deep-linkable ids.
- Use `mdm relations <file>` or `mdm relations <file>#id` to inspect graph edges.
- Use `mdm export <file> --format json|mermaid|opml` for downstream tools.

## Workflow

1. Clarify whether the user wants validation, inspection, query results, deep links, relations, or export.
2. Choose the narrowest `mdm` command that answers that need.
3. If one branch matters, use a deep link target instead of the whole file.
4. If the user is exploring, prefer plain output and small, focused queries.
5. If the user needs a machine-consumable result, prefer `--json` or `export --format json`.
6. If the map looks malformed or generated, run `mdm validate` before drawing conclusions.
7. Summarize the important result, not just the fact that a command ran.

## Output Conventions

- Quote exact commands when giving instructions.
- If you run commands for the user, report the meaningful result in prose.
- When sharing example commands, keep them copyable and minimal.
- When a query returns too much, narrow it with tags, metadata, or a deep link.

## Validation

Before handing back a result:

1. Check that the command chosen matches the user’s actual question.
2. Check that query syntax is valid and not empty.
3. Check that deep links point to real ids or known label paths.
4. Check that export format matches the downstream use case.
5. If using `relations`, be explicit whether the result is whole-map outgoing relations or node-focused incoming plus outgoing relations.

## Example User Prompts

- “Validate this mdmind file and tell me if the ids and relations are clean.”
- “Find all blocked launch items owned by jason in this map.”
- “Export only the active todo branches from this map as JSON.”

## Gotchas

- `find` is for matching content; `links` is for discovering stable ids.
- `relations file.md` and `relations file.md#id` answer different questions.
- `export --query` can return an empty tree if the filter matches nothing.
- `--plain` and `--json` are mutually exclusive on commands that support both.
- If a file has parser errors, some higher-level conclusions are unreliable until `validate` is addressed.

## Return Format

- For user questions, summarize the answer and include the exact command when useful.
- For workflows, give the smallest command sequence that reaches the goal.
- For automation-oriented tasks, prefer `--json` or `export --format json`.
