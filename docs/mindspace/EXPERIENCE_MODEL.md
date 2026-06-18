# Mindspace Experience Model

Mindspace should feel like working with an agent over a local, inspectable
workspace.

The main user experience is not a new CLI habit. It is this loop:

1. The user asks Claude Code, Codex, Hermes, or another capable agent to
   organize, link, summarize, import, or maintain a local knowledge workspace,
   often through a persona/job template rather than a perfect hand-written
   prompt.
2. The agent uses `mdm mindspace ...` as the deterministic tool layer for scan,
   setup, lint, context, session, review, and safe write proposals.
3. The user opens `mdmind .` to inspect the workspace, edit maps and plans,
   navigate files, preview related material, and approve or reject changes.

Short version:

> Talk to the agent. Trust the local workspace. Edit and review in `mdmind`.

## What The User Thinks They Are Doing

Users should not feel like they are learning "Mindspace commands" first. They
should feel like they can say:

> Organize this launch folder. Make a roadmap map, link the PRD and customer
> notes, keep sources read-only, and show me anything risky before writing.

or:

> Read these interview notes and drafts. Build a claims map, link every claim
> back to evidence, and open the workspace so I can review it.

or:

> Use this project memory folder while fixing the auth retry bug. Pull only the
> task branch, linked decisions, and source docs you need. Leave proposed memory
> edits for review.

The agent can run commands, move files, add links, create Markdown pages,
generate native mdmind maps, and propose reorganization. Mindspace makes those
actions bounded, inspectable, and reversible.

## Guided Request Layer

Mindspace should assume that many users are not expert agent operators.

They may know the outcome they want:

- "make this launch folder usable"
- "help me fix this task without rereading the whole repo"
- "find continuity risks in this chapter"
- "turn these interviews into source-backed claims"

but not the prompt details that keep an agent safe and useful.

Job templates are the guided request layer. A template packages:

- a better starting prompt
- expected folder roles
- useful map shapes and branch ids
- source and write policies
- deterministic checks
- the `mdmind .` review surface the user should see afterward

The user can accept a recommended template, customize the common knobs, or ask
the agent to adapt it. See [JOB_TEMPLATES.md](JOB_TEMPLATES.md).

## Surface Responsibilities

| Surface | Primary role | User promise |
| --- | --- | --- |
| Agent conversation | The work request surface. Users ask for organization, synthesis, linking, cleanup, imports, and maintenance in natural language. | "I can ask for the workspace I want instead of operating a new tool by hand." |
| Agent skill or project instructions | The workflow memory. Skills teach agents how to use Mindspace safely and apply job templates across Claude Code, Codex, Hermes, OpenClaw, or similar environments. | "My agent knows the right local protocol without me pasting rules every time." |
| Job templates | The guided request layer. Persona-shaped templates turn vague user goals into safe agent workflows with expected outputs and review paths. | "I can ask better without becoming an agent expert." |
| `mdmind .` | The human workspace. Users inspect, edit, navigate, preview files, work with outlines/plans, and review proposed changes. | "I can see and shape what the agent did in plain local files." |
| `mdm mindspace ...` | The deterministic contract. Agents, scripts, tests, and power users call it for predictable scan/lint/context/session/review behavior. | "The agent's work is grounded in commands I can audit and reproduce." |
| Plain files | The durable substrate. Maps, Markdown pages, sources, indexes, logs, reviews, and sidecars remain readable. | "Nothing important disappears into a hidden database." |

## Product Stance

The CLI is not the main UX. It is the spine.

This matters because modern agent products already let people describe the work
they want done. Claude Code can read and edit projects, run commands, use
skills, and coordinate subagents. Codex can understand codebases, edit files,
review code, debug, and automate development tasks. Agent skills and project
instructions are becoming the natural place to teach repeatable workflows.

Mindspace should meet users there:

- make the agent good at organizing and maintaining local knowledge
- make template-shaped requests easy for users who do not know how to prompt an
  agent precisely
- make `mdmind` excellent for human inspection, editing, navigation, and review
- keep `mdm` precise enough that agents cannot pretend a guess is a verified
  workspace fact

## The Agent-Native Workflow

### 1. Choose The Job

The agent identifies the likely job template and confirms it in human language:

```text
This looks like launch planning. I will use that template unless you want a
lighter pass. It keeps sources read-only, organizes roadmap and decision maps,
and leaves risky changes for review.
```

If the match is unclear, the agent asks one concise question rather than
forcing the user to design the workflow.

### 2. Understand

The agent scans the folder and explains what it found:

```text
I found three native maps, six ordinary Markdown pages, a source folder, an
inbox, and one trusted instruction file. Two cross-file relations are broken.
I will not write anything until you approve the setup.
```

Under the hood:

```bash
mdm mindspace scan . --json
mdm mindspace lint . --json
```

Those two commands are implemented now. They do not write the folder; they give
the agent a deterministic inventory and a fail/pass structural signal before it
proposes setup or edits.

### 3. Shape

The user asks for structure:

```text
Turn this into a launch mindspace. Keep docs as pages, put customer notes under
sources, create a roadmap map if one is missing, and make an inbox for loose
captures.
```

The agent proposes `.mdmind/mindspace.json`, new or changed map files, generated
indexes, and review items. Setup writes remain explicit and small.

Under the hood:

```bash
mdm mindspace setup . --preview
mdm mindspace setup . --write
```

### 4. Work

The user delegates real work:

```text
Build a decision map for the pricing launch. Link each decision to the PRD and
customer evidence. Create TODO branches for unresolved risks.
```

The agent creates and edits Markdown and mdmind files. Mindspace tracks targets,
roles, provenance, and proposed writes.

Under the hood:

```bash
mdm mindspace context maps/roadmap.md#launch/pricing --relations 2 --backlinks
mdm mindspace session ...
```

### 5. Inspect

The user opens:

```bash
mdmind .
```

They see the workspace landing, maps, branches, source previews, generated
reports, recent targets, pinned maps, and review queue. The TUI is not the
agent. It is the place where the human can think with the structured result.

### 6. Review

Risky changes become review items:

```text
Review: move inbox/pricing-objections.md into maps/customer-insights.md
Reason: it supports launch/pricing and has two linked customer quotes
Validation: passes
Digest: current
```

The human approves, rejects, edits, or asks the agent for a narrower attempt.

Under the hood:

```bash
mdm mindspace review ...
```

## What Makes Mindspace Different

Mindspace is not just "AI notes."

- It gives agents branch-addressable targets, not vague file folders.
- It lets agents create and connect Markdown, maps, sources, logs, and reports
  without making every file the same kind of object.
- It gives users job templates instead of expecting perfect prompts.
- It gives humans a local workspace to inspect and edit the result.
- It keeps safety outside the model: read-only scans, explicit setup writes,
  role-aware sources, scoped sessions, checkpoints, digests, and review queues.
- It lets each agent ecosystem use its native surface while sharing the same
  local file contract.

## Research Inputs

Current agent products support this direction:

- Claude Code is positioned as an agentic tool that reads codebases, edits
  files, runs commands, and integrates with development tools:
  <https://code.claude.com/docs/en/overview>
- Claude Code skills package repeatable workflows and load only when relevant:
  <https://code.claude.com/docs/en/skills>
- Claude Code subagents isolate specialized work in separate contexts:
  <https://code.claude.com/docs/en/sub-agents>
- Codex is positioned around understanding projects, editing files, reviewing,
  debugging, and automating development tasks:
  <https://developers.openai.com/codex>
- Codex uses `AGENTS.md` as layered project guidance before work starts:
  <https://developers.openai.com/codex/guides/agents-md>
- Recent LLM-Wiki work argues that agent-native retrieval should support
  searching, reading, traversing, linking, and self-correction rather than flat
  chunk lookup:
  <https://arxiv.org/abs/2605.25480>

Mindspace's bet is to apply that agent-native workflow to local plain-text
knowledge, while keeping humans in charge through `mdmind` and deterministic
contracts.
