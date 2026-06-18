# Priya: Launch Planning

Use `launch-planning` when the user wants a calm operating view over a product,
marketing, or internal launch folder.

## Starting Prompt

```text
Use the launch planning template for this folder. Identify maps, docs, inbox,
decisions, customer evidence, and the log. Keep source material read-only. Build
or update a roadmap map with blocked work, open decisions, risks, and next
milestones. Show me the manifest and any risky edits before writing.
```

## Expected Workspace

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

## Useful Branches

- `roadmap/current`
- `roadmap/blocked`
- `roadmap/risks`
- `roadmap/milestones`
- `decisions/open`
- `evidence/customer-signals`

## Workflow

1. Run scan and lint.
2. Explain detected roles and risks.
3. Propose setup if no manifest exists.
4. Create or update roadmap, decisions, and customer-insight maps only in the
   approved write scope.
5. Link roadmap branches to decisions and evidence with sparse relations.
6. Leave ambiguous moves, duplicate notes, and risky rewrites as review items.

## mdmind Review

The human should see current launch status, blocked branches, open decisions,
touched files, and review items needing approval.

## Success

Priya can answer "what matters this week?" without opening six files, and risky
edits are visible before they become part of the plan.
