Command used:

```bash
mdm find inputs/agent-todo-workflow.md "task:blocked @owner:jason" --plain
```

Result:

- Parent rollup visualization is blocked and owned by Jason.
- It lives under `codex-todo/blocked/rollups`.
- The blocker is the task-state read model, referenced by the related checklist model branch.

No export is needed because this is a narrow inspection query, not a downstream data handoff.
