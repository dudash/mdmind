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
checkpoints. It also adds guided job templates so users do not have to invent
perfect agent prompts from scratch.

For the user-facing experience model, see
[EXPERIENCE_MODEL.md](EXPERIENCE_MODEL.md). The short version is: users ask an
agent to organize, link, generate, or maintain a workspace; `mdmind .` is where
they inspect and edit the result; `mdm mindspace ...` is the deterministic
contract agents and tests rely on underneath.

## Product Model

A mindspace is a local folder that agents, `mdm`, and `mdmind` can treat as one
working context while keeping files plain, local, and inspectable.

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
such as `obsidian-vault` or `llm-wiki`.

Job templates sit above the manifest. A template describes the work to be done,
the expected outputs, and the review path. It may guide manifest setup, context
selection, generated maps, review items, and TUI emphasis, but the manifest
remains the durable role contract for the folder. See
[JOB_TEMPLATES.md](JOB_TEMPLATES.md).

The user-facing onboarding path is agent-first:

```text
Organize this folder as a mindspace. Keep sources read-only, identify maps and
ordinary pages, propose a manifest, and show me what you would write before you
write it.
```

The deterministic substrate is inspect first, then explicitly set up:

```bash
mdm mindspace scan .
mdm mindspace setup . --preview
mdm mindspace setup . --write
mdmind .
```

Docs for scripts and agents may show these commands directly. Product stories
should lead with the natural-language request and `mdmind` review surface.

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
| Job template | A reusable work pattern with persona fit, starting prompt, expected roles, map shapes, checks, write policy, review surface, success criteria, and customization knobs. |
| Agent session | A scoped collaboration episode with goal, role, target branch or file, allowed paths, context bundle, status, proposed edits, validation results, and closeout summary. |
| Review item | A proposed writeback, relation, repair, move, or stale-digest fallback with target, rationale, diff or proposed record, validation state, and accept/reject status. |
| Checkpoint | A restorable local safety snapshot before risky writes, covering affected maps, generated artifacts, reviews, source metadata, or the whole mindspace when needed. |

## Branch Refs And Relations

Mindspace branch refs use one canonical shape:

```text
maps/tasks.md#todo/focus
```

Same-file ids remain scoped to the current map:

```text
[[todo/focus]]
[[rel:supports->todo/focus]]
```

Cross-file branch refs include the path and id:

```text
[[maps/decisions.md#decision/api-shape]]
[[rel:implements->maps/decisions.md#decision/api-shape]]
```

The current core parser and CLI preserve relation targets as text, but classify
them as:

| Target kind | Example | Meaning |
| --- | --- | --- |
| `same_file_id` | `[[todo/focus]]` | Branch id in the current map only. |
| `path_qualified_branch` | `[[maps/tasks.md#todo/focus]]` | Branch id in another Markdown map, resolved by current validation from the map directory or an ancestor workspace root. |
| `external_file` | `[[sources/brief.pdf]]` | File-level reference; use normal Markdown links for rich source citations unless the relation itself is meaningful. |
| `url` | `[[https://example.com]]` | External URL relation; uncommon, but preserved when explicit. |

`mdm validate <map>` now checks path-qualified branch refs when the target file
is readable Markdown. It warns for missing files, missing branch ids, duplicate
target ids, non-Markdown branch targets, and same-file ids that do not exist.
This is still single-map validation with local cross-file checks, not full
mindspace inventory. Full folder-wide incoming backlinks and workspace switcher
behavior belong to `mdm mindspace scan`, `mdm mindspace lint`, and `mdmind .`.

## Command Vocabulary

Mindspace commands are future commands unless already implemented. Existing
single-file commands remain unchanged. Session and review commands live under
`mdm mindspace`, not a broad `mdm agent` namespace.

These commands are the public contract, not the required primary UI. Agent
skills, project instructions, MCP adapters, and the TUI should all use this same
behavior instead of creating hidden alternatives.

| Command | Stage | Contract |
| --- | --- | --- |
| `mdm mindspace new <path>` | v1 | Create a fresh native mdmind workspace. Existing `mdm init <path>` remains single-map creation. |
| `mdm mindspace scan <root>` | Current | Inspect a folder read-only, with or without a manifest. |
| `mdm mindspace setup <root> --preview` | Current | Print the proposed `.mdmind/mindspace.json` without writing. |
| `mdm mindspace setup <root> --write` | Current | Create or update `.mdmind/mindspace.json` only; never move or rewrite existing notes. |
| `mdm mindspace lint <root>` | Current | Report deterministic structural problems without AI judgment. |
| `mdm mindspace context <target>` | Current | Export bounded context with provenance and budget controls. |
| `mdm mindspace template list` | Current | List built-in job templates. Trusted local templates are future. |
| `mdm mindspace template show <id>` | Current | Print one built-in template as JSON, human text, or an agent prompt. |
| `mdmind .` | MVP | Open the human workspace surface while preserving one-active-map editing. |
| `mdm mindspace session ...` | Current | Create and manage durable session records; current apply is preview-only. |
| `mdm mindspace review ...` | Current | List, approve, reject, or stale-mark durable review records without mutating maps. |

Reserved JSON format names:

| Format | Payload |
| --- | --- |
| `mindspace_scan.v1` | Inventory, detected roles, validation summaries, warnings, and next actions. |
| `mindspace_setup.v1` | Proposed or written manifest plus role explanations and write summary. |
| `mindspace_diagnostics.v1` | Deterministic lint diagnostics with stable issue codes. |
| `mindspace_context.v1` | Context bundle with included items, provenance, budgets, and omission reasons. |
| `mindspace_template_catalog.v1` | Template list with ids, names, persona fit, job fit, safety defaults, and built-in/local provenance. |
| `mindspace_template.v1` | One template with prompt, expected roles, map shapes, checks, write policy, review surface, and customization knobs. |
| `mindspace_session.v1` | Session records, state transitions, plans, previews, and closeouts. |
| `mindspace_review.v1` | Review items, decisions, rationale, and target/digest state. |

### Current Scan And Lint Output

`mdm mindspace scan <root> --json` is the first implemented Mindspace
substrate. It is read-only and succeeds even when it finds deterministic
diagnostics, so agents can inventory imperfect folders before deciding what to
do next.

The `mindspace_scan.v1` data payload includes:

| Field | Meaning |
| --- | --- |
| `root` | Canonical scanned root path. |
| `manifest` | Presence, path, schema version, name, role count, and validity for `.mdmind/mindspace.json`. |
| `summary` | Files scanned, directories scanned, role counts, and diagnostic counts. |
| `roles` | Detected role records with role, path, file/directory kind, safety flags, and detection reason. |
| `maps` | Native map records with parse status, validation counts, ids, tags, metadata keys, refs, relations, and task counts. |
| `diagnostics` | Stable coded diagnostics for manifest, scan, parser, and validation findings. |
| `skipped` | Ignored dependency, generated, or hidden folders. |

Current role inference is intentionally conservative:

- `sources/`, `source/`, `raw/`, and `references/` are `source`; source records
  are read-only by default.
- `inbox/` and `_inbox/` are `inbox`.
- `.mdmind/reports/` is `report` and generated.
- `AGENTS.md`, `CLAUDE.md`, and `GEMINI.md` are trusted `instruction` files.
- `index.md` is `index`; `log.md`, `activity.md`, and `journal.md` are `log`.
- Other Markdown files are classified through the existing mdmind map parser as
  native `map`, damaged native `map`, or ordinary `page`. Damaged-map
  classification is conservative in folder scans so docs with mdmind examples
  stay ordinary pages unless their path or name looks map-like.

`mdm mindspace lint <root> --json` reuses the same scan and returns
`mindspace_diagnostics.v1`. It exits `1` only when diagnostics include errors;
warnings remain visible but do not fail the command.

### Current Setup Output

`mdm mindspace setup <root> --preview --json` emits `mindspace_setup.v1` without
writing files. `mdm mindspace setup <root> --write --json` writes only
`.mdmind/mindspace.json`; it does not create reports, move notes, rewrite maps,
or import Markdown.

Setup consumes the current read-only scan inventory, preserves existing manifest
role overrides and settings when a manifest already exists, then adds inferred
roles for detected maps, pages, sources, inbox, indexes, logs, instructions, and
the generated report sidecar path `.mdmind/reports`.

The `mindspace_setup.v1` data payload includes:

| Field | Meaning |
| --- | --- |
| `root` | Canonical setup root path. |
| `manifest_path` | Always `.mdmind/mindspace.json`. |
| `mode` | `preview` or `write`. |
| `written` | Whether the manifest was written. |
| `existing_manifest` | Whether setup updated an existing manifest proposal. |
| `created_directory` | Whether `--write` created `.mdmind/`. |
| `template` | Optional template guidance used for setup explanation. |
| `summary` | Role totals, preserved/inferred/added counts, and scan diagnostics. |
| `manifest` | The full proposed or written manifest object. |
| `notes` | Human-readable safety notes agents can summarize. |

### Current Context Output

`mdm mindspace context <target> --json` emits `mindspace_context.v1` without
writing files. It scans the mindspace root, loads native map files, and exports
a bounded context bundle an agent can cite later.

Current inputs:

```bash
mdm mindspace context maps/roadmap.md#roadmap/current --json
mdm mindspace context . --query "@owner:jason" --max-branches 8 --json
mdm mindspace context maps/roadmap.md#roadmap/current --include-source-refs
```

Supported controls:

| Flag | Meaning |
| --- | --- |
| `--root <path>` | Mindspace root to scan; defaults to the current directory. |
| `--query <query>` | Include matching branches across scanned maps using the existing filter language. |
| `--template <id>` | Attach built-in job-template guidance to the bundle metadata. |
| `--relation-depth <n>` | Follow outgoing same-file and path-qualified branch relations up to the given depth. |
| `--include-backlinks` | Include incoming relation sources for included ids. |
| `--include-source-refs` | Include bounded local source-reference excerpts and URL records. |
| `--max-files <n>` | Maximum distinct map files included in branch records. |
| `--max-branches <n>` | Maximum branch records included. |
| `--max-detail-chars <n>` | Maximum detail characters across included branches. |
| `--max-source-chars <n>` | Maximum characters per included local source excerpt. |

The `mindspace_context.v1` data payload includes:

| Field | Meaning |
| --- | --- |
| `root` | Canonical scanned root path. |
| `target` | Requested target, such as `maps/roadmap.md#roadmap/current` or `.`. |
| `query` | Optional query used to select branches. |
| `template` | Optional built-in job template metadata. |
| `options` | Relation, backlink, source, file, branch, detail, and source budgets. |
| `summary` | Map, file, branch, source, omission, and diagnostic counts. |
| `branches` | Included branch records with file path, line, id, breadcrumb, inclusion reason, relation depth, and bounded subtree. |
| `sources` | Optional source-reference records with target, kind, origin branch, read-only flag, byte count, excerpt, and omitted chars. |
| `omitted` | Deterministic omission records for budget limits, disabled source refs, unresolved relations, and unreadable sources. |
| `diagnostics` | Stable scan diagnostics produced while building the bundle. |

Context bundles are deterministic and non-AI. They rank by discovered order,
relation expansion, and explicit budgets; they do not summarize sources with an
LLM or fetch URLs.

### Current Session And Review Output

`mdm mindspace session ...` emits `mindspace_session.v1` for durable agent
session records under `.mdmind/sessions/`. `mdm mindspace review ...` emits
`mindspace_review.v1` for durable review items under `.mdmind/reviews/`.

Current session commands:

```bash
mdm mindspace session start maps/tasks.md#todo/focus --role implementer --json
mdm mindspace session plan <session-id> --json
mdm mindspace session apply <session-id> --preview --json
mdm mindspace session submit <session-id> --rationale "ready for review" --json
mdm mindspace session close <session-id> --json
```

Current review commands:

```bash
mdm mindspace review list --json
mdm mindspace review approve <review-id> --json
mdm mindspace review reject <review-id> --reason "wrong target branch" --json
```

The current substrate is record-first. It writes session/review JSON sidecars
and target digests; it does not mutate map files. `session apply --preview` is
read-only and reports review state. `review approve` records approval only after
checking the current target digest. If the target changed, the review becomes
`stale` instead of applying silently.

The `mindspace_session.v1` payload includes:

| Field | Meaning |
| --- | --- |
| `session` | Session record with id, status, root, target, role, goal, scope, target snapshot, review ids, timestamps, and notes. |
| `current_snapshot` | Current target digest and provenance at read time. |
| `stale` | Whether the current target digest differs from the session target snapshot. |
| `notes` | Safety and next-action notes. |

The `mindspace_review.v1` payload includes:

| Field | Meaning |
| --- | --- |
| `id` | Review id and filename stem under `.mdmind/reviews/`. |
| `session_id` | Source session id. |
| `status` | `pending`, `approved`, `rejected`, or `stale`. |
| `target_snapshot` | Digest/provenance captured when the session started. |
| `current_snapshot` | Digest/provenance checked when the review was submitted or decided. |
| `stale` | Whether current target content differs from the captured digest. |
| `rationale` | Agent or user rationale for the review item. |
| `proposal` | Optional proposal text; current commands store it but do not apply it. |
| `decision_reason` | Approval, rejection, or stale-digest decision note. |

### Current Template Output

`mdm mindspace template list --json` exposes the built-in persona/job template
catalog as `mindspace_template_catalog.v1`.

The catalog includes:

| Field | Meaning |
| --- | --- |
| `id` | Stable CLI id such as `launch-planning`. |
| `name` | Human-readable template name. |
| `persona_fit` | Primary persona this template was designed around. |
| `job_fit` | Work the template is meant to guide. |
| `primary_outputs` | Map or page paths the template tends to produce or update. |
| `safety_defaults` | Compact write-policy defaults for discovery. |
| `provenance` | `built_in` for packaged templates. |

`mdm mindspace template show <id> --json` exposes one built-in template as
`mindspace_template.v1`, including starting prompt, folder roles, map shapes,
agent workflow, deterministic checks, write policy, `mdmind .` review surface,
success criteria, customization knobs, and provenance.

`--prompt` prints a copyable agent prompt plus safety and review guidance for
users who do not yet know how to ask for the Mindspace job.

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
- built-in persona/job templates and helper commands
- path-qualified branch refs
- deterministic `lint`
- bounded `context` bundles with provenance
- TUI workspace entry, switcher, peek/open navigation, recents, pinned maps, and
  cross-map navigation
- an `mdmind .` landing that can show the active template, agent-touched files,
  and review path

### v1: Reviewable Agent Teaming

v1 adds durable collaboration:

- packaged `mdmind-mindspace-workflow` skill with template references
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
