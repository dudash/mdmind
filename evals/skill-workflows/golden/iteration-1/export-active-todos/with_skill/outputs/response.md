Use raw export JSON because the downstream tool needs map data, not an mdm response envelope:

```bash
mdm export inputs/agent-todo-workflow.md --query "#todo @status:active" --format json > outputs/export.json
```

I would not use command `--json` here because `mdm export --format json` intentionally returns the raw map document shape.
