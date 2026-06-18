# Mateo: Project Memory And Agent Handoff

Use `project-memory` when the user wants an agent to work in a project with
bounded context and durable memory updates.

## Starting Prompt

```text
Use the project memory template. Start from the current task branch, gather only
linked decisions, API docs, and relevant debugging notes, then propose any
durable memory updates for review. Do not rewrite unrelated project notes.
```

## Expected Workspace

```text
AGENTS.md
maps/tasks.md
maps/decisions.md
docs/api.md
notes/debugging.md
log.md
```

## Useful Branches

- `tasks/current`
- `tasks/blocked`
- `tasks/handoff`
- `decisions/accepted`
- `debugging/known-failures`

## Workflow

1. Run scan and lint.
2. Resolve the target branch before reading broad context.
3. Gather bounded context with provenance.
4. Do the coding or investigation in the normal project surface.
5. Propose durable memory updates as review items when knowledge changed.
6. Validate changed maps.

## mdmind Review

The human should see the target task branch, included context, accepted/open
decisions, memory update proposals, and recent handoff notes.

## Success

Mateo can audit what the agent saw, and useful project knowledge returns to the
mindspace without silent drift.
