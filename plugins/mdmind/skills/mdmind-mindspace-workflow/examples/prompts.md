# Mindspace Workflow Forward-Test Prompts

Use these prompts to test whether `mdmind-mindspace-workflow` helps an agent
choose templates, scan/lint first, keep sources read-only, validate maps, and
hand off review in `mdmind .`.

## Priya: Launch Planning

```text
Use $mdmind-mindspace-workflow on this launch folder. Organize the docs, inbox,
customer notes, and roadmap into a launch mindspace. Keep sources read-only,
use the launch-planning template, and write a short handoff that tells me what
to inspect in mdmind.
```

Expected evidence:

- `mdm mindspace scan <root> --json`
- `mdm mindspace lint <root> --json`
- `launch-planning` selected
- roadmap/decision/evidence outputs or review items

## Mateo: Project Memory

```text
Use $mdmind-mindspace-workflow while fixing this task. Start from
maps/tasks.md#tasks/current, gather only linked decisions and debugging notes,
and propose durable memory updates for review after the work.
```

Expected evidence:

- bounded target branch
- no unrelated note rewrites
- memory updates reviewable
- changed maps validated

## Ren: Story Continuity

```text
Use $mdmind-mindspace-workflow to check this chapter against the character,
place, timeline, and theme maps. Do not rewrite the draft. Leave continuity
risks and story-bible updates as review items.
```

Expected evidence:

- `story-continuity` selected
- draft page stays read-only unless explicitly approved
- continuity risks separated from story-bible proposals
- mdmind review path names draft, linked maps, and warnings

## Nova: Claims And Evidence

```text
Use $mdmind-mindspace-workflow to turn these interviews and papers into a
claims/evidence mindspace. Keep sources read-only, make every claim traceable,
and flag weak or stale evidence for review.
```

Expected evidence:

- `claims-evidence` selected
- source folders remain read-only
- claims include evidence state or review flags
- weak evidence is visible in the handoff
