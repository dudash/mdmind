# Docs Portal

This folder is the versioned source for mdmind docs. It is organized by what a
reader is trying to do, not by when a feature was built.

## Lanes

- [manual/](manual/): task-focused user guides. These are the best source for a future public docs site.
- [help/](help/): source content for embedded TUI help and guide-shaped map material.
- [reference/](reference/): exact behavior for syntax, queries, ids, relations, and other durable contracts.
- [mindspace/](mindspace/): optional folder-level workspace docs, contracts, design boundaries, and tips.
- [agents/](agents/): guidance for agent use, skills, evals, and machine-readable CLI contracts.
- [design/](design/): product and UX design notes. These are rationale, not the user manual.
- [product/](product/): shipped product summaries and PRDs. Linear owns active task tracking and future backlog.

## Start Here

If you are new to mdmind:

- [help/USER_GUIDE.md](help/USER_GUIDE.md)
- [manual/TUI_WORKFLOWS.md](manual/TUI_WORKFLOWS.md)
- [manual/USING_MDMIND_AS_OUTLINER.md](manual/USING_MDMIND_AS_OUTLINER.md)

If you want to use mdmind with AI agents:

- [agents/AGENT_USAGE.md](agents/AGENT_USAGE.md)
- [agents/AGENT_CLI_CONTRACT.md](agents/AGENT_CLI_CONTRACT.md)
- [agents/AGENT_TODO_MEMORY.md](agents/AGENT_TODO_MEMORY.md)
- [agents/SKILLS_CUSTOMIZING.md](agents/SKILLS_CUSTOMIZING.md)
- [agents/SKILL_EVAL_HARNESS.md](agents/SKILL_EVAL_HARNESS.md)

If you want exact behavior:

- [../CHANGELOG.md](../CHANGELOG.md)
- [../spec/README.md](../spec/README.md)
- [reference/QUERY_LANGUAGE.md](reference/QUERY_LANGUAGE.md)
- [reference/IDS_AND_DEEP_LINKS.md](reference/IDS_AND_DEEP_LINKS.md)
- [reference/CROSS_LINKS_AND_BACKLINKS.md](reference/CROSS_LINKS_AND_BACKLINKS.md)
- [reference/NODE_DETAILS.md](reference/NODE_DETAILS.md)

If you want product direction:

- [product/README.md](product/README.md)
- [product/shipped/README.md](product/shipped/README.md)
- [product/prds/](product/prds/)
- [mindspace/README.md](mindspace/README.md)

## Boundaries

- User-facing guides belong in [manual/](manual/) unless they are specifically embedded-help source.
- Built-in help source belongs in [help/](help/).
- Release notes belong in [../CHANGELOG.md](../CHANGELOG.md); active release tasks still belong in Linear.
- Design notes belong in [design/](design/), even when they describe a shipped feature.
- Mindspace docs belong in [mindspace/](mindspace/) unless a change is only about core map syntax, CLI behavior, or TUI behavior outside the optional workspace layer.
- Active implementation tasks and future prioritization belong in Linear, not in repo docs.
- Historical planning shelves live under [product/_archive/](product/_archive/) for context only.

## Working On The Repo

If you are changing code or release automation, start with [DEVELOPER.md](../DEVELOPER.md).
