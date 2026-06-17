# Mindspace Value

Mindspace is valuable when useful knowledge stops fitting in one map but still
needs to stay local, plain, and safe.

The promise is not "mdmind becomes Obsidian." The promise is:

> Point mdmind at a folder and understand what is a map, what is a source, what
> is generated, what is trusted, what changed, what an agent may touch, and what
> needs human review.

## What Users Get

Mindspace turns a loose folder into a dependable workspace:

- maps keep branch-level structure and stable ids
- ordinary Markdown pages can coexist without becoming maps
- raw sources can stay read-only by default
- generated indexes, logs, reports, sessions, reviews, and checkpoints have
  clear roles
- agents can request bounded context instead of reading the whole folder
- humans can review proposed workspace changes before they land
- deterministic `scan`, `lint`, and `context` commands reduce reliance on model
  judgment

The unique value is the combination of local plain files, branch-addressable
maps, deterministic CLI contracts, and reviewable agent collaboration.

## Real-World Examples

### Coding Project Memory

A developer has:

```text
AGENTS.md
maps/tasks.md
maps/decisions.md
docs/api.md
notes/debugging.md
log.md
```

Without Mindspace, an agent may reread too much, miss the relevant decision, or
rewrite the wrong section.

With Mindspace:

```bash
mdm mindspace scan .
mdm mindspace lint .
mdm mindspace context maps/tasks.md#todo/auth-retry --relations 2 --backlinks
```

The agent receives the task branch, related decisions, backlinks, source docs,
and provenance. The human can inspect exactly what context was provided.

Unique value: the target is a branch such as
`maps/tasks.md#todo/auth-retry`, not a vague instruction to read a notes folder.

### Research Folder

A researcher has:

```text
sources/interviews/
sources/papers/
maps/claims.md
maps/questions.md
wiki/overview.md
index.md
```

Mindspace can distinguish raw sources from synthesis maps and ordinary prose.
Later source records can report that a source changed after a claim was last
reviewed.

Value: the workspace can answer, "Which claims may be stale because their
source changed?"

Unique value: source-backed structure without a hidden database or a required
notes app migration.

### Product Planning

A founder or PM has:

```text
maps/roadmap.md
maps/customer-insights.md
maps/decisions.md
docs/prd.md
log.md
```

Mindspace lets `mdmind .` become a calm workspace entrypoint: open the roadmap,
jump to a decision, inspect related evidence, and return through recent maps.

Value: product memory becomes navigable instead of scattered across docs, chat
transcripts, and todo files.

Unique value: the TUI stays a map workshop, but gains workspace switching,
cross-map backlinks, pinned maps, and review surfaces.

### Agent Review Queue

An agent proposes to:

- add a relation between a task and a decision
- move stale inbox notes into a map
- update a synthesis branch
- repair broken links

Mindspace can turn those proposals into review items instead of directly
mutating files.

Value: the human sees the target, rationale, diff, validation state, and stale
digest status before accepting or rejecting.

Unique value: agent collaboration becomes a local, inspectable workflow rather
than "the chat changed my files."

## The Wedge

Mindspace is for the moment when a single map becomes a working knowledge base.

It should help users say:

> Here is my folder. Understand its roles, validate the structure, package the
> right context, and keep agent edits reviewable.

That wedge is local-first workspace understanding plus agent-safe maintenance,
built around plain text and structured maps.
