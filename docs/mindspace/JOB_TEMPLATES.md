# Mindspace Job Templates

Mindspace helps people who do not know how to prompt Claude Code, Codex, Hermes,
or another agent from scratch.

The product should not assume that users can already say the perfect thing. A
mindspace gives the agent a predictable local structure, but job templates give
the user a predictable way to ask for useful work.

## Product Promise

A user should be able to start with:

```text
Help me organize this launch folder.
```

and be guided toward a stronger request:

```text
Use the launch planning template. Keep sources read-only, create or update a
roadmap map, link decisions to customer evidence and the PRD, identify blocked
work, and leave risky changes for review in mdmind.
```

The template does three jobs:

- teach the agent what kind of work is being requested
- teach the user what good guidance looks like
- produce a workspace shape that `mdm` and `mdmind` can inspect

Templates are not adoption profiles. They do not say "this folder is an
Obsidian vault" or "this folder is an LLM wiki." They say "this is the job the
user wants done right now." The same mindspace can use different templates over
time.

## Template Anatomy

Every template should be understandable by a human and actionable by an agent.

| Field | Meaning |
| --- | --- |
| `id` | Stable CLI template id, such as `launch-planning`. |
| `name` | Human-readable name. |
| `best_for` | Persona and job fit. |
| `starting_prompt` | Natural-language request a user can copy or adapt. |
| `folder_roles` | Expected map/page/source/inbox/index/log/instruction/report roles. |
| `map_shapes` | Suggested native mdmind maps and durable branch ids. |
| `agent_workflow` | Ordered steps the agent should follow. |
| `mdm_checks` | Deterministic commands the agent should run before claiming success. |
| `write_policy` | What may be edited, generated, appended, or only proposed. |
| `review_surface` | What `mdmind .` should show the human after the work. |
| `success_criteria` | Observable outcomes that prove the job helped. |
| `customization_knobs` | Safe variables a user or agent can tune without inventing a new template. |

The common workflow should stay the same:

1. Scan and lint before writes.
2. Explain the detected workspace roles in plain language.
3. Ask for approval before setup or broad writes.
4. Use the template to propose maps, pages, indexes, and review items.
5. Run deterministic validation after edits.
6. Open or summarize the `mdmind .` review path for the human.

## Common Template Knobs

Most personas can be served by customizing a common template instead of creating
many brittle variants.

| Knob | Options | Why it matters |
| --- | --- | --- |
| Source strictness | `read_only`, `cite_required`, `summary_allowed` | Controls whether the agent may synthesize without explicit evidence. |
| Write mode | `review_only`, `scoped_apply`, `append_only_log` | Controls how much the agent can change without human approval. |
| Structure depth | `light`, `normal`, `detailed` | Keeps small folders from becoming over-modeled. |
| Primary output | `map`, `page`, `index`, `report`, `review_items` | Keeps the job focused. |
| Relation density | `none`, `sparse`, `evidence_heavy` | Prevents link spam while preserving important edges. |
| Context budget | file/branch/detail limits | Keeps agent context bounded and explainable. |
| Review tone | `risks`, `decisions`, `continuity`, `claims`, `handoff` | Shapes what the human sees first in `mdmind .`. |

## Persona Templates

### Priya Planner: Launch Planning

Priya needs a calm operating view over moving work.

Starting prompt:

```text
Use the launch planning template for this folder. Identify maps, docs, inbox,
decisions, customer evidence, and the log. Keep source material read-only. Build
or update a roadmap map with blocked work, open decisions, risks, and next
milestones. Show me the manifest and any risky edits before writing.
```

Default workspace shape:

```text
AGENTS.md
docs/prd.md
maps/roadmap.md
maps/decisions.md
maps/customer-insights.md
inbox/
index.md
log.md
```

Useful map branches:

- `roadmap/current`
- `roadmap/blocked`
- `roadmap/risks`
- `roadmap/milestones`
- `decisions/open`
- `evidence/customer-signals`

Agent workflow:

1. Run `mdm mindspace scan <root> --json` and `mdm mindspace lint <root> --json`.
2. Explain maps, pages, sources, inbox, log, and instructions.
3. Propose setup if no manifest exists.
4. Create or update roadmap, decisions, and customer-insight maps only inside
   the approved write scope.
5. Link roadmap branches to decisions and evidence with sparse relations.
6. Leave ambiguous moves, duplicate notes, or risky rewrites as review items.

`mdmind .` should emphasize:

- current launch status
- blocked branches
- open decisions
- files touched by the latest agent session
- review items needing approval

Success looks like:

- Priya can answer "what matters this week?" without opening six files.
- The agent can produce a status update from branch-addressable context.
- Risky edits are visible before they become part of the plan.

### Mateo Techie: Project Memory And Agent Handoff

Mateo needs exact context and repeatable agent handoffs.

Starting prompt:

```text
Use the project memory template. Start from the current task branch, gather only
linked decisions, API docs, and relevant debugging notes, then propose any
durable memory updates for review. Do not rewrite unrelated project notes.
```

Default workspace shape:

```text
AGENTS.md
maps/tasks.md
maps/decisions.md
docs/api.md
notes/debugging.md
log.md
```

Useful map branches:

- `tasks/current`
- `tasks/blocked`
- `tasks/handoff`
- `decisions/accepted`
- `debugging/known-failures`

Agent workflow:

1. Scan and lint the workspace.
2. Resolve the target branch before reading broad context.
3. Export a bounded context bundle with provenance.
4. Perform the coding or investigation work in the agent's normal project
   surface.
5. Propose memory updates as review items when the durable knowledge changed.
6. Validate edited maps with `mdm validate`.

`mdmind .` should emphasize:

- target task branch
- context bundle contents
- accepted and open decisions
- memory update proposals
- recent handoff notes

Success looks like:

- The agent uses the right branch, not the whole folder.
- Mateo can audit what the agent saw.
- Durable learnings return to the mindspace without silent drift.

### Ren Writer: Story Continuity

Ren needs help maintaining a world without losing authorship.

Starting prompt:

```text
Use the story continuity template. Check this chapter against character, place,
timeline, and theme maps. Do not rewrite the draft. Create review items for
continuity risks and suggest map updates where the story bible is stale.
```

Default workspace shape:

```text
maps/book.md
maps/characters.md
maps/places.md
maps/timeline.md
pages/chapter-08-draft.md
pages/research-notes.md
inbox/
log.md
```

Useful map branches:

- `book/chapters`
- `characters/main`
- `places/active`
- `timeline/current`
- `themes/open`
- `continuity/risks`

Agent workflow:

1. Scan and identify maps versus prose pages.
2. Keep drafts as pages unless Ren explicitly asks to import or rewrite.
3. Gather only linked character, place, timeline, and theme branches.
4. Produce continuity review items with target, rationale, and suggested fix.
5. Propose story-bible map updates separately from draft changes.

`mdmind .` should emphasize:

- chapter branch or draft page
- linked characters and places
- continuity warnings
- proposed story-bible updates
- recent scenes and pinned maps

Success looks like:

- Ren sees risks without the agent flattening the prose voice.
- The story bible becomes easier to maintain.
- Review items feel like editorial suggestions, not file churn.

### Nova Researcher: Claims And Evidence

Nova needs source-grounded synthesis and auditability.

Starting prompt:

```text
Use the claims and evidence template. Keep sources read-only. Build or update a
claims map where every claim links to evidence, open questions, and confidence.
Flag claims with missing evidence or stale sources for review.
```

Default workspace shape:

```text
sources/interviews/
sources/papers/
maps/claims.md
maps/questions.md
wiki/overview.md
index.md
log.md
```

Useful map branches:

- `claims/core`
- `claims/weak-evidence`
- `questions/open`
- `sources/key`
- `synthesis/current`
- `review/stale`

Agent workflow:

1. Scan and lint the folder.
2. Treat `sources/` as read-only and untrusted.
3. Build claims as native map branches with durable ids.
4. Link each claim to source refs or source records.
5. Mark evidence gaps and contradictions as review items.
6. Use source reports when hashes or stale digests exist.

`mdmind .` should emphasize:

- claims by confidence or evidence state
- source previews
- open questions
- stale-source warnings
- review queue for synthesis changes

Success looks like:

- Nova can inspect why a claim exists.
- The agent does not merge source text and trusted instructions.
- Stale or weak evidence becomes visible instead of buried.

## Custom Templates

Custom templates should start from the common template anatomy, not a blank
prompt.

A user or agent can create a custom template by changing:

- the starting prompt
- the expected folder roles
- the map branch skeleton
- write and review policy
- the TUI landing emphasis
- success criteria

Customization should not change the safety baseline:

- scan before setup
- sources read-only by default
- setup preview before manifest writes
- bounded context before agent work
- review items for risky or stale writes
- deterministic validation before closeout

## Implementation Implications

Mindspace templates should appear in three places:

1. **Agent skill references** so Claude Code, Codex, Hermes, or another agent
   can choose and apply the right job template.
2. **`mdm mindspace` helper commands** so scripts and agents can list, inspect,
   and preview templates without scraping docs.
3. **`mdmind .` workspace views** so users can see which job template shaped a
   session, what changed, and what still needs review.

Current helper commands:

```bash
mdm mindspace template list --json
mdm mindspace template show launch-planning --json
mdm mindspace template show launch-planning --prompt
mdm mindspace setup . --template launch-planning --preview
```

Future template-aware helpers:

```bash
mdm mindspace context maps/roadmap.md#roadmap/current --template launch-planning --json
```

Candidate JSON formats:

- `mindspace_template_catalog.v1`
- `mindspace_template.v1`

The first implementation can ship with built-in templates. Later versions can
allow project-local overrides in `.mdmind/templates/` or trusted instruction
files, but local overrides must remain inspectable and separate from raw
sources.
