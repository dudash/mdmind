# Mindspace Value

Mindspace is valuable when useful knowledge stops fitting in one map, but still
needs to stay local, plain, agent-editable, and human-reviewable.

The promise is not "mdmind becomes Obsidian" and it is not "learn a new CLI."
The promise is:

> Ask your agent to organize the folder. Use `mdmind` to inspect, edit, navigate,
> and review the result. Trust `mdm` to keep the agent honest underneath.

## What Users Get

Mindspace turns a loose folder into a dependable agent-maintained workspace:

- users delegate organization, synthesis, linking, and cleanup in natural
  language
- agents can create native maps, ordinary Markdown pages, generated indexes,
  logs, reports, sessions, and review items
- maps keep branch-level structure and stable ids
- ordinary Markdown pages can coexist without becoming maps
- raw sources can stay read-only by default
- agents can request bounded context instead of reading the whole folder
- humans can inspect and edit the structured result in `mdmind .`
- risky writes become reviewable before they land
- deterministic scan, lint, context, session, and review commands reduce
  reliance on model judgment

The unique value is the combination of agent-native work, local plain files,
branch-addressable maps, deterministic contracts, and human review in the TUI.

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

They tell Codex or Claude Code:

```text
Use this memory folder while fixing auth retry. Read only the task branch,
linked decisions, and API docs you need. If you learn something durable, propose
an update to the memory map for review.
```

Mindspace gives the agent a branch target such as
`maps/tasks.md#todo/auth-retry`, then lets it gather linked decisions, docs,
and backlinks with provenance.

The human opens `mdmind .` to inspect the task branch, see what context the
agent used, and approve or reject proposed memory edits.

Unique value: the agent works from exact branch context instead of a vague
instruction to read a notes folder.

### Research Folder

A researcher has:

```text
sources/interviews/
sources/papers/
maps/claims.md
maps/questions.md
wiki/overview.md
index.md
log.md
```

They tell an agent:

```text
Build a claims map from these interview notes. Keep sources read-only. Link
each claim to evidence and mark anything that needs follow-up.
```

Mindspace separates raw sources from synthesis maps and ordinary prose. Later
source records can report that a source changed after a claim was last reviewed.

The human opens `mdmind .` to review claims, source links, stale-source reports,
and proposed follow-up branches.

Unique value: source-backed structure without a hidden database, a required
notes-app migration, or blind trust in an agent summary.

### Product Planning

A founder or PM has:

```text
maps/roadmap.md
maps/customer-insights.md
maps/decisions.md
docs/prd.md
log.md
```

They tell Claude, Codex, or Hermes:

```text
Turn this launch folder into a workspace. Link roadmap branches to decisions,
customer evidence, and PRD sections. Create review items for anything that
looks risky or ambiguous.
```

Mindspace lets the agent organize and connect the folder, while `mdmind .`
becomes the calm workspace entrypoint: open the roadmap, jump to a decision,
peek at evidence, and return through recent branches.

Unique value: product memory becomes navigable and reviewable instead of being
scattered across docs, chat transcripts, and todo files.

### Writing And Worldbuilding

A writer has:

```text
maps/book.md
maps/characters.md
maps/places.md
pages/chapter-08-draft.md
pages/research-notes.md
inbox/
```

They tell an agent:

```text
Find continuity risks for Chapter 8 using the linked character, place, and
theme branches. Do not rewrite the draft. Leave suggested fixes as review
items.
```

Mindspace lets the agent traverse only the relevant map branches and pages. The
writer uses `mdmind .` to peek across files, edit the outline, and accept only
the suggestions that preserve the voice of the work.

Unique value: the agent can help maintain the world without becoming the
coauthor or flattening the project into a generic vault.

### Agent Review Queue

An agent proposes to:

- add a relation between a task and a decision
- generate a new synthesis page from sources
- move stale inbox notes into a map
- update a branch after source changes
- repair broken links

Mindspace can turn those proposals into review items instead of directly
mutating files.

Value: the human sees the target, rationale, diff, validation state, source
provenance, and stale digest status before accepting or rejecting.

Unique value: agent collaboration becomes a local, inspectable workflow rather
than "the chat changed my files."

## The Wedge

Mindspace is for the moment when a single map becomes a working knowledge base
that an agent can help maintain.

It should help users say:

> Here is my folder. Organize it, link it, generate the missing maps or pages,
> use only the context you need, and show me the proposed changes in `mdmind`
> before they become part of my workspace.

That wedge is agent-native local knowledge work, grounded in plain text and
structured maps.
