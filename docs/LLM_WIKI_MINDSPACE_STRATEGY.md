# LLM Wiki Mindspace Strategy

## Goal

Make `mdmind` useful as the structured layer inside an LLM wiki or AI second brain without turning it into a team wiki, a full notes app, or a general document manager.

The product should keep its current center of gravity:

- plain-text maps
- keyboard-first shaping
- stable ids and deep links
- relations and backlinks
- CLI inspection and export
- agent-readable structure

The new bet is a small mindspace layer around those maps.

In this document, **mindspace** means a local folder of mdmind maps, source files, indexes, logs, and safety history that `mdm` can understand as one working context.

## Core Thesis

An LLM wiki needs more than a folder of Markdown files.

It needs:

- raw sources that remain trustworthy
- synthesized pages or maps that can evolve
- an index so humans and agents can find the right material
- a log so the mindspace has memory over time
- stable links across files
- checks that detect drift, missing links, and stale synthesis
- focused context exports so agents do not read the whole note folder

Many people already have a favorite Markdown notes workflow. `mdmind` should fit alongside those workflows instead of trying to replace them.

The opening for `mdmind` is different: become the local-first structure and graph layer that agents can operate safely.

## What mdmind Enables Beyond Existing Workflows

Existing AI second-brain workflows can already combine a Markdown folder, an agent, a rules file, and a set of conventions. That is enough to get started.

The gap is that most of those systems rely on social convention:

- "put notes in this folder"
- "use this filename pattern"
- "remember to link sources"
- "update the index"
- "do not create duplicates"
- "include only the relevant context"

`mdmind` can turn several of those conventions into inspectable product behavior.

### Branch-Level Stable Handles

Most Markdown workflows address files. `mdmind` can address exact branches inside files.

That gives agents durable targets for:

- one decision inside a larger project map
- one claim inside a research synthesis
- one task inside a TODO map
- one person, place, scene, or source branch

Useful shape:

```text
maps/decisions.md#mindspace/source-manifest
maps/research.md#claim/compile-once
maps/tasks.md#task/context-bundle
```

Why it matters:

- agents can update the right branch instead of rewriting a whole note
- humans can deep-link to the exact decision or task
- context bundles can start from a precise node

### A Typed, Validatable Knowledge Graph

Plain Markdown links say "related." `mdmind` relations can say more:

```text
[[rel:supported-by->sources/meeting-2026-05-12.md]]
[[rel:implements->maps/decisions.md#mindspace/links]]
[[rel:blocked-by->maps/tasks.md#import/opml]]
```

Why it matters:

- source, task, decision, and dependency relationships become queryable
- `mdm mindspace lint` can detect missing targets
- backlinks can be derived instead of manually maintained
- agents can follow "why", "blocked by", and "supported by" links differently

### Deterministic Health Checks Before AI Automation

Existing agent workflows often ask the model to inspect a note folder and "find problems." That can be useful, but it is not a reliable foundation.

`mdmind` can provide deterministic checks first:

- unresolved links
- duplicate ids
- missing source files
- orphaned maps
- stale source records
- decisions without evidence
- task branches without status
- generated indexes that are out of date

Why it matters:

- the user can trust the report without trusting a model's judgment
- agents can fix one concrete category at a time
- automation becomes reviewable instead of magical

### Markdown Repair And Normalization

Existing notes workflows involve many editors and many Markdown habits. Some will produce files that are close to mdmind maps but not quite compliant: tabs, inconsistent indentation, heading sections, loose bullets, smart punctuation, copied checklist syntax, or characters that confuse the parser.

`mdmind` can provide a repair layer that turns near-miss Markdown into valid mdmind structure.

Why it matters:

- users can keep using other editors without breaking their maps
- agents can clean up after broad Markdown edits
- imports and pasted notes can become usable faster
- strict validation stays useful because repair exists beside it

The key is to separate three modes:

| Mode | Behavior |
| --- | --- |
| `validate` | Report problems, change nothing |
| `repair --preview` | Show proposed normalization and diagnostics |
| `repair --write` | Apply safe fixes and report what changed |

Safe automatic fixes:

- convert tabs to spaces
- normalize indentation to two-space levels when parent/child intent is clear
- trim trailing whitespace
- normalize checkbox markers
- convert simple heading sections into tree nodes when requested
- move body text under `| detail` lines when requested
- strip or escape unsupported control characters
- normalize repeated blank lines around imported sections

Unsafe fixes should require preview or explicit flags:

- guessing deep hierarchy from ambiguous indentation
- rewriting headings into tree structure
- merging duplicate ids
- deleting unknown tokens
- changing relation targets
- changing source links

### Focused Context Bundles For Agents

Today, agents often read too much or too little. They scan whole folders, miss relevant backlinks, or include stale notes.

`mdmind` can export a bounded context bundle from graph structure:

```bash
mdm mindspace context maps/projects.md#project/mindspace-layer --relations 2 --backlinks
```

The bundle can include:

- the target branch
- relevant children
- incoming backlinks
- selected outgoing relations
- source references
- provenance for each included item
- budget controls for large mindspaces

Why it matters:

- the agent starts better prepared
- the user can inspect exactly what context was included
- context is selected by structure, not only keyword search

### Source-Backed Synthesis And Staleness

LLM wiki workflows depend on a difference between raw sources and maintained synthesis. Existing tools can store both, but they do not usually know when synthesis is stale.

`mdmind` can track:

- source records
- source references from claims and decisions
- source hash or modified time
- last synthesis update
- uncompiled or changed sources

Why it matters:

- the user can see when a source changed after a claim was written
- agents can update synthesis instead of adding one more summary
- knowledge accumulates with provenance

### Reviewable Writeback Surfaces

Agents are good at making changes, but users need to see what changed and why.

`mdmind` can provide review branches:

```text
- Link Review #review [id:review/links]
  - Candidate Relations
    - Context bundle depends on cross-file links [[rel:depends-on->maps/decisions.md#mindspace-links]]
      @status:proposed
```

Why it matters:

- link suggestions become reviewable objects
- task extraction can preserve rationale
- weekly reviews can update durable maps, not only append prose

### The Same Artifact Works For Humans And Agents

The TUI is not just a pretty viewer over generated notes. It is the human workshop for the same structure agents inspect through the CLI.

Why it matters:

- agents can maintain maps with `mdm`
- humans can reshape the same maps in `mdmind`
- the file remains readable in plain Markdown
- there is no hidden database required for the core workflow

## Product Posture

`mdmind` should be a good neighbor to Markdown note folders, not a replacement for every note folder workflow.

Good posture:

- single-file maps stay first-class
- mindspace behavior is optional
- source files are referenced before they are rewritten
- generated indexes are transparent plain text
- the CLI owns scanning, validation, and context packaging
- the TUI remains focused on shaping one map at a time

Poor posture:

- cloning full notes-app panes, plugins, and graph views
- treating every Markdown note as an mdmind map
- requiring global ids on every node
- hiding source provenance behind AI-generated summaries
- making vector search or autonomous agents a prerequisite

## Multi-Map Management Is Required

Single-map `mdmind` is useful inside an AI second-brain workflow, but it is not enough to own the workflow.

People already use ordinary shell tools like `tree`, `find`, `rg`, and folder conventions to orient agents inside Markdown note collections. That is a strong signal: agents need a mindspace map before they can work well.

If `mdmind` does not provide a multi-map layer, users will keep using:

- the filesystem tree as the real mindspace index
- a separate notes app for browsing many Markdown files
- ad hoc `tree` / `rg` output as agent context
- manually maintained map-of-content files
- custom agent prompts to explain where things live

That can work, but it leaves `mdmind` as one file format among many. The product opportunity is to become a semantic `tree` for structured knowledge: not a rich document manager, but a navigable, validatable index of maps, sources, ids, relations, and work surfaces.

The goal is not full document management. The goal is enough multi-map management that a human or agent can answer:

- what maps exist in this mindspace?
- what are the major branches and ids?
- which sources, tasks, decisions, and claims are connected?
- which map should I open for this job?
- what changed since the last index?
- what links are broken?
- what context should I load before working?

### Minimum Viable Multi-Map Layer

The first mindspace version should provide these capabilities:

| Capability | Why It Matters |
| --- | --- |
| Mindspace manifest | Defines which folders and files belong to the mdmind mindspace |
| Map inventory | Replaces raw `tree` output with an agent-readable list of maps, roots, ids, tags, and metadata |
| Cross-map search | Lets users and agents find branches across maps without opening files one by one |
| Cross-file deep links | Lets one map point to an exact branch in another map |
| Mindspace backlinks | Shows incoming references across files, not only inside one map |
| Mindspace lint | Finds missing targets, duplicate ids, stale indexes, and broken source references |
| Markdown repair | Normalizes near-miss Markdown into compliant mdmind maps with previewable changes |
| Generated index | Gives humans a readable mindspace overview and agents a compact orientation file |
| Context bundle export | Produces the focused packet an agent needs for a task |
| TUI map switcher | Lets humans jump between known maps without leaving the mdmind flow |

### What To Avoid

Multi-map management should not become a full notes app.

Avoid:

- rich Markdown preview as the core product
- multi-pane file browsing as the main interface
- generalized file tagging across every Markdown note
- sync, permissions, publishing, or team wiki features
- hiding the mindspace model in an opaque database

The right product shape is closer to:

```bash
mdm mindspace tree
mdm mindspace find "#decision @status:active"
mdm mindspace links
mdm mindspace backlinks maps/decisions.md#mindspace/source-manifest
mdm mindspace lint
mdm mindspace repair --preview
mdm mindspace context maps/projects.md#project/mindspace-layer
```

And in the TUI:

- open mindspace map picker
- jump to id across maps
- follow cross-file relation
- show mindspace backlinks for current branch
- return to previous map/branch

That is enough for `mdmind` to feel like a mindspace brain rather than only a file editor.

## Proposed Mindspace Shape

A starter mindspace could look like this:

```text
research-brain/
  .mdmind/
    mindspace.json
  AGENTS.md
  sources/
    karpathy-llm-wiki.md
    product-interview-001.md
  maps/
    synthesis.md
    decisions.md
    tasks.md
  index.md
  log.md
```

The structure should be flexible. The manifest tells `mdm` which folders matter, but the files remain normal files.

Suggested roles:

| Area | Role |
| --- | --- |
| `.mdmind/mindspace.json` | Mindspace manifest, source folders, map globs, generated file paths, conventions |
| `AGENTS.md` | Short always-loaded instructions for agents |
| `sources/` | Raw source material, transcripts, copied articles, PDFs, web captures, local notes |
| `maps/` | Native mdmind synthesis maps, decisions, task maps, research maps |
| `index.md` | Generated or maintained navigation layer |
| `log.md` | Chronological activity and maintenance notes |

## Command Shape

The first version should be deterministic and CLI-first.

Possible commands:

```bash
mdm mindspace init --name "Research Brain"
mdm mindspace status
mdm mindspace scan --json
mdm mindspace index --write
mdm mindspace links --plain
mdm mindspace relations --json
mdm mindspace lint
mdm mindspace repair --preview
mdm mindspace context maps/synthesis.md#llm-wiki --relations 2 --backlinks
mdm mindspace sources report --stale --json
```

The TUI can later use this mindspace graph, but it should not carry the first implementation.

## File And Link Model

Same-file links already work:

```text
[[decision/mindspace-layer]]
[[rel:informs->decision/mindspace-layer]]
```

Mindspace links should extend that shape with path-qualified targets:

```text
[[maps/synthesis.md#concept/llm-wiki]]
[[rel:informs->maps/decisions.md#decision/mindspace-layer]]
[Karpathy LLM Wiki](https://gist.github.com/karpathy/442a6bf555914893e9891c11519de94f)
```

Resolution rules should be boring and reliable:

- ids are scoped to a file by default
- cross-file targets include a path plus anchor
- relative paths resolve from the current file
- mindspace lint reports missing files, missing anchors, and duplicate ids
- external URLs and non-map files are references, not required relation targets

## Checkpoints And Git

A mindspace should make safety feel built in for standard users while still giving power users access to Git-backed history.

`mdmind` already has local checkpoint concepts for interactive editing. A mindspace should extend that idea across multiple maps, generated indexes, repair reports, and source metadata.

For most users, the product should expose:

- automatic checkpoints before repair, import, source updates, and agent writeback
- named checkpoints for meaningful milestones
- plain-language restore flows
- change summaries before risky operations
- recovery for one map, one branch, or the whole mindspace

Git should sit underneath or beside that model, not in front of it.

The important distinction:

| Layer | Role |
| --- | --- |
| mdmind checkpoint | Fast local safety while shaping maps |
| mindspace checkpoint | Named snapshot of related map, index, and source metadata changes |
| Git commit | Advanced durable history for users who already want Git workflows |

Standard-user commands should avoid Git language:

```bash
mdm mindspace checkpoint create "Before source repair"
mdm mindspace checkpoint list
mdm mindspace checkpoint restore checkpoint-id --preview
mdm mindspace changes
mdm mindspace restore --last
```

Power-user commands can expose Git explicitly:

```bash
mdm mindspace git status
mdm mindspace git commit -m "Update LLM wiki synthesis"
```

Git integration should stay conservative:

- hide Git unless the user asks for it or the mindspace is configured for it
- show plain-language dirty-file summaries before write operations
- never auto-commit without explicit user consent
- include generated index and repair reports in summaries
- make agent writeback auditable through diffs
- work even when a mindspace is not a Git repository

This gives users two kinds of confidence: simple local recovery by default, and durable project history for people who want the power-user path.

## User Journeys

The deeper pattern behind these journeys is:

```text
capture -> classify -> synthesize -> link -> review -> act -> write back
```

The user captures messy material. The agent organizes it. The wiki accumulates structured synthesis. The user reviews and acts. The agent writes back the outcome so the next session starts smarter.

The practical mechanics are usually:

- open the agent in the notes or mindspace folder
- keep always-loaded instructions short
- separate raw sources from maintained synthesis
- treat daily and weekly notes as views, not the source of truth
- use skills or commands for repeated work
- prefer reviewable writeback over invisible rewrites
- build deterministic health checks before autonomous maintenance

### 1. The Coding Agent Power User

This user already lives in a coding agent. They have project notes, TODOs, research snippets, and decisions scattered across chat history and Markdown files.

Today:

- they ask the agent to remember things
- context gets lost between sessions
- TODO files become messy prose
- agents read too much or miss the important branch

With the mindspace layer:

1. They run `mdm mindspace init`.
2. They keep project decisions in `maps/decisions.md`.
3. They keep agent tasks in `maps/tasks.md`.
4. They ask agents to update those maps instead of inventing new note files.
5. Before a session, the agent runs `mdm mindspace context ...`.
6. After a session, the agent updates `log.md` and relevant map branches.

Delight moments:

- The agent finds the exact decision branch by id instead of rereading old chat.
- `mdm mindspace lint` catches a broken relation before the next session.
- A context bundle includes only the relevant branch, backlinks, and source references.
- The user can open the same map in `mdmind` and reshape the work visually or structurally.

### 2. The Researcher Building A Personal LLM Wiki

This user reads papers, articles, transcripts, and notes. They want accumulated synthesis, not repeated summarization.

Today:

- sources pile up
- summaries drift away from evidence
- indexes are manually maintained
- links between claims, questions, and sources are inconsistent

With the mindspace layer:

1. They add raw material under `sources/`.
2. They create synthesis maps under `maps/`.
3. Source references attach to claims and themes.
4. `mdm mindspace sources report --stale` shows what changed or has not been compiled.
5. `mdm mindspace index --write` refreshes the navigation map.

Delight moments:

- A source appears in the stale report before it silently falls out of memory.
- A claim branch shows the exact source files that support it.
- The user can query `#question`, `#claim`, or `@confidence:low` across the mindspace.
- The index feels like it updates itself, but it stays readable as Markdown.

### 3. The Markdown Note Folder User Who Wants Agent-Readable Structure

This user already has a Markdown note folder or notes folder. They do not want another knowledge app. They want a sharper structured layer for agent work.

Today:

- Markdown notes are good for capture and browsing, but agents need conventions
- backlinks exist, but task, source, decision, and relation semantics are uneven
- AI-generated notes can become hard to audit

With the mindspace layer:

1. They keep using their existing notes workflow.
2. They place a few mdmind maps inside or beside the notes folder.
3. `mdm mindspace scan` treats those maps as structured control surfaces.
4. Existing Markdown links remain visible.
5. mdmind ids and relations give agents stable handles.

Delight moments:

- No forced migration.
- An mdmind map can link back to an existing note or source file.
- The agent can use the map as a command center without flattening the whole note folder.
- The user gets structured decisions and tasks without giving up their existing note environment.

### 4. Brain Dump To Action

This user captures quick ideas, meeting notes, tasks, people, half-formed project thoughts, and links throughout the day. They do not want to decide the perfect filing location at capture time.

Today:

- inbox notes become a junk drawer
- agents can create duplicate notes for the same person, project, or concept
- tasks lose their original rationale
- the user cannot tell what was classified, skipped, or changed

With the mindspace layer:

1. The user drops everything into an inbox or daily note.
2. The agent classifies the new material.
3. The agent creates or updates task, project, person, idea, and source branches.
4. Unclear items stay in a review branch.
5. The user reviews the map instead of a wall of generated prose.

Useful shape:

```text
- Inbox Triage #workflow [id:inbox]
  - New Captures #inbox @status:open
  - Classified #done
  - Needs Review #review
  - Created Tasks #todo [[maps/tasks.md#active]]
  - Created Concepts [[maps/synthesis.md#concepts]]
```

Delight moment:

- The agent reports, "I found five new captures, created two tasks, linked one to an existing project, and left two in Needs Review because ownership was unclear."

### 5. Daily Brief That Knows The Real Work

This user starts the morning and wants to know what is overdue, what carried forward, what projects are active, what decisions are waiting, and what needs attention today.

Today:

- daily notes become isolated journals
- tasks are copied forward instead of linked
- decisions and blockers outside the daily note are easy to miss
- the brief can look confident while being incomplete

With the mindspace layer:

1. Durable task and decision maps remain the source of truth.
2. The agent builds a daily working view from those maps.
3. The daily brief links back to task, decision, and source branches.
4. The user adjusts the day without losing the underlying structure.

Useful shape:

```text
- Daily Brief #brief @date:2026-05-12 [id:brief/2026-05-12]
  - Focus
    - [ ] Ship mindspace strategy doc #todo [[maps/tasks.md#mindspace/strategy]]
  - Carry Forward
    - [ ] Resolve cross-file link syntax #todo [[maps/decisions.md#mindspace/links]]
  - Watch
    - Import pipeline still blocks source ingestion [[maps/risks.md#import]]
```

Delight moment:

- The daily brief includes "why this matters" links back to decisions and sources. The user sees the reasoning trail, not just a task list.

### 6. Meeting Notes To Decisions And Follow-Ups

This user has messy meeting notes with decisions, action items, risks, owners, unresolved questions, and links to supporting material.

Today:

- action items get separated from the decision that created them
- owners and deadlines are inconsistently represented
- follow-ups pile up in prose
- later, the user cannot answer "why did we do this?"

With the mindspace layer:

1. Meeting notes stay as source material.
2. Decisions become durable branches.
3. Follow-up tasks link to the decision they implement.
4. The meeting source remains attached as evidence.

Useful shape:

```text
- API Review Meeting #source @date:2026-05-12 [id:source/api-review]
  - Decisions
    - Use path-qualified mindspace links #decision [id:decision/mindspace-links]
      | Keeps cross-file references readable and deterministic.
      [[rel:supported-by->source/api-review]]
  - Follow-Ups
    - [ ] Add validation cases #todo @owner:jason [[rel:implements->decision/mindspace-links]]
```

Delight moment:

- Weeks later, the user asks for the decision and gets the meeting source, rationale, task status, and open questions in one compact bundle.

### 7. Research Ingestion To Living Synthesis

This user adds articles, papers, transcripts, video notes, links, and copied excerpts. They want the wiki to update what it already knows instead of appending another summary.

Today:

- one-source summaries pile up
- old synthesis becomes stale
- contradictions are easy to miss
- source provenance gets weak

With the mindspace layer:

1. Raw sources are registered.
2. Claims and themes live in synthesis maps.
3. Source references attach to claims and decisions.
4. Stale reports show which synthesis may need review.

Useful shape:

```text
- LLM Wiki Research #research [id:research/llm-wiki]
  - Claims #claim
    - Compile once, query later #claim @confidence:high [id:claim/compile-once]
      | A maintained wiki reduces repeated retrieval and repeated summarization.
      [[rel:supported-by->sources/karpathy-llm-wiki.md]]
  - Open Questions #question
    - How should stale synthesis be reported?
  - Contradictions #review
```

Delight moment:

- The mindspace says, "This source changed after the synthesis branch was last touched." That is more valuable than another summary.

### 8. Weekly Review That Compounds Memory

This user wants to review what shipped, what decisions were made, what tasks remain open, what patterns keep repeating, and which projects are stuck.

Today:

- weekly reviews become static summaries
- the same task is rediscovered repeatedly
- patterns are not tied back to project maps
- the user does not know what changed in the underlying knowledge base

With the mindspace layer:

1. The agent reads daily notes, task maps, decision maps, and project maps.
2. It creates a weekly review.
3. It also updates durable maps.
4. It regenerates or checks the mindspace index.

Useful shape:

```text
- Weekly Review #review @week:2026-W20 [id:review/2026-W20]
  - Shipped
  - Decisions Made [[maps/decisions.md#recent]]
  - Still Open [[maps/tasks.md#active]]
  - Repeated Patterns
  - Follow-Up
    - [ ] Promote mindspace spec to next implementation slice #todo
```

Delight moment:

- The review does not just summarize the week. It cleans up the working memory for next week.

### 9. The Product Builder With A Growing Project Brain

This user has roadmap notes, user feedback, design decisions, bugs, and research. They want an agent to help keep the whole project coherent.

Today:

- project context is split between repo docs, issue trackers, chats, and notes
- roadmap decisions lose rationale
- source evidence is separated from tasks

With the mindspace layer:

1. Feedback sources live under `sources/feedback/`.
2. Product maps live under `maps/product/`.
3. Decisions link to evidence and follow-up tasks.
4. Mindspace lint finds orphaned decisions or broken evidence links.
5. Context bundles prepare agents for implementation sessions.

Delight moments:

- "Why did we decide this?" jumps to the branch with rationale and evidence.
- A TODO is linked to the decision and source that created it.
- The context export becomes a clean handoff packet for an agent or contributor.

### 10. Project Brief From Scattered Context

This user is starting or restarting a project. Context is spread across notes, tasks, meeting logs, decisions, and research.

Today:

- project context may be duplicated across notes
- stale decisions can appear current
- briefs may not expose source confidence
- follow-up work is not always linked back

With the mindspace layer:

1. A project map becomes the control surface.
2. Decisions, risks, evidence, and next work are linked branches.
3. The agent generates a project brief from the graph.
4. The user can inspect every included source and branch.

Useful shape:

```text
- Mindspace Layer Project #project @status:active [id:project/mindspace-layer]
  - Goal
  - Current Shape
  - Decisions [[maps/decisions.md#mindspace]]
  - Risks [[maps/risks.md#mindspace]]
  - Evidence [[maps/research.md#research/llm-wiki]]
  - Next Work [[maps/tasks.md#mindspace]]
```

Delight moment:

- The project brief has citations to map nodes and source files, so the user can trust it enough to act.

### 11. Backlink And Relationship Repair

This user has many notes and maps. Some should be linked but are not.

Today:

- link suggestions can be noisy
- agents over-link everything
- relationship type is often missing
- the user cannot distinguish "related" from "depends on" or "supports"

With the mindspace layer:

1. The agent scans recent notes or inbox files.
2. It proposes relation candidates.
3. The user reviews accepted/rejected/proposed states.
4. Accepted relations become durable graph structure.

Useful shape:

```text
- Link Review #review [id:review/links]
  - Candidate Relations
    - Source manifest supports stale report [[rel:supports->maps/decisions.md#source-manifest]]
      @status:proposed
    - Context bundle depends on cross-file links [[rel:depends-on->maps/decisions.md#mindspace-links]]
      @status:accepted
```

Delight moment:

- The user reviews relationship changes as a short decision list instead of approving invisible edits across many files.

### 12. Delegating Work To Agents

This user wants to assign work to one or more agents: write a brief, research a topic, update docs, prepare a meeting, or clean up stale notes.

Today:

- agent work can be hard to audit
- outputs land in inconsistent places
- tasks lose links to artifacts
- multiple agents can collide on the same files

With the mindspace layer:

1. Task nodes carry owner, status, inputs, outputs, and done criteria.
2. Context bundle export gives the agent a bounded handoff.
3. The agent links outputs back to the task.
4. The task branch becomes the audit trail.

Useful shape:

```text
- Agent Delegation #workflow [id:agents/delegation]
  - [ ] Deep-dive mindspace stories #todo @owner:agent @status:active [id:task/story-deep-dive]
    - Inputs
      - Strategy doc [[docs/LLM_WIKI_MINDSPACE_STRATEGY.md]]
      - Research map [[maps/research.md#llm-wiki]]
    - Outputs
      - User story section [[docs/LLM_WIKI_MINDSPACE_STRATEGY.md#user-journeys]]
    - Done When
      - Stories cover capture, review, synthesis, links, and delegation
```

Delight moment:

- The task itself becomes the handoff packet. The agent knows inputs, outputs, status, and done criteria before it starts.

### 13. Personal CRM And People Memory

This user wants to remember people, projects, open promises, preferences, prior context, and previous decisions.

Today:

- person notes become sensitive and require careful handling
- agents may invent or overstate facts
- relationships need provenance
- the user needs a clear review loop

With the mindspace layer:

1. Person branches use conservative metadata.
2. Details point back to meeting sources.
3. Open promises become task branches.
4. Meeting prep can gather backlinks without overstating certainty.

Useful shape:

```text
- People #people [id:people]
  - Mira Patel #person [id:people/mira-patel]
    - Projects
      - Mindspace Layer [[maps/projects.md#project/mindspace-layer]]
    - Open Promises
      - [ ] Send context bundle proposal #todo @status:open
    - Notes
      | Mentioned interest in source provenance during 2026-05-12 meeting.
      [[rel:supported-by->sources/meetings/2026-05-12.md]]
```

Delight moment:

- Before a meeting, the user gets useful context without the system pretending private memory is more certain than it is.

### 14. Knowledge Health Check

This user wants the mindspace to stay healthy.

They need to know:

- what links are broken
- what sources are missing
- what synthesis is stale
- what tasks are orphaned
- what decisions lack evidence
- what indexes need regeneration

Today:

- health checks can become vague
- agents may rewrite too much
- users need reviewable, scoped fixes
- broken structure is hard to see manually

With the mindspace layer:

1. `mdm mindspace lint` reports deterministic structural problems.
2. Source reports show missing, changed, and uncompiled sources.
3. Proposed cleanup can be represented as a map branch.
4. The agent fixes one category at a time.

Useful shape:

```text
- Mindspace Health #report @date:2026-05-12 [id:health/2026-05-12]
  - Broken Links
  - Stale Sources
  - Orphaned Maps
  - Decisions Missing Evidence
  - Proposed Cleanup
```

Delight moment:

- The user can run one command and get a short, actionable report. The agent can fix one category at a time instead of doing a risky rewrite.

### 15. The Writer Or Worldbuilder

This user maintains characters, places, scenes, research, lore, and unresolved questions.

Today:

- notes sprawl across documents
- relationships are hard to track
- agents over-summarize nuance

With the mindspace layer:

1. Character and setting maps live under `maps/world/`.
2. Source notes and excerpts live under `sources/`.
3. Relations connect scenes, places, motifs, and unresolved questions.
4. Context bundles gather just the branches needed for the current chapter or scene.

Delight moments:

- A scene branch can pull backlinks from character, location, and motif maps.
- The user can see unresolved questions without reading the whole world bible.
- The agent gets enough context to help without trampling the structure.

## Delight Principles

### 1. The Mindspace Feels Remembered

The user should feel that work accumulates.

Not because an AI claims to remember, but because the files show it:

- index entries
- log entries
- source records
- stable ids
- linked decisions
- validation reports

### 2. The Agent Seems Better Prepared

The best agent moment is not a flashy response. It is the agent starting with the right context.

The context bundle should feel like a quiet superpower:

- compact
- sourced
- explainable
- deterministic
- easy to inspect

### 3. Broken Knowledge Becomes Visible

Knowledge work breaks in boring ways: stale summaries, missing sources, renamed files, orphaned notes, duplicated concepts.

`mdmind` can make those visible with calm checks:

- unresolved target
- source changed since last index
- branch references missing source
- duplicate id in same file
- orphaned synthesis map

### 4. The User Never Feels Locked In

Every artifact should remain useful outside `mdmind`.

That means:

- Markdown stays readable
- ids and relations remain visible text
- generated files can be regenerated
- source files remain normal files
- users can keep their existing notes workflow

### 5. The TUI Feels Like The Workshop

The mindspace layer should make `mdmind` feel more valuable, not heavier.

The CLI finds and packages context. The TUI is where a human can shape it.

Good future TUI affordances:

- follow a cross-file relation
- open mindspace backlinks for the current node
- jump through mindspace index results
- show source references attached to the current branch

Bad future TUI affordances:

- file browser bloat
- multi-pane document management
- hidden background ingestion
- replacing the shell as the mindspace command surface

## Feature Set

### Phase 1: Mindspace Spec

Deliver:

- mindspace manifest design
- starter folder conventions
- docs for single-map to mindspace migration
- `AGENTS.md` / agent instruction pattern

Result:

- users and agents know where things belong

### Phase 2: Cross-File Graph

Deliver:

- path-qualified deep-link syntax
- cross-file id resolution
- mindspace backlinks
- validation for missing targets and duplicate ids

Result:

- multiple maps can behave like one navigable knowledge graph

### Phase 3: Mindspace Index And Lint

Deliver:

- scan command
- generated index
- relation and reference reports
- lint command
- repair preview for near-miss Markdown
- JSON outputs for agents

Result:

- agents can inspect the mindspace without loading every file
- users can recover from common edits made outside `mdmind`

### Phase 4: Markdown Repair And Normalization

Deliver:

- `mdm repair file.md --preview`
- `mdm repair file.md --write`
- `mdm mindspace repair --preview`
- safe fixes for tabs, whitespace, checkbox markers, control characters, and obvious indentation drift
- explicit flags for heading conversion and body-to-detail conversion
- machine-readable repair reports

Result:

- external editor workflows become forgiving without weakening the mdmind spec

### Phase 5: Context Bundles

Deliver:

- target-based context export
- query-based context export
- provenance in every bundle
- relation/backlink depth controls
- budget controls

Result:

- agents start sessions with focused, auditable context

### Phase 6: Mindspace Checkpoints And Advanced Git

Deliver:

- named mindspace checkpoints
- automatic checkpoints before risky writes
- plain-language change summaries before repair, import, and agent writeback
- previewable restore flows for one map, one branch, or the whole mindspace
- optional power-user Git status and commit commands

Result:

- standard users can trust agent and repair workflows without learning Git
- power users can opt into durable Git history when the mindspace becomes important

### Phase 7: Source Manifest And Staleness

Deliver:

- source registry
- source hash or mtime tracking
- missing and unreferenced source reports
- stale synthesis report

Result:

- raw sources and maintained synthesis stay connected

### Phase 8: Starter Mindspace And Adoption

Deliver:

- example LLM wiki mindspace
- docs quickstart
- agent prompt
- eval candidate for maintaining a mindspace

Result:

- users can feel the workflow in ten minutes

## Agent Operating Model

Recommended division of responsibility:

| Artifact | Role |
| --- | --- |
| `AGENTS.md` | Short rules agents always load |
| mdmind maps | Durable memory, decisions, tasks, synthesis |
| source files | Evidence and raw context |
| generated index | Navigation and discovery |
| generated context bundle | Focused session input |
| Linear or GitHub | External coordination and public execution tracking |

Agents should:

- inspect before editing
- update maps rather than creating random notes
- preserve source links
- validate after changes
- append meaningful activity to the log
- keep summaries tied to evidence

Agents should not:

- overwrite raw sources
- invent ids for every node
- flatten maps into prose
- hide uncertainty
- treat generated indexes as hand-authored truth

## Success Metrics

Useful signals:

- a user can start from one map and grow into a mindspace without migration pain
- a map edited in another Markdown editor can be repaired without hand-fixing every line
- an agent can answer "what context matters for this task?" through `mdm`
- mindspace lint catches real broken links or stale source references
- context bundles are smaller than loading the full mindspace
- examples make the workflow understandable without a long essay
- Markdown note folder users see mdmind as additive to workflows they already like

## Open Questions

- Should the mindspace manifest be JSON, TOML, or a native mdmind map?
- Should `index.md` be generated plain Markdown, a native mdmind map, or both?
- How much of a normal Markdown note folder should `mdm mindspace scan` inspect by default?
- Should source hashes be stored in the manifest, a separate lock file, or generated index metadata?
- What is the minimal TUI support that makes cross-file links feel good without creating a document manager?
- Should mindspace commands live under `mdm mindspace`, or should some become top-level commands later?

## Recommended First Move

Start with the mindspace spec and cross-file resolution.

Those two decisions unlock everything else. If the manifest and target syntax are clean, scan, lint, index, context bundles, and source reports can grow naturally around the existing parser, validator, query, relation, and export primitives.

The first implementation should prove one simple story:

> I have two mdmind maps and one source file. `mdm` can scan them, resolve links between them, tell me what is broken, and export the smallest useful context packet for an agent.
