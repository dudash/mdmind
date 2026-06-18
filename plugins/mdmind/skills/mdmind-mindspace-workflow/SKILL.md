---
name: mdmind-mindspace-workflow
description: Guide agent-run folder-level Mindspace work for mdmind. Use when a user asks Claude Code, Codex, Hermes, or another agent to organize, inspect, link, synthesize, clean up, maintain, or review a local folder workspace with persona/job templates, mdm mindspace scan/lint, setup preview, bounded context, safe writes, and mdmind human review. Do not use for ordinary single-map authoring, one-off mdm CLI inspection, or short prose answers that do not need a folder-level workspace.
---

# mdmind Mindspace Workflow

Use this skill to turn vague folder-level requests into safe Mindspace jobs.

Mindspace is agent-native: the user talks to an agent, the agent uses `mdm
mindspace ...` for deterministic facts, and the human reviews or edits in
`mdmind .`. The CLI is the contract layer, not the primary user habit.

## Core Sequence

1. Name the job template that best fits the request, or ask one concise question
   if the job is ambiguous.
2. Run `mdm mindspace scan <root> --json` before proposing setup or writes.
3. Run `mdm mindspace lint <root> --json` before claiming the workspace is clean.
4. Explain detected maps, pages, sources, inbox, indexes, logs, instructions,
   and reports in plain language.
5. Preview setup or risky changes before writing. A template is guidance, not
   permission.
6. Keep sources read-only by default and keep raw source content separate from
   trusted instructions.
7. Use `mdmind-map-authoring` when creating or revising native maps.
8. Use `mdm-cli-inspection` when validating, querying, deep-linking, or
   exporting maps.
9. Validate changed maps and summarize what the human should inspect in
   `mdmind .`.

## Read References As Needed

- Read `references/workflow.md` for the full operating loop, vague prompt
  sharpening, setup, future context/session placeholders, and handoff language.
- Read `references/safety.md` before any setup, file move, generated artifact,
  broad rewrite, or source-backed synthesis.
- Read `references/templates.md` to choose a template, customize common knobs,
  or inspect the current built-in helper commands.
- Read `references/priya-launch-planning.md` for launch planning, roadmaps,
  decisions, customer evidence, blocked work, and status review.
- Read `references/mateo-project-memory.md` for project memory, bounded context,
  coding-agent handoff, decisions, and durable memory proposals.
- Read `references/ren-story-continuity.md` for story continuity, draft
  protection, character/place/timeline checks, and editorial review items.
- Read `references/nova-claims-evidence.md` for claims maps, source-backed
  synthesis, evidence links, confidence, open questions, and stale evidence.

## Template Selection

Use built-in helpers when available:

```bash
mdm mindspace template list --json
mdm mindspace template show <template-id> --json
mdm mindspace template show <template-id> --prompt
```

Current built-in ids:

- `launch-planning`
- `project-memory`
- `story-continuity`
- `claims-evidence`

If the exact persona is not listed, start from the closest template and tune the
common knobs: source strictness, write mode, structure depth, primary output,
relation density, context budget, and review tone.

## Missing Helpers

Some Mindspace commands are planned but may not exist yet. Do not invent output
from missing helpers. Degrade to current references and implemented commands.

Current helpers:

```bash
mdm mindspace scan <root> --json
mdm mindspace lint <root> --json
mdm mindspace setup <root> --preview --json
mdm mindspace setup <root> --write --json
mdm mindspace context <target> --template <template-id> --json
mdm mindspace template list --json
mdm mindspace template show <template-id> --json
mdm commands --json
```

Planned helpers include `session`, `review`, and `mdmind .` workspace landing
behavior. When they are unavailable, describe the intended review path and keep
proposed writes explicit.

## Closeout Standard

Before final handoff, report:

- which template was used or adapted
- which deterministic commands ran
- what files or branches changed
- what stayed read-only
- what needs human review in `mdmind .`
- any command, validation, or helper that was unavailable
