# Mindspace Templates

Use this reference to choose, show, or adapt job templates.

Templates help users who know the outcome they want but do not know how to
prompt an agent safely. A template is not a workspace type and not permission to
write. It is a reusable job shape.

## Current Built-In Templates

List templates from the local CLI when possible:

```bash
mdm mindspace template list --json
```

Show one template:

```bash
mdm mindspace template show launch-planning --json
mdm mindspace template show launch-planning --prompt
```

Built-in ids:

| ID | Persona | Job |
| --- | --- | --- |
| `launch-planning` | Priya Planner | Roadmap, decisions, customer evidence, blocked work, and launch review. |
| `project-memory` | Mateo Techie | Bounded context, coding-agent handoff, decisions, and durable memory proposals. |
| `story-continuity` | Ren Writer | Character/place/timeline checks without rewriting prose drafts. |
| `claims-evidence` | Nova Researcher | Source-backed claims, evidence links, confidence, and open questions. |

## Anatomy

Every template should define:

- id
- name
- persona fit
- job fit
- starting prompt
- folder roles
- map shapes and durable branches
- agent workflow
- deterministic checks
- write policy
- `mdmind .` review surface
- success criteria
- customization knobs

## Common Knobs

Use knobs to adapt a template for unknown personas instead of inventing a new
workflow from scratch.

| Knob | Options | Use |
| --- | --- | --- |
| source strictness | `read_only`, `cite_required`, `summary_allowed` | How tightly synthesis must point to evidence. |
| write mode | `review_only`, `scoped_apply`, `append_only_log` | How much the agent can change after approval. |
| structure depth | `light`, `normal`, `detailed` | How much map structure to create. |
| primary output | `map`, `page`, `index`, `report`, `review_items` | What the job should produce first. |
| relation density | `none`, `sparse`, `evidence_heavy` | How aggressively to add cross-links. |
| context budget | file, branch, and detail limits | How much context the agent should read. |
| review tone | `risks`, `decisions`, `continuity`, `claims`, `handoff` | What the human sees first. |

## Selection Heuristics

- Choose launch planning when the user asks for launch status, roadmap, risks,
  decisions, customer signals, or weekly operating view.
- Choose project memory when the user asks an agent to use a project folder
  while coding, debugging, remembering decisions, or handing off task context.
- Choose story continuity when the user asks about drafts, scenes, characters,
  places, timelines, themes, or story bible maintenance.
- Choose claims/evidence when the user asks for research synthesis, source
  grounding, claims, evidence gaps, confidence, citations, or stale sources.

If two templates fit, name the tradeoff and ask one short question.
