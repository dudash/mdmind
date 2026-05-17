---
name: mdm-cli-inspection
description: Inspect, validate, query, audit, deep-link, list external refs, or export mdmind Markdown outline and mind map files with the mdm CLI. Use for existing maps, structured notes, knowledge maps, tags, metadata, ids, relations, external Markdown refs, subtree inspection, filtered search, plain/json output, or exports to JSON, Mermaid, and OPML. Do not use this skill for messy source ingestion; agents should read PDFs, websites, and prose-heavy HTML themselves, author the mdmind map with map-authoring guidance, then validate it.
license: Apache-2.0
metadata:
  author: mdmind
  version: "0.7.0"
---

# mdm CLI Inspection

Use `mdm` as the read, query, validate, and export surface for map files.

## Use For

- Validate a map file before handoff.
- Inspect a map from the CLI without launching an interactive UI.
- Answer questions about tags, metadata, ids, relations, or branches in a map.
- Find items with text, `#tags`, or `@key:value` filters.
- Audit structured fields like owner, status, region, priority, or source.
- List deep-link ids.
- List external Markdown refs to local files, URLs, and images.
- Check incoming and outgoing relations.
- Export a full tree, a subtree, or a filtered working set.
- Inspect TODO maps for active, blocked, done, owner, priority, or area-based work.

## Do Not Use For

- Ingesting PDFs, websites, saved articles, or prose-heavy HTML. Agents are usually better at reading the source, deciding the outline structure, writing the mdmind map directly, and validating it.
- Converting external outline files unless the user explicitly asks to use `mdm import`; keep this skill focused on inspection, validation, querying, and export.
- Rewriting large parts of a map unless the user is explicitly asking for file edits.
- Purely visual diagram design.

If the task is mainly about creating or reshaping map content, use `mdmind-map-authoring` instead.

## Defaults

- Start with the smallest command that answers the question.
- Use `mdm commands --json` when you need to discover command safety,
  interactivity, arguments, flags, output modes, and examples locally.
- Prefer `--plain` for grep-friendly inspection and quick human scanning.
- Use command `--json` when another tool needs the mdm response envelope with
  `ok`, `command`, `format`, `data`, `summary`, `error`, and `next_actions`.
- Use `mdm export --format json` when another tool needs raw map document data.
- Prefer deep links like `map.md#product/tasks` when the user cares about one branch.
- Prefer `--query` on export when the user wants a filtered machine-readable subset.
- Use `mdm refs <file>` when the question is about attached external files, URLs, or image refs.
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
- Use `mdm refs <file>` to list external Markdown refs attached to nodes.
- Use `mdm relations <file>` or `mdm relations <file>#id` to inspect graph edges.
- Use `mdm export <file> --format json|mermaid|opml` for downstream tools.
- Use `mdm commands --json` to discover the current command catalog.

## TODO Map Inspection

For task-heavy maps, start with active and blocked work before broader summaries:

```bash
mdm find TODO.md "#todo @status:active" --plain
mdm find TODO.md "task:open" --plain
mdm find TODO.md "task:done" --plain
mdm find TODO.md "@status:blocked" --plain
mdm kv TODO.md --keys status,owner,priority,area --plain
mdm view TODO.md#todo/focus
mdm validate TODO.md
```

Use `find` for the working set, `kv` for ownership/status audits, and `view` with a deep link when the user needs one branch in context. After agent edits, run `validate` before summarizing the handoff.

When inspecting task files, expect explicit checkbox markers to round-trip in the raw map, task-aware filters like `task:open` to find checkbox/tag/status task conventions, and derived parent rollups to appear in rendered views.

## Workflow

1. Clarify whether the user wants validation, inspection, query results, deep links, external refs, relations, or export.
2. Choose the narrowest `mdm` command that answers that need.
3. If one branch matters, use a deep link target instead of the whole file.
4. If the user is exploring, prefer plain output and small, focused queries.
5. If the user needs a machine-consumable result, choose deliberately:
   use command `--json` for an mdm response envelope, or
   `mdm export --format json` for raw document JSON.
6. If the user asks to turn messy source material into a map, use map-authoring guidance and validate the authored result rather than relying on deterministic import.
7. If the map looks malformed or generated, run `mdm validate` before drawing conclusions.
8. Summarize the important result, not just the fact that a command ran.

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
4. If using `refs`, distinguish external Markdown refs from internal ids or `[[...]]` relations.
5. Check that export format matches the downstream use case.
6. If using `relations`, be explicit whether the result is whole-map outgoing relations or node-focused incoming plus outgoing relations.
7. If authoring from messy source material, validate the authored map and mention that agent interpretation was used.

## Example User Prompts

- “Validate this mdmind file and tell me if the ids and relations are clean.”
- “Find all blocked launch items owned by jason in this map.”
- “List the local files and web links attached to this map.”
- “Export only the active todo branches from this map as JSON.”

## Gotchas

- `find` is for matching content; `links` is for discovering stable ids.
- `refs` is for Markdown links/images that point outside the map; `links` is for map node ids.
- `relations file.md` and `relations file.md#id` answer different questions.
- `export --query` can return an empty tree if the filter matches nothing.
- Command-style `--json` returns an envelope. The command-specific payload is
  under `data`; do not expect top-level arrays from `find --json`, `refs --json`,
  or `validate --json`.
- `mdm export --format json` intentionally remains raw document JSON instead of
  an envelope.
- `--plain` and `--json` are mutually exclusive on commands that support both.
- Agents are often better than deterministic import for messy ingestion: they can read PDFs/sites/prose, choose useful structure, preserve intent, and create cleaner mdmind files.
- If a file has parser errors, some higher-level conclusions are unreliable until `validate` is addressed.

## Return Format

- For user questions, summarize the answer and include the exact command when useful.
- For workflows, give the smallest command sequence that reaches the goal.
- For automation-oriented tasks, prefer command `--json` for response metadata
  and `mdm export --format json` for raw map data.
