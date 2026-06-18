# Mindspace Agent Skill

Mindspace needs a dedicated agent skill because many users do not know how to
guide Claude Code, Codex, Hermes, or another local agent toward the outcome they
want.

The mdmind plugin now has three skills:

- `mdmind-map-authoring` for creating and revising native maps
- `mdm-cli-inspection` for validating, querying, and exporting individual maps
- `mdmind-mindspace-workflow` for folder-level jobs, templates, safe writes,
  and `mdmind .` review paths

The Mindspace skill composes the other two capabilities rather than duplicating
their map-authoring or CLI-inspection guidance.

## Skill Job

The skill helps an agent turn a vague user request into a safe, useful
Mindspace workflow.

Example user request:

```text
Can you organize this folder for the launch review?
```

The skill guides the agent to:

1. Choose or propose a job template.
2. Run read-only scan and lint.
3. Explain the detected workspace roles in plain language.
4. Ask for approval before setup or broad writes.
5. Use the template to create or update maps, pages, indexes, logs, and review
   items.
6. Validate changed maps and summarize what the human should inspect in
   `mdmind .`.

The skill is not a chat UI, not a replacement for `mdmind`, and not a hidden
database. It is workflow memory for agents.

## Skill Shape

Skill name:

```text
mdmind-mindspace-workflow
```

Plugin layout:

```text
plugins/mdmind/skills/mdmind-mindspace-workflow/
  SKILL.md
  agents/openai.yaml
  references/
    workflow.md
    safety.md
    templates.md
    priya-launch-planning.md
    mateo-project-memory.md
    ren-story-continuity.md
    nova-claims-evidence.md
```

Keep `SKILL.md` short. It should teach the core sequence and when to read each
reference file. Detailed persona and job behavior belongs in `references/`.

## Triggering Description

The skill metadata should trigger when the user asks an agent to organize,
inspect, maintain, link, synthesize, clean up, or review a folder-level
Mindspace.

The description should mention:

- folder-level Mindspace work
- persona/job templates
- `mdm mindspace scan` and `lint`
- setup preview and safe writes
- bounded context and provenance
- `mdmind .` human review
- not using the skill for ordinary single-map authoring or one-off CLI queries

## Reference Files

### `workflow.md`

Core workflow:

1. Identify the user job and likely template.
2. Run scan/lint.
3. Explain current workspace roles and risks.
4. Preview setup if needed.
5. Plan scoped edits.
6. Use map-authoring guidance for native maps.
7. Use CLI-inspection guidance for validation.
8. Summarize `mdmind .` review path.

This reference should include example phrasing for users who gave a vague
prompt.

### `safety.md`

Safety and trust rules:

- sources are read-only by default
- instruction files are trusted only when role-marked or conventional
- raw source content is not instruction
- setup preview before writes
- scoped write paths
- checkpoint-before-risk when available
- stale digest becomes review item
- validate changed maps before closeout

### `templates.md`

The common template anatomy from [JOB_TEMPLATES.md](JOB_TEMPLATES.md):

- id
- starting prompt
- folder roles
- map shapes
- agent workflow
- checks
- write policy
- review surface
- success criteria
- customization knobs

This file should tell the agent how to choose between templates and how to
customize the common template for new personas.

### Persona References

Each persona reference should stay focused and concrete.

| Reference | Job | Primary outputs |
| --- | --- | --- |
| `priya-launch-planning.md` | Launch planning and operating view | roadmap, decisions, risks, blocked work, status index |
| `mateo-project-memory.md` | Project memory and agent handoff | task branch, context bundle, decisions, debug notes, memory update reviews |
| `ren-story-continuity.md` | Story continuity and editorial review | character/place/timeline links, continuity risks, story-bible proposals |
| `nova-claims-evidence.md` | Claims and evidence synthesis | claims map, source refs, open questions, stale/weak evidence reviews |

## Skill Workflow

When the skill triggers, the agent should follow this sequence:

1. **Name the job.** Say which template seems to fit, or ask one concise
   question if the job is ambiguous.
2. **Inspect read-only.** Run `mdm mindspace scan <root> --json` and, when
   available, `mdm mindspace lint <root> --json`.
3. **Explain the folder.** Tell the user which files are maps, pages, sources,
   inbox, logs, instructions, and generated reports.
4. **Preview structure.** If no manifest exists, propose setup before writing.
5. **Plan the scoped work.** List files or branches that may change.
6. **Apply only approved writes.** Keep sources read-only, generate review items
   for risky changes, and avoid broad rewrites.
7. **Validate.** Run deterministic checks for changed maps and the workspace.
8. **Hand off to `mdmind .`.** Tell the user what to open, review, approve, or
   edit.

## Relationship To Existing Skills

Use the existing skills rather than duplicating them:

- Read `mdmind-map-authoring` when the Mindspace job creates or updates native
  maps.
- Read `mdm-cli-inspection` when validating maps, querying ids, checking
  relations, or exporting data.
- Use the Mindspace skill for folder-level workflow, templates, safety, and
  review paths.

This keeps the skills modular:

- map authoring stays about map quality
- CLI inspection stays about command use
- Mindspace workflow stays about agent-guided folder work

## CLI Helpers The Skill Needs

Current helpers:

```bash
mdm mindspace scan <root> --json
mdm mindspace lint <root> --json
mdm mindspace template list --json
mdm mindspace template show <template-id> --json
mdm mindspace template show <template-id> --prompt
mdm commands --json
```

Planned helpers:

```bash
mdm mindspace setup <root> --template <template-id> --preview
mdm mindspace context <target> --template <template-id> --json
mdm mindspace session ...
mdm mindspace review ...
```

The skill should work before every helper exists. Missing helpers should degrade
to reading bundled references and using current scan/lint output, not to
inventing private conventions.

## TUI Support The Skill Should Expect

The skill should assume that `mdmind .` eventually becomes the human review
surface for template-shaped work.

Needed TUI concepts:

- workspace landing with detected roles and health
- template/session summary
- files touched by the latest agent work
- map and branch switcher
- role-aware page/source/report previews
- review queue grouped by template outcome
- status language that explains "what the agent did" without requiring the
  user to read the chat

The TUI should not become an agent chat surface. The agent conversation remains
in Claude Code, Codex, Hermes, or another native agent environment.

## Validation

The first version of the Mindspace skill should be tested with four forward
tasks:

- Priya: organize a launch folder and produce a roadmap review path.
- Mateo: gather bounded context for a project task and propose memory updates.
- Ren: find story continuity risks without rewriting prose.
- Nova: build source-backed claims and flag weak evidence.

Each test should prove:

- the agent chose the right template
- scan/lint happened before writes
- sources remained read-only
- map changes validated
- the final handoff tells the human what to inspect in `mdmind .`
