# Mindspace Reference

Mindspace is the optional folder-level workspace contract for `mdmind`.

The product rationale lives in
[../product/prds/mindspace-user-need-and-ecosystem-fit.md](../product/prds/mindspace-user-need-and-ecosystem-fit.md).
This reference is the implementation target for future Mindspace work. Keep
Mindspace-specific docs, design notes, tips, and contracts in this section
unless the behavior is also core single-map behavior.

Mindspace does not replace single-file maps. `mdmind file.md`, `mdm view
file.md`, `mdm validate file.md`, and the existing `.md` map format remain
first-class. Mindspace adds a folder inventory and safety model around maps,
Markdown pages, sources, generated artifacts, agent sessions, reviews, and
checkpoints.

## Product Model

A mindspace is a local folder that `mdm` and `mdmind` can inspect as one working
context while keeping files plain, local, and inspectable.

A mindspace can contain mixed roles:

| Role | Meaning |
| --- | --- |
| `map` | Native mdmind `.md` maps with tree structure, ids, tags, metadata, details, tasks, and relations. |
| `page` | Ordinary Markdown prose pages that should not be forced into the native map parser. |
| `source` | Raw or tracked inputs used for synthesis, claims, decisions, or reports. Sources are read-only by default. |
| `inbox` | Loose captures waiting for triage, mapping, linking, or archival. |
| `index` | Human-readable generated or maintained navigation entrypoint. |
| `log` | Append-oriented history, run notes, or activity records. |
| `instruction` | Trusted local guidance such as `AGENTS.md`, `CLAUDE.md`, or workflow rules. |
| `report` | Generated scan, lint, source, context, or review output. |

The manifest records roles. It does not require every Markdown file to become a
native mdmind map, and it does not ask users to choose visible adoption profiles
such as `obsidian-vault` or `llm-wiki`. The onboarding path is inspect first,
then explicitly set up:

```bash
mdm mindspace scan .
mdm mindspace setup . --preview
mdm mindspace setup . --write
mdmind .
```

## Manifest Schema

The manifest path is `.mdmind/mindspace.json`.

The format name is `mdmind.mindspace.v1`.

Minimal manifest fields:

| Field | Type | Required | Meaning |
| --- | --- | --- | --- |
| `schema_version` | string | yes | Must be `mdmind.mindspace.v1` for this contract. |
| `name` | string | yes | Human-readable workspace name. |
| `root` | string | yes | Root path relative to the manifest location; use `..` for the folder containing `.mdmind/`. |
| `roles` | array | yes | Role entries that describe files, folders, or globs. |
| `settings` | object | yes | Mindspace-level defaults for safety and deterministic checks. |

Role entry fields use snake_case:

| Field | Type | Required | Meaning |
| --- | --- | --- | --- |
| `role` | string | yes | One of `map`, `page`, `source`, `inbox`, `index`, `log`, `instruction`, or `report`. |
| `path` | string | conditional | File or directory path relative to the mindspace root. Required when `glob` is absent. |
| `glob` | string | conditional | Glob relative to the mindspace root. Required when `path` is absent. |
| `read_only` | boolean | no | Defaults to `true` for `source`; otherwise false unless configured. |
| `generated` | boolean | no | Marks files that mdmind may regenerate only through explicit write commands. |
| `append_only` | boolean | no | Marks logs or history files that should not be rewritten in place. |
| `trusted` | boolean | no | Marks local instructions as trusted guidance, distinct from untrusted source content. |

Example:

```json
{
  "schema_version": "mdmind.mindspace.v1",
  "name": "Research Brain",
  "root": "..",
  "roles": [
    {"role": "map", "glob": "maps/**/*.md"},
    {"role": "page", "glob": "wiki/**/*.md"},
    {"role": "source", "path": "sources", "read_only": true},
    {"role": "inbox", "path": "inbox"},
    {"role": "index", "path": "index.md", "generated": true},
    {"role": "log", "path": "log.md", "append_only": true},
    {"role": "instruction", "path": "AGENTS.md", "trusted": true},
    {"role": "report", "path": ".mdmind/reports", "generated": true}
  ],
  "settings": {
    "checkpoint_before_risky_write": true,
    "source_read_only_default": true,
    "review_on_stale_digest": true,
    "inbox_stale_days": 14
  }
}
```

Generated inventory and source hashes stay out of the manifest. They belong in
generated sidecars such as `.mdmind/inventory.json`, source records, or future
lock/report files.

## Core Objects

These are product objects first. Storage can evolve as long as command output
preserves the contract.

| Object | Required shape |
| --- | --- |
| Map record | A known native mdmind map with path, parse status, validation summary, ids, tags, metadata keys, outgoing refs, and modified digest. |
| Branch ref | A stable pointer to a branch by file path plus branch id, for example `maps/tasks.md#todo/focus`. Same-file ids remain file-scoped. |
| Relation edge | A same-file or cross-file relation with source branch, target, relation kind, parse provenance, and resolution status. |
| Source record | A tracked source with path, kind, title, hash or digest when available, modified time, added time, and optional canonical URL. |
| Context bundle | A bounded packet for a task with target, included branches/pages/sources, provenance, inclusion reasons, budgets, and output format. |
| Agent session | A scoped collaboration episode with goal, role, target branch or file, allowed paths, context bundle, status, proposed edits, validation results, and closeout summary. |
| Review item | A proposed writeback, relation, repair, move, or stale-digest fallback with target, rationale, diff or proposed record, validation state, and accept/reject status. |
| Checkpoint | A restorable local safety snapshot before risky writes, covering affected maps, generated artifacts, reviews, source metadata, or the whole mindspace when needed. |

## Command Vocabulary

Mindspace commands are future commands unless already implemented. Existing
single-file commands remain unchanged. Future session and review commands live
under `mdm mindspace`, not a broad `mdm agent` namespace.

| Command | Stage | Contract |
| --- | --- | --- |
| `mdm mindspace new <path>` | v1 | Create a fresh native mdmind workspace. Existing `mdm init <path>` remains single-map creation. |
| `mdm mindspace scan <root>` | MVP | Inspect a folder read-only, with or without a manifest. |
| `mdm mindspace setup <root> --preview` | MVP | Print the proposed `.mdmind/mindspace.json` without writing. |
| `mdm mindspace setup <root> --write` | MVP | Create or update `.mdmind/mindspace.json` only; never move or rewrite existing notes. |
| `mdm mindspace lint <root>` | MVP | Report deterministic structural problems without AI judgment. |
| `mdm mindspace context <target>` | MVP | Export bounded context with provenance and budget controls. |
| `mdmind .` | MVP | Open the human workspace surface while preserving one-active-map editing. |
| `mdm mindspace session ...` | v1 | Create and manage scoped agent collaboration sessions. |
| `mdm mindspace review ...` | v1 | List, approve, reject, or inspect proposed writeback and repair items. |

Reserved JSON format names:

| Format | Payload |
| --- | --- |
| `mindspace_scan.v1` | Inventory, detected roles, validation summaries, warnings, and next actions. |
| `mindspace_setup.v1` | Proposed or written manifest plus role explanations and write summary. |
| `mindspace_diagnostics.v1` | Deterministic lint diagnostics with stable issue codes. |
| `mindspace_context.v1` | Context bundle with included items, provenance, budgets, and omission reasons. |
| `mindspace_session.v1` | Session records, state transitions, plans, previews, and closeouts. |
| `mindspace_review.v1` | Review items, decisions, rationale, and target/digest state. |

## Safety Model

Mindspace is local-first and deterministic before it is agentic.

- `scan`, `lint`, and `context` must not write user files.
- `setup --preview` must not write. `setup --write` may write only
  `.mdmind/mindspace.json` and required parent directories.
- Source roles are read-only by default.
- Trusted instruction files and manifest rules are treated separately from
  untrusted source content.
- Risky writes create or require checkpoints first.
- Batch repair, generated relation creation, and agent writeback default to
  reviewable proposals when confidence or freshness is uncertain.
- Agent sessions must declare target branches or files and allowed write scopes.
- A write proposal based on a stale file or branch digest becomes a review item
  instead of silently applying.
- Git integration may be useful later, but Git is not the default safety model
  and is not required for local restore.

## Phases

### MVP: Navigation And Deterministic Context

The MVP makes a folder understandable and navigable:

- `.mdmind/mindspace.json`
- read-only `scan`
- previewable `setup`
- path-qualified branch refs
- deterministic `lint`
- bounded `context` bundles with provenance
- TUI workspace entry, switcher, peek/open navigation, recents, pinned maps, and
  cross-map navigation

### v1: Reviewable Agent Teaming

v1 adds durable collaboration:

- `session` and `review` objects
- scoped writeback with digest checks
- checkpoint-before-write flows
- source records and stale-source reports
- generated `index.md` and `log.md`
- structured JSON envelopes for all Mindspace commands

### v2: Ecosystem And Advanced Automation

v2 exposes the stable local model to richer integrations:

- MCP or OpenClaw-compatible tool adapters
- packaged Hermes/OpenClaw skills or plugins
- named working sets and optional saved layouts
- graph and unlinked-mention review lenses
- policy rules for low-risk automatic approvals

Broad packaging, paid-product boundaries, and audience-specific starter packs
remain downstream of MVP validation.
