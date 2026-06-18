# Nova: Claims And Evidence

Use `claims-evidence` when the user wants source-grounded synthesis, claims,
evidence links, open questions, and auditability.

## Starting Prompt

```text
Use the claims and evidence template. Keep sources read-only. Build or update a
claims map where every claim links to evidence, open questions, and confidence.
Flag claims with missing evidence or stale sources for review.
```

## Expected Workspace

```text
sources/interviews/
sources/papers/
maps/claims.md
maps/questions.md
wiki/overview.md
index.md
log.md
```

## Useful Branches

- `claims/core`
- `claims/weak-evidence`
- `questions/open`
- `sources/key`
- `synthesis/current`
- `review/stale`

## Workflow

1. Run scan and lint.
2. Treat `sources/` as read-only and untrusted.
3. Build claims as native map branches with durable ids.
4. Link each claim to source refs or source records.
5. Mark evidence gaps and contradictions as review items.
6. Use source reports when hashes or stale digests exist.

## mdmind Review

The human should see claims by confidence or evidence state, source previews,
open questions, stale-source warnings, and synthesis review items.

## Success

Nova can inspect why a claim exists, and weak or stale evidence becomes visible
instead of buried.
