# Mindspace User Stories

These stories show the target Mindspace experience through the four avatar
personas in `docs/art/`.

They are intentionally concrete. Mindspace should not feel like a vague
"workspace for notes." It should feel like a quiet local workshop where people
can ask an agent to organize real files, then use `mdmind` to understand the
folder, move by branch-level meaning, and trust what changed.

Implementation note: these are product stories for the Mindspace arc. Core
relation target classification and path-qualified branch validation exist now.
Folder scan, setup, context bundles, workspace landing, sessions, reviews, and
source reports remain staged in the Mindspace roadmap unless marked as current
single-map behavior.

Experience note: these stories lead with agent requests because that is the
intended user habit. The `mdm mindspace ...` commands are still shown where they
matter, but as the deterministic substrate an agent, script, or power user can
audit. Persona/job templates make those requests easier to give; see
[JOB_TEMPLATES.md](JOB_TEMPLATES.md) and
[EXPERIENCE_MODEL.md](EXPERIENCE_MODEL.md).

## Cast

| Persona | Avatar | Primary interaction | Mindspace promise |
| --- | --- | --- | --- |
| Priya Planner | <img src="../art/priya-planner.png" alt="Priya Planner" width="120"> | Ask an agent to organize launch work; review and edit in `mdmind .`. | "Show me what matters, what is blocked, what changed, and what needs review." |
| Mateo Techie | <img src="../art/mateo-techie.png" alt="Mateo Techie" width="120"> | Drive Codex, Claude Code, Hermes, scripts, and `mdm` directly when useful. | "Give agents exact branch context and make cross-file references verifiable." |
| Ren Writer | <img src="../art/ren-writer.png" alt="Ren Writer" width="120"> | Ask an assistant editor for continuity passes; keep authorship in `mdmind .`. | "Let the tree carry the draft, while sources, scenes, and references stay close." |
| Nova Researcher | <img src="../art/nova-researcher.png" alt="Nova Researcher" width="120"> | Ask an agent for source-grounded synthesis; audit claims in `mdmind .`. | "Keep sources read-only, claims branch-addressable, and context packets cited." |

## Story 1: Priya Turns A Launch Folder Into A Calm Operating Room

<img src="../art/priya-planner.png" alt="Priya Planner" width="260">

Priya is holding a clipboard covered in flags, a notebook with constellation
lines, and a field bag full of sticky notes. That is her actual working life:
roadmap decisions in one file, customer insights in another, launch notes in a
third, and a small graveyard of "final-final" planning docs.

Her folder starts like this:

```text
launch-q3/
  AGENTS.md
  docs/prd.md
  docs/launch-email.md
  maps/roadmap.md
  maps/customer-insights.md
  maps/decisions.md
  inbox/
  log.md
```

### 0 Minutes In: The Familiar Fog

Priya opens the folder because the launch review starts in an hour.

What Priya is thinking:

> I know the answer is in here. I also know I am about to open six files and
> reconstruct the whole story in my head.

Narrator insight:

Priya does not need another place to put notes. She needs a way to see the
roles her files are already playing. Her first relief should come before any
write happens, even if the discovery is being driven by an agent.

What Priya asks Claude Code or Codex:

```text
Use the launch planning template. Organize this launch folder as a mindspace.
Identify the maps, docs, inbox, log, and trusted instructions. Do not write
anything yet; show me the proposed model first.
```

Under the hood:

```bash
mdm mindspace scan launch-q3
```

Mindspace reads without writing. It identifies native maps, ordinary Markdown
pages, the inbox folder, the log, and trusted local instructions.

Delight:

The output does not say "17 Markdown files." It says something closer to:

```text
Mindspace scan: launch-q3

Maps
  maps/roadmap.md              42 ids, 18 open tasks, 3 warnings
  maps/customer-insights.md    31 ids, 12 source refs
  maps/decisions.md            19 ids, 2 unresolved relation targets

Pages
  docs/prd.md
  docs/launch-email.md

Workspace
  inbox/                       7 items
  log.md                       append-oriented
  AGENTS.md                    trusted instruction
```

Priya's first aha:

> It knows what kind of mess this is.

### 8 Minutes In: Setup Feels Like Consent, Not Capture

The agent proposes:

```bash
mdm mindspace setup launch-q3 --preview
```

Mindspace proposes a small `.mdmind/mindspace.json`, but does not write it.

What Priya is thinking:

> Good. It is not taking over the folder. It is asking me whether this model is
> right.

Narrator insight:

This is the moment Mindspace distinguishes itself from a vault migration. Priya
can inspect, correct, and approve roles. The product earns trust by pausing
before it writes.

The preview calls out:

- maps are editable mdmind maps
- docs are ordinary Markdown pages
- inbox is triage material
- sources, if configured later, default read-only
- generated reports and logs are explicit roles

Priya tells the agent:

> Yes, write only the manifest. Do not move any files yet.

The agent applies:

```bash
mdm mindspace setup launch-q3 --write
```

The only write is `.mdmind/mindspace.json`.

### 20 Minutes In: Branches Replace Vibes

Priya opens the human surface:

```bash
cd launch-q3
mdmind .
```

The workspace landing shows:

- current roadmap branch
- open launch blockers
- recent maps
- unresolved cross-file refs
- inbox items older than 14 days
- one recommended next action: validate relations before the review

Priya searches the switcher for "pricing".

Instead of a generic file list, she sees branch-level hits:

```text
maps/roadmap.md#launch/pricing
maps/decisions.md#decision/pricing-packaging
maps/customer-insights.md#segment/self-serve/pricing-objection
docs/prd.md#Pricing
```

What Priya is thinking:

> This is how my brain names the work. Not "file 14." The actual branch.

Narrator insight:

The tree remains the mental model. Mindspace adds folder reach, but the unit of
meaning is still the branch.

### 35 Minutes In: The Aha Is A Cross-File Backlink

Priya is focused on:

```text
maps/roadmap.md#launch/pricing
```

The current branch has:

```text
- Pricing launch #todo @status:blocked [id:launch/pricing]
  [[rel:blocked-by->maps/decisions.md#decision/pricing-packaging]]
  [[maps/customer-insights.md#segment/self-serve/pricing-objection]]
```

Current core proof:

```bash
mdm relations maps/roadmap.md --plain
mdm validate maps/roadmap.md
```

`mdm relations` classifies the cross-file targets as
`path_qualified_branch`. `mdm validate` can check that
`maps/decisions.md#decision/pricing-packaging` exists.

What Priya is thinking:

> I can point the launch task at the exact decision and the exact customer
> evidence. I do not have to duplicate either one.

Narrator insight:

Priya's delight is not a graph visualization. It is the disappearance of
reconstruction work. The relationship is explicit, readable in plain Markdown,
and checkable by a deterministic tool.

### 52 Minutes In: Agent Help Becomes Reviewable

Priya asks an agent:

> Prepare the launch review packet for the pricing branch. Use linked decisions,
> customer insight branches, and the relevant PRD headings. Leave proposed file
> changes for review.

Under the hood:

```bash
mdm mindspace context maps/roadmap.md#launch/pricing \
  --relations 2 \
  --backlinks \
  --max-files 8
```

The packet includes:

- the pricing launch branch
- the blocking decision branch
- customer-insight backlinks
- relevant PRD headings
- provenance for each included item
- omission notes for everything left out

What Priya is thinking:

> Now I can ask for help without wondering whether the agent read the wrong
> docs or too much of the folder.

The agent proposes two edits:

- add a missing relation from the PRD pricing section to the launch branch
- move an inbox note into `maps/customer-insights.md`

Mindspace creates review items instead of silently applying them.

Delight:

Priya sees the target branch, rationale, diff, validation result, and whether
the target changed since the agent read it.

Final aha:

> The folder became a workspace, but it still feels like mine.

## Story 2: Mateo Gives A Coding Agent Exact Memory Without Giving It The Whole Repo

<img src="../art/mateo-techie.png" alt="Mateo Techie" width="260">

Mateo has a small terminal device in one hand and a wired graph pack on his
back. He likes systems that can be inspected, scripted, and trusted under
pressure. His problem is not "remembering notes." His problem is that coding
agents keep missing local decisions.

His repo memory folder:

```text
project-memory/
  AGENTS.md
  maps/tasks.md
  maps/decisions.md
  maps/debugging.md
  docs/api.md
  docs/release-checklist.md
  log.md
```

### 0 Minutes In: Agent Context Is Too Wide Or Too Narrow

Mateo has a bug:

```text
maps/tasks.md#todo/auth-retry
```

What Mateo is thinking:

> If I tell the agent to inspect the repo, it will read too much. If I point it
> at the task, it will miss the decision that explains why retries are weird.

Narrator insight:

Mateo needs a branch-addressable memory layer. The exact unit matters because
technical work fails when context is approximate.

He writes:

```text
- Retry failed auth refresh #todo @status:active [id:todo/auth-retry]
  [[rel:depends-on->maps/decisions.md#decision/auth-token-model]]
  [API auth](../docs/api.md#Authentication)
```

Current core proof:

```bash
mdm validate maps/tasks.md
mdm relations maps/tasks.md --plain
```

The relation row says:

```text
out  ...  depends-on  path_qualified_branch  maps/decisions.md#decision/auth-token-model
```

Mateo's first aha:

> This is not a wiki link pretending to be a task dependency. It is a typed
> relation I can validate.

### 6 Minutes In: The Tool Catches The Wrong Branch

Mateo mistypes:

```text
[[rel:depends-on->maps/decisions.md#decision/auth-tokens]]
```

Mateo or the agent runs:

```bash
mdm validate maps/tasks.md
```

Mindspace-aware validation reports that the target file exists, but the branch
id does not.

What Mateo is thinking:

> That would have wasted an agent turn. Nice.

Narrator insight:

This is a tiny delight, but it is foundational. The product prevents false
confidence before any large workspace automation exists.

### 20 Minutes In: Context Bundle Instead Of Folder Dump

What Mateo asks the agent:

```text
Work on maps/tasks.md#todo/auth-retry. Use linked decisions and API docs only.
If you update project memory, submit the changes as review items.
```

Under the hood:

```bash
mdm mindspace context maps/tasks.md#todo/auth-retry \
  --relations 2 \
  --backlinks \
  --include-source-refs \
  --format markdown
```

The context bundle includes:

- the task branch and child notes
- the token-model decision branch
- backlinks from debugging notes
- `docs/api.md#Authentication` as a page reference
- `AGENTS.md` because it is trusted local guidance
- a budget summary

It excludes:

- unrelated release checklist branches
- old debugging notes not linked to this task
- source files not referenced by included branches

What Mateo is thinking:

> This is the first context packet I could paste into an agent and then audit
> later.

Narrator insight:

Mateo's delight is not that the agent is magical. It is that the agent's
working memory becomes deterministic enough to inspect.

### 38 Minutes In: Review Queue Turns Agent Work Into Engineering Workflow

The agent proposes:

- update `maps/tasks.md#todo/auth-retry`
- add a new relation to `maps/debugging.md#trace/token-expiry`
- append a run note to `log.md`

Mindspace turns the proposed map edits into a session record and review items.

Future substrate commands:

```bash
mdm mindspace session submit <session-id>
mdm mindspace review list --json
mdm mindspace review approve <review-id>
```

Mateo sees:

```text
Review: add relation
Target: maps/tasks.md#todo/auth-retry
Rationale: debugging trace explains the token expiry condition
Validation: passes
Digest: current
```

What Mateo is thinking:

> This feels like code review for knowledge work.

Final aha:

> Agents can help maintain project memory without becoming the source of truth.

## Story 3: Ren Builds A Living Story World Without Losing The Draft

<img src="../art/ren-writer.png" alt="Ren Writer" width="260">

Ren carries maps, notebooks, loose cards, and a pencil. Their worldbuilding
folder is beautiful and dangerous: every idea connects to every other idea, and
that is exactly how the work can become impossible to write.

Their folder:

```text
novel/
  maps/book.md
  maps/characters.md
  maps/places.md
  maps/themes.md
  pages/chapter-08-draft.md
  pages/research-notes.md
  inbox/
  log.md
```

### 0 Minutes In: The Vault Is Too Big For The Scene

Ren is revising Chapter 8. They need the reunion scene, two character arcs, and
one city detail. The rest of the world should stay nearby but quiet.

What Ren is thinking:

> I need the world to remember itself without dragging the whole world into the
> scene.

Narrator insight:

Ren does not need a generic file graph. They need a writing surface where the
tree protects focus and relations preserve meaning across maps.

The scene branch:

```text
- Chapter 8 Reunion #chapter @status:revision [id:chapter/08/reunion]
  [[maps/characters.md#character/iona/wound]]
  [[rel:set-in->maps/places.md#city/glass-market]]
  [[rel:echoes->maps/themes.md#theme/debt-and-mercy]]
```

### 12 Minutes In: Relations Become Creative Handles

Ren asks the TUI or agent to inspect the relation targets. Under the hood:

```bash
mdm relations maps/book.md#chapter/08/reunion --plain
```

They see outgoing relation targets classified as path-qualified branches. Same
file backlinks still work for branches inside `maps/book.md`; cross-map
backlinks wait for Mindspace inventory.

What Ren is thinking:

> The relation words are mine: set-in, echoes. The structure is helping, not
> flattening the story into software categories.

Narrator insight:

This is where mdmind's plain-text taste matters. A relation should be readable
in a manuscript-adjacent file, not feel like database syntax.

### 25 Minutes In: Peek Before Open Protects Flow

Target future TUI behavior:

Ren highlights:

```text
maps/places.md#city/glass-market
```

They press a peek action. The TUI shows:

```text
Glass Market
maps/places.md#city/glass-market

Preview:
  A covered market built over old canal locks.
  Bright glass awnings. Debt notices nailed to blue doors.

Relations:
  incoming from chapter/08/reunion
  echoes theme/debt-and-mercy
```

What Ren is thinking:

> I got the detail I needed without leaving the scene.

Narrator insight:

The delight is restraint. Mindspace helps Ren touch another file without
turning the session into file browsing.

### 42 Minutes In: Working Set Becomes A Writer's Desk

Ren opens:

```bash
cd novel
mdmind .
```

The workspace landing shows:

- current draft branch
- pinned maps: Book, Characters, Places
- recent branch: `chapter/08/reunion`
- open revision tasks
- unresolved story relations
- inbox captures from the last writing sprint

What Ren is thinking:

> This feels like my desk after I lay out the right cards.

Narrator insight:

A writer's workspace is not an exhaustive index. It is a remembered working
set. Mindspace should privilege recents, pins, and the current creative thread.

### 58 Minutes In: The Agent Is An Assistant Editor, Not A Coauthor

Ren asks an agent:

> Find continuity risks for Chapter 8 using the linked character, place, and
> theme branches. Do not rewrite the draft.

Under the hood:

```bash
mdm mindspace context maps/book.md#chapter/08/reunion \
  --relations 1 \
  --backlinks \
  --max-branches 12
```

The agent returns review items:

- "Iona's wound is described on the wrong side in one detail."
- "Glass Market curfew conflicts with the Chapter 7 log."
- "The debt-and-mercy theme is present, but not tied to the choice at the end."

No draft file changes are applied automatically.

What Ren is thinking:

> It found the threads. It did not grab the pen.

Final aha:

> Mindspace lets the story world be richly connected while the draft remains
> protected.

## Story 4: Nova Turns A Research Folder Into Source-Grounded Memory

<img src="../art/nova-researcher.png" alt="Nova Researcher" width="260">

Nova has a magnifying glass, blueprints, clipped notes, and a bag of carefully
tagged evidence. Her work fails if synthesis loses contact with sources. She
needs to know not only what a claim says, but why it was trusted.

Her folder:

```text
research-brief/
  sources/interviews/
  sources/papers/
  maps/questions.md
  maps/claims.md
  maps/synthesis.md
  pages/overview.md
  index.md
  log.md
```

### 0 Minutes In: Source Anxiety

Nova has 14 interview notes, 9 PDFs, and a synthesis map written over several
weeks.

What Nova is thinking:

> Which claims are still grounded? Which ones were written before the latest
> interview? What did I actually read when I made that conclusion?

Narrator insight:

Nova's trust problem is temporal and evidentiary. Mindspace must make sources
visible without turning raw sources into editable map content.

### 10 Minutes In: Roles Change The Mood

What Nova asks an agent:

```text
Inspect this research folder. Keep sources read-only, identify synthesis maps
and ordinary pages, and tell me what should be reviewed before you write
anything.
```

Under the hood:

```bash
mdm mindspace scan research-brief
```

Mindspace separates:

- native maps for questions, claims, and synthesis
- ordinary Markdown overview
- source folders
- index and log
- future generated reports

What Nova is thinking:

> It knows the difference between evidence and my interpretation of evidence.

Narrator insight:

This is the source-read-only aha. The product is not just indexing files; it is
assigning safety posture.

### 22 Minutes In: A Claim Becomes Addressable

Nova's claim branch:

```text
- Teams trust local tools when review is visible #claim @status:draft [id:claim/local-review-trust]
  [Interview A](../sources/interviews/a.md)
  [Interview D](../sources/interviews/d.md)
  [[rel:supported-by->maps/questions.md#question/review-friction]]
```

Current core behavior:

```bash
mdm refs maps/claims.md --plain
mdm relations maps/claims.md --plain
mdm validate maps/claims.md
```

She can inspect source references separately from semantic relations. The
relation to a question branch is same-file or path-qualified depending on where
the question lives.

What Nova is thinking:

> Links are evidence. Relations are reasoning. I can see both.

Narrator insight:

That distinction is a core Mindspace value: references point to material;
relations explain structure.

### 40 Minutes In: Context With Provenance Beats A Summary

Under the hood:

```bash
mdm mindspace context maps/claims.md#claim/local-review-trust \
  --include-source-refs \
  --backlinks \
  --max-detail-chars 4000
```

The context bundle says:

```text
Included
  maps/claims.md#claim/local-review-trust
    reason: target branch
  maps/questions.md#question/review-friction
    reason: outgoing relation supported-by
  sources/interviews/a.md
    reason: source reference from target branch
  sources/interviews/d.md
    reason: source reference from target branch

Omitted
  sources/papers/local-first-tools.pdf
    reason: source budget
```

What Nova is thinking:

> I do not just know the answer. I know what the answer was allowed to see.

Narrator insight:

For Nova, delight is auditability. Context without provenance is just another
opaque summary.

### 70 Minutes In: Staleness Becomes Review, Not Panic

Under the hood:

```bash
mdm mindspace lint research-brief
```

The stale-source report says:

```text
Changed source
  sources/interviews/d.md

Affected branches
  maps/claims.md#claim/local-review-trust
  maps/synthesis.md#section/adoption-trust

Suggested review
  Re-check claim wording against updated interview notes.
```

What Nova is thinking:

> This is the difference between a pile of notes and a research system.

Narrator insight:

Mindspace should not rewrite Nova's synthesis automatically. It should create
the right review surface and preserve the chain of trust.

Final aha:

> My claims can age visibly instead of silently.

## Cross-Persona Delight Principles

These stories point to the same design spine:

| Moment | What the user feels | Product behavior |
| --- | --- | --- |
| Agent request before commands | "I can ask for the workspace I want." | Agent skills translate natural-language work into deterministic Mindspace operations. |
| Inspection before setup | "It understands the folder without taking it over." | `scan` is read-only; `setup --preview` comes before writes. |
| Branch-level targeting | "This points at the actual idea, not just the file." | Stable ids and path-qualified branch refs. |
| Role separation | "Sources, maps, pages, logs, and reviews are different kinds of things." | Manifest roles and role-aware navigation. |
| Bounded context | "The agent saw the right slice, and I can inspect why." | Context bundles with provenance, budgets, and omission reasons. |
| Reviewable writes | "Help can arrive without surprise edits." | Sessions, reviews, checkpoints, and stale-digest fallback. |
| Calm navigation | "I can move across files and come back." | One active map, peek/open, switcher, recents, and pins. |

## Product Tests These Stories Imply

The stories become useful only if future slices can prove them:

- Priya test: ask an agent to organize a mixed planning folder, preview a
  manifest, open a roadmap branch in `mdmind .`, and show cross-file relation
  diagnostics.
- Mateo test: classify and validate `maps/decisions.md#decision/auth-token-model`
  from a task map, then have an agent produce a bounded context packet with a
  trusted `AGENTS.md`.
- Ren test: peek a cross-file branch without switching the active map, then
  open it and return to the previous branch.
- Nova test: ask an agent for source-grounded synthesis, include source refs
  with provenance, flag changed sources, and turn stale synthesis into review
  items rather than automatic edits.

Each test should include the user's visible moment of relief. If a slice cannot
produce one of those moments, it is probably infrastructure, not product value.
