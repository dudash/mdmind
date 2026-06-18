# Mindspace Workflow

Use this reference when the user asks an agent to organize, maintain, synthesize,
or review a folder as a Mindspace.

## Operating Loop

1. Identify the root folder and the job the user wants done.
2. Choose the closest job template. If unclear, ask one concise question.
3. Run `mdm mindspace scan <root> --json`.
4. Run `mdm mindspace lint <root> --json`.
5. Explain the workspace roles and deterministic issues before writes.
6. Propose setup or scoped edits. Preview broad or risky changes first.
7. Apply only approved writes.
8. Validate changed maps with `mdm validate <map>`.
9. Summarize the `mdmind .` review path.

## Vague Prompt Sharpening

When the user says:

```text
Can you organize this folder?
```

respond with a template-shaped confirmation:

```text
This looks like launch planning. I will use the launch-planning template unless
you want a lighter pass. I will scan/lint first, keep sources read-only, propose
setup before writing, and leave risky changes for mdmind review.
```

If the job is ambiguous, ask one question:

```text
Should this be a launch planning workspace, a project memory handoff, a story
continuity review, or a claims/evidence synthesis?
```

## Current Commands

Use current helper commands before relying on docs:

```bash
mdm mindspace template list --json
mdm mindspace template show launch-planning --json
mdm mindspace scan . --json
mdm mindspace lint . --json
mdm mindspace setup . --preview --json
mdm mindspace setup . --write --json
```

Use `mdm commands --json` when you need to confirm current command availability,
output modes, reads, writes, and examples.

## Planned Commands

The following commands may be referenced in product docs but might not exist in
the current installed version:

```bash
mdm mindspace context <target> --json
mdm mindspace session ...
mdm mindspace review ...
mdmind .
```

When a helper is missing, do not fake it. Use scan/lint/template output, propose
the intended write or review item in prose, and tell the user what should be
implemented later.

## Handoff Shape

A strong Mindspace closeout says:

- template used or adapted
- scan/lint result
- maps, pages, sources, inbox, logs, instructions, and reports detected
- files or branches changed
- files kept read-only
- validation result
- review items or risky changes
- what to inspect in `mdmind .`

Keep the handoff short enough that the user can act on it.
