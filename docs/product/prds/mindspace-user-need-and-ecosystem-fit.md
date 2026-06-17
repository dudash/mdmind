# PRD: Mindspace User Need And Ecosystem Fit

Status: Product rationale; canonical Mindspace docs live in
[../../mindspace/](../../mindspace/)
Last reviewed: 2026-06-17

## Summary

This PRD explains the product need and ecosystem fit for Mindspace. The
implementation contract lives in
[../../mindspace/REFERENCE.md](../../mindspace/REFERENCE.md).

`mdmind` is a local-first structured knowledge workspace with three product
surfaces:

- `mdmind`, the human-facing TUI for outlines, notes, todos, decisions, maps,
  reviews, and focused navigation
- `mdm`, the CLI surface for agents, scripts, validation, inspection, export,
  and future workspace maintenance
- the mdmind `.md` spec, a portable plain-text structure for hierarchy, ids,
  details, tags, metadata, relations, tasks, and deep links

Mindspace should extend these surfaces from one map file to a folder-level
workspace without turning mdmind into Obsidian-in-a-terminal. The first bet is
not "replace Obsidian" or "be another LLM wiki." The first bet is:

> Users who let agents maintain Markdown knowledge need deterministic structure,
> inspection, repair, and context packaging so the knowledge base stays useful
> as it grows.

Mindspace should support native mdmind workspaces and mixed Markdown folders.
That lets `mdmind` stand alone for outline/map-first users while also improving
Obsidian, Hermes LLM Wiki, Claude Code, OpenClaw, and other agent-maintained
Markdown workflows.

The single-map editor remains the core workshop. Mindspace adds the layer around
it: inventory, switching, indexing, linting, context packaging, agent sessions,
reviewable writeback, and restore points.

## Problem

Agent-maintained knowledge bases are becoming a recognizable workflow:

1. Put raw sources into a local folder.
2. Give the agent a schema or instructions file.
3. Let the agent compile sources into Markdown pages.
4. Browse and edit the result in a human notes tool.
5. Periodically ask the agent to ingest, query, lint, and repair the knowledge
   base.

This is powerful, but most implementations rely on convention and agent
discipline:

- indexes drift
- logs become inconsistent
- links break
- source-backed claims become stale
- agents read too much or too little context
- duplicate notes appear for the same concept
- todos and decisions disappear into prose pages
- users cannot tell what the agent changed or why
- health checks depend on model judgment instead of deterministic tooling

Users can paper over this with better prompts, but that only works while the
workspace is small and the agent behaves well. `mdmind` can turn parts of the
workflow into inspectable product behavior.

## Existing Signals

### Karpathy LLM Wiki

Karpathy's LLM Wiki pattern describes a persistent Markdown wiki maintained by
an LLM, with raw immutable sources, generated wiki pages, and a schema file such
as `CLAUDE.md` or `AGENTS.md`. The repeated operations are ingest, query, lint,
index, and log. Obsidian is described as the IDE, the LLM as the programmer, and
the wiki as the codebase.

Product implication: mdmind should not assume the wiki is one format. It should
understand sources, prose pages, indexes, logs, agent instructions, and native
maps as different roles in the same local workspace.

Source: <https://gist.github.com/karpathy/442a6bf555914893e9891c11519de94f>

### Obsidian-Based LLM Wiki Workflows

User writeups and Obsidian-oriented skills show a common desire: keep Obsidian
as the page-first human interface while an agent handles cross-linking,
classification, inbox processing, and wiki maintenance.

Recurring workflow primitives:

- `sources/` or `raw/` for immutable source material
- `inbox/` for loose notes and unprocessed captures
- `wiki/` for compiled Markdown pages
- `index.md` for navigation
- `log.md` for history
- agent instructions for folder conventions and update rules
- commands or prompts for ingest, process inbox, query, and lint

Product implication: mdmind should integrate with Obsidian conventions without
requiring all notes to become mdmind maps. Native mdmind maps should be used
where hierarchy, branch ids, tasks, decisions, evidence, and relations add
clear value.

Sources:

- <https://aimaker.substack.com/p/llm-wiki-obsidian-knowledge-base-andrej-karphaty>
- <https://www.mindstudio.ai/blog/andrej-karpathy-llm-wiki-knowledge-base-claude-code>
- <https://github.com/kepano/obsidian-skills>

### Hermes And OpenClaw

Hermes classifies `llm-wiki` as a research skill and Obsidian as note-taking.
That split is useful: LLM Wiki is a workflow, Obsidian is a human notes surface,
and mdmind can be an outline/map-first workspace plus deterministic agent
interface.

OpenClaw's docs distinguish tools, skills, and plugins: skills teach workflows,
tools perform typed actions, and plugins package runtime capabilities. That
maps cleanly to mdmind:

- mdmind skills teach map authoring and CLI inspection
- `mdm` commands can become typed tools later
- plugin packaging can distribute both skills and tools

Product implication: first ship the local workflow and CLI contract. Package for
agent ecosystems after the user need is clear, not before.

Sources:

- <https://hermes-agent.nousresearch.com/docs/reference/skills-catalog/>
- <https://docs.openclaw.ai/tools>

### Existing LLM Wiki Implementations

Open-source implementations and comments around the LLM Wiki pattern repeatedly
add helper behavior for discovery, init, lint, provenance, source hashes,
triage reports, contradiction detection, and "fix the wiki" loops.

Product implication: these are not one-off ideas. They are the places where
agents need deterministic support. `mdm mindspace lint`, `sources report`,
`context`, and reviewable writeback are likely higher-value than broad template
surface area.

Sources:

- <https://github.com/ekadetov/llm-wiki>
- <https://github.com/atomicstrata/llm-wiki-compiler>

### Agentic Teaming Report

The agentic teaming report reviewed for this PRD reinforces a more concrete
product shape:

- keep one active map editor, then add a workspace landing view, switcher,
  recents, pinned working set, and cross-map navigation
- make agent collaboration an explicit session and review model, not an
  invisible chat habit
- use branch-level digests and review fallbacks for concurrent human/agent work
  instead of introducing CRDT complexity
- treat prompt injection, untrusted source content, write scopes, and
  checkpoint-before-write as core product requirements
- stabilize the shell/JSON CLI contract before adding protocol adapters such as
  MCP or runtime-specific tool servers

Product implication: the PRD should promote TUI switching and reviewable agent
sessions from "nice future idea" to staged product requirements.

## Product Positioning

Mindspace should be positioned as:

> An outline/map-first local knowledge workspace that keeps agent-maintained
> Markdown folders inspectable, navigable, and safe to update.

This preserves the existing product truth:

- `mdmind` is a human-facing TUI, not only an agent backend.
- `mdm` is a real CLI product surface, not only a helper binary.
- mdmind `.md` files are durable plain-text artifacts, not hidden database
  records.

## Ecosystem Roles

| Surface | Best role | mdmind relationship |
| --- | --- | --- |
| Obsidian | Page-first notes, browsing, graph view, plugins, sync, longform Markdown | Companion, not primary replacement target |
| LLM Wiki pattern | Workflow for raw sources, generated wiki, schema, index, log | Compatible pattern mdmind can make more deterministic |
| Hermes `llm-wiki` | Research skill for maintaining LLM wikis | Complement; mdmind provides structure, validation, context bundles, source checks |
| OpenClaw | Agent runtime with tools, skills, plugins | Distribution and future typed-tool surface |
| mdmind TUI | Human outline/map workspace | Primary product surface for structured users |
| mdm CLI | Agent/script maintenance layer | Primary deterministic automation surface |
| mdmind `.md` spec | Structured local artifact format | Substrate for maps inside mixed Markdown folders |

## Core Objects

Mindspace should make a few workspace objects explicit. These are product
objects first; implementation storage can evolve.

| Object | Purpose |
| --- | --- |
| Map record | one known mdmind map in the folder inventory |
| Branch ref | stable pointer to a branch by file path plus id |
| Relation edge | same-file or cross-file relation with kind and provenance |
| Source record | immutable or tracked source with path, title, hash, and timestamps |
| Context bundle | bounded packet for an agent or human task |
| Agent session | scoped collaboration episode with goal, role, target, and permissions |
| Review item | proposed writeback, relation, repair, or move needing human action |
| Checkpoint | restorable local safety snapshot before risky writes |

## Target Users

### Primary: Agent-Heavy Builder Or Researcher

This user works in a repo or local folder with agents. They already have project
notes, decisions, todos, research snippets, docs, and handoff notes scattered
across Markdown files, chat history, and issue trackers.

Jobs:

- preserve decisions and next actions across sessions
- hand a focused branch to an agent without rereading the whole folder
- keep local task memory inspectable
- validate that agent-written maps still parse
- review what the agent changed

Why mdmind:

- TUI for shaping the map as a human
- CLI for agent validation, query, and export
- stable branch ids for exact handoff targets
- task and decision maps stay readable as plain text

### Secondary: Obsidian LLM Wiki User

This user likes Obsidian and wants an LLM-maintained second brain. They do not
want a second full notes app, but they need better maintenance, linting, and
structured project memory.

Jobs:

- ingest sources without losing provenance
- process inbox notes into durable pages or maps
- find broken links and orphan pages
- keep index and log current
- identify stale source-backed claims
- create structured task, decision, claim, and evidence maps inside the vault

Why mdmind:

- works beside Obsidian rather than replacing it
- can parse native maps where structure matters
- can lint mixed Markdown conventions
- can export context packets for agents

### Tertiary: Terminal-First Outliner User

This user prefers terminal-native workflows and wants mdmind to be the primary
workspace for notes, todos, outlines, and structured research.

Jobs:

- open a folder as a workspace
- switch between maps
- review inbox, open todos, stale sources, and broken links
- use CLI and TUI interchangeably

Why mdmind:

- native TUI is the daily interface
- maps are plain text and git-friendly
- agent workflows are optional, not required

## Non-Goals

Mindspace should not initially:

- replace Obsidian's plugin ecosystem, graph view, mobile/sync story, or longform
  prose workflow
- require every Markdown file in a folder to become a mdmind map
- provide hosted sync, team permissions, or cloud RAG
- do LLM summarization inside `mdm`
- make vector search part of the MVP
- expose Git as the normal user mental model
- create many user-visible "profiles" such as `obsidian-vault`,
  `claude-obsidian`, `llm-wiki`, or `plain-markdown`

## Product Principles

### Adopt The Folder, Do Not Ask Users To Classify It

Users should not have to pick a product taxonomy. They have a folder. `mdm`
should inspect it and explain what it found.

Good user-facing shape:

```bash
mdm mindspace scan .
mdm mindspace setup . --preview
mdmind .
```

`scan` should work read-only even when no manifest exists. `setup` should write
a manifest only after preview.

### Mixed Format By Design

A mindspace can contain:

- native mdmind maps
- normal Markdown wiki pages
- immutable raw sources
- inbox notes
- generated indexes
- append-only logs
- agent instruction files
- reports and context bundles

The manifest records roles. It does not force every file into one schema.

### Structure Must Earn Its Keep

Use mdmind maps where branch-level structure matters:

- todos
- decisions
- claims and evidence
- research questions
- project memory
- agent handoff
- review queues
- source/staleness reports

Use normal Markdown where prose pages are enough.

### Deterministic First, Agent Judgment Second

`mdm` should provide deterministic checks before asking an LLM to reason about
the workspace. Agents can then fix one concrete category at a time.

### Human Review Is A Feature

Agent writeback should be previewable and reviewable. Generated reports,
candidate links, source-staleness findings, and repair proposals should be
visible artifacts, not hidden model behavior.

### Lightweight Switching Before Layout Management

Mindspace should make many maps feel native without becoming a pane/layout
manager. The first human upgrade is a switcher-and-stack model:

- workspace landing view
- universal map/id/session switcher
- recent-map stack
- pinned working set
- back/forward navigation across file and branch jumps
- cross-map backlinks for the current branch

Named working sets and saved layouts can wait until the workspace model proves
itself.

### Sessions Before Autonomous Agents

mdmind should not expose a broad "AI mode." It should provide durable session
objects that agents and humans can both inspect:

- goal
- role
- target branch
- allowed paths or branches
- context bundle
- proposed edits
- validation results
- review items
- closeout summary

This keeps agent collaboration local, bounded, and reviewable.

## MVP Hypothesis

Users maintaining Markdown knowledge with agents will get immediate value from a
folder-level mdmind workspace if it can:

1. inspect an existing folder without writing anything
2. identify native maps, Markdown pages, sources, inbox files, index/log files,
   and agent instructions
3. lint high-confidence structural problems
4. export a focused context bundle for an agent task
5. open the workspace in the TUI for human review, map switching, and map editing

The MVP should prove that mdmind adds value before building broad ecosystem
packaging or paid-product surfaces.

## MVP Scope

### 1. Mindspace Scan

Command:

```bash
mdm mindspace scan . --json
```

Read-only inventory:

- detected root
- manifest presence
- Markdown file count
- native mdmind map count
- candidate source folders
- candidate inbox folders
- index/log files
- agent instruction files
- Obsidian hints such as `.obsidian/`, wikilinks, frontmatter
- parse/validation status for native maps
- warnings for ambiguous roles

Human output should summarize what was found. JSON output should be stable
enough for agents and tests.

### 2. Mindspace Setup Preview

Commands:

```bash
mdm mindspace setup . --preview
mdm mindspace setup . --write
```

Behavior:

- proposes `.mdmind/mindspace.json`
- explains detected roles
- does not move or rewrite user files
- supports explicit overrides later through manifest edits
- writes only after `--write`

This replaces user-visible adoption profiles.

### 3. Mindspace Lint

Command:

```bash
mdm mindspace lint .
```

Initial deterministic checks:

- invalid mdmind maps
- duplicate branch ids within scoped map files
- broken mdmind relations
- missing local source references
- broken Markdown file links where safe to resolve
- broken Obsidian wikilinks where safe to resolve
- missing index/log files when configured
- inbox files older than a configurable threshold
- oversized catch-all Markdown pages as warnings, not errors

Later checks:

- source hash drift
- source-backed claim staleness
- orphan pages or maps
- index drift
- candidate contradictions

### 4. Focused Context Bundle

Command:

```bash
mdm mindspace context maps/tasks.md#todo/focus --format markdown
```

Behavior:

- starts from a file, branch id, or query
- includes selected child nodes, backlinks, outgoing relations, and source refs
- includes provenance for every included item
- supports size controls
- does not summarize with an LLM
- emits Markdown for human/agent reading and JSON for tooling

### 5. TUI Workspace Entry And Switcher

Command:

```bash
mdmind .
```

First TUI slice:

- if a folder is opened, show a workspace landing state when possible
- list native maps, pinned maps, recent files, open todos, validation warnings, and inbox
  candidates
- search maps, ids, tags, sessions, and review items through a universal
  switcher
- keep a recent-map stack and back/forward navigation across file and branch
  jumps
- show cross-map backlinks for the current branch when scan data is available
- open a native map from the workspace
- keep normal `mdmind file.md` behavior unchanged

The TUI is important because mdmind is not only an automation backend.

### 6. Basic Cross-File References

Mindspace needs path-qualified branch targets early enough for scan, lint,
context bundles, and TUI switching to share one addressing model.

Recommended syntax:

```text
[[decision/api-shape]]
[[maps/decisions.md#decision/api-shape]]
[[rel:implements->maps/decisions.md#decision/api-shape]]
```

Same-file ids remain scoped to the file. Cross-file targets should include a
path plus id. Ordinary Markdown links and Obsidian wikilinks should remain
references unless they are explicitly promoted to mdmind relations.

## Agent Collaboration Scope

Agent sessions should be a v1 feature, not part of the smallest scan/lint MVP.
The PRD includes them because they shape the safety model and future sidecars.

Candidate commands:

```bash
mdm mindspace session start maps/tasks.md#todo/focus --role implementer
mdm mindspace session plan <session-id> --json
mdm mindspace session apply <session-id> --preview
mdm mindspace session submit <session-id>
mdm mindspace review list --json
mdm mindspace review approve <review-id>
mdm mindspace review reject <review-id> --reason "wrong target branch"
mdm mindspace session close <session-id>
```

These commands should create and operate on durable workspace records. They
should not require any particular agent vendor. The reference spec owns the
canonical command namespace.

## State And Safety Model

Mindspace should stay local-first and plain-text compatible. Recommended state
shape:

```text
project-root/
  .mdmind/
    mindspace.json
    inventory.json
    sessions/
    checkpoints/
    reviews/
    locks/
  AGENTS.md
  sources/
  maps/
  index.md
  log.md
```

Recommended safety model:

- source folders are read-only by default
- agent sessions have explicit write scopes
- risky writes create checkpoints first
- batch repair and cross-file relation creation default to review mode
- each session starts from branch or file digests
- if a target changed since session start, the proposal becomes a review item
  instead of silently applying
- trusted instruction files and manifest rules are treated separately from
  untrusted source content

This favors optimistic concurrency with review fallbacks over CRDT-style
collaboration.

## Future Scope

- source manifest with hash tracking and stale synthesis reports
- inbox triage workflow with reviewable move/link/map suggestions
- repair preview and safe write mode for near-miss maps
- agent sessions with roles, scopes, context bundles, review items, and closeout
  summaries
- TUI review queue, named working sets, and optional graph lens
- Obsidian-friendly generated reports and Dataview-compatible metadata
- JSON Canvas or graph export if real users need Obsidian visualization bridges
- MCP or OpenClaw plugin tools for `scan`, `lint`, `context`, `repair`,
  `sources report`, and `setup --preview`
- packaged Hermes/OpenClaw skill bundles once the workflow is validated

## Command Vocabulary

Recommended user-facing verbs:

| Verb | User | Meaning |
| --- | --- | --- |
| `new` | human | create a fresh native mdmind workspace |
| `scan` | human or agent | inspect an existing folder, read-only |
| `setup` | human or explicit agent task | write mindspace configuration after preview |
| `lint` | human or agent | find structural problems |
| `context` | agent or human | export focused context with provenance |
| `open` / `mdmind .` | human | enter the workspace in the TUI |
| `session` | agent or human | create and manage scoped agent collaboration |
| `review` | human first | inspect, approve, or reject proposed writeback |

Avoid leading with `adopt` as a user-facing verb. It is accurate internally but
less clear than `scan` plus `setup`. Fresh creation can be named `new` in user
copy or `init` in CLI if consistency with existing `mdm init` wins.

## Phased Roadmap

### MVP: Mindspace Navigation And Deterministic Context

Target: make a folder understandable and navigable without turning mdmind into a
full notes app.

Scope:

- `.mdmind/mindspace.json`
- `mindspace scan`, `status`, `tree` or equivalent inventory
- basic cross-file branch refs
- `mindspace lint` for high-confidence errors
- `mindspace context` with provenance
- TUI workspace landing
- universal switcher over maps and ids
- recent-map stack and pinned working set

Success: a user can open a folder, discover maps, switch between maps in the
TUI, lint obvious problems, and hand a bounded context bundle to an agent.

### v1: Reviewable Agent Teaming

Target: make human-plus-agent collaboration reliable across several maps.

Scope:

- agent sessions with goal, role, target, and write scope
- review queue with diff preview and rationale
- checkpoint-before-write flows
- generated `index.md` and `log.md`
- source records and stale-source report
- structured JSON envelopes for all mindspace commands

Success: a human can assign scoped work to an agent, review proposed changes,
approve or reject them, and recover from mistakes.

### v2: Ecosystem And Advanced Automation

Target: expose the mature local model to agent runtimes and richer workspace
lenses.

Scope:

- MCP or OpenClaw-compatible protocol adapter
- packaged Hermes/OpenClaw skills or plugins
- named working sets and optional saved layouts
- cross-map graph lens and unlinked-mention review
- richer repair/import workflows
- policy rules for low-risk automatic approvals

Success: external agent frameworks can use mdmind without bypassing local
review, safety, and provenance.

## Success Metrics

Early qualitative signals:

- a user can point `mdm` at an existing Markdown folder and understand what it
  found without reading docs
- an agent can run `scan --json` and choose a safer next command
- lint finds real issues the user cares about
- context bundles reduce the need for agents to read entire folders
- users can keep Obsidian or normal Markdown pages without feeling forced into a
  new notes app
- native mdmind users can open a folder and continue working in the TUI

Possible quantitative checks:

- time to first useful scan under 2 minutes
- zero writes during `scan`
- setup preview is deterministic in tests
- lint returns stable issue codes
- context bundle tests show provenance for every included item
- TUI switcher can find maps and ids from the inventory
- session proposals with stale digests become review items rather than applying
- eval cases show agents choose `mdm mindspace context` or `lint` instead of
  ad hoc folder-wide reads for relevant tasks

## Risks And Open Questions

### Risk: Mindspace Becomes Too Broad

Supporting every Markdown convention can turn mdmind into a generic notes
manager. Mitigation: keep the MVP focused on inventory, native map validation,
high-confidence link/source checks, and context bundles.

### Risk: Users Do Not Want Another Workspace Layer

Obsidian users may only want better prompts or plugins. Mitigation: make
`scan`, `lint`, and reports useful without forcing a full migration.

### Risk: Native mdmind Identity Gets Diluted

If all messaging is about Obsidian and LLM Wiki, mdmind looks like a helper
tool. Mitigation: keep the TUI workspace path first-class and describe mdmind as
outline/map-first.

### Risk: Agent Fixes Become Unsafe

Agents may rewrite too much. Mitigation: deterministic lint first, repair
preview before write, checkpoints before risky operations, and reviewable
reports.

### Risk: Switching Becomes A Full Workspace Manager

It would be tempting to copy Obsidian layouts, panes, and plugin breadth.
Mitigation: ship switcher, recents, pinned set, and back/forward before any
layout manager.

### Risk: Prompt Injection Or Untrusted Sources Corrupt The Workspace

Source material can contain instructions that should not control mdmind.
Mitigation: separate trusted control files from untrusted sources, keep source
folders immutable by default, scope writes through sessions, and add evals for
prompt-injection and over-editing behavior.

### Risk: Large Mindspaces Become Slow

Backlinks, scans, and context bundles can become expensive. Mitigation:
incremental inventory, content digests, and cache invalidation under `.mdmind/`.

### Open Questions

- Is the first lovable workflow agent project memory, research synthesis, or
  Obsidian LLM Wiki maintenance?
- Should native workspace creation be `mdm mindspace new` or an extension of
  existing `mdm init`?
- Which Obsidian conventions should be linted in MVP versus later?
- Does the first TUI workspace slice need map switching only, or also pinned
  working sets?
- Should source staleness be in MVP or the first follow-up?

## Linear Recommendation

Before creating more implementation issues, update the existing roadmap around
this PRD:

- Keep the mindspace spec issue as the product/design anchor.
- Reframe mindspace from "LLM wiki layer" to "folder-level mdmind workspace that
  can stand alone or improve mixed Markdown/LLM wiki folders."
- Replace multiple user-facing adoption profile ideas with one `scan` plus
  `setup` flow.
- Promote scan, lint, context, source/staleness, and TUI workspace entry as the
  core candidate slices.
- Promote TUI map switching from future polish to MVP scope.
- Add a v1 issue for agent sessions, review queue, digest checks, and scoped
  writeback.
- Defer paid product, community, and broad ecosystem packaging until one
  workflow proves clear user value.

Candidate issue changes:

- Update `MDM-33` to define mixed-format mindspace and command vocabulary.
- Update `MDM-35` around `scan` and deterministic `lint`.
- Update `MDM-36` around focused context bundles with provenance.
- Update `MDM-38` as the likely first follow-up for source manifests and stale
  synthesis.
- Update `MDM-42` so TUI workspace entry, universal switching, recents, pinned
  maps, and cross-map backlinks are first-class.
- Create or update an issue for agent sessions and reviewable writeback.
- Defer or merge standalone split-map research once cross-file targets are part
  of the mindspace spec.

## Decision Needed

The main product decision is the first wedge:

1. Native mdmind workspace for agent project memory and structured research
2. Companion to Obsidian/LLM Wiki maintenance
3. A small common core that starts with scan/lint/context and lets the first
   users reveal whether native or companion mode is stronger

Recommendation: choose option 3 for the implementation plan, but message the
product as option 1 plus compatibility:

> mdmind is an outline/map-first local workspace. Mindspace lets it understand a
> folder, whether that folder is a native mdmind workspace or a mixed Markdown
> knowledge base maintained with agents.
