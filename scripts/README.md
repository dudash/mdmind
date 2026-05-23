# Scripts

Small maintainer scripts live here, grouped by owner surface.

- [docs/](docs/): documentation checks and repo-doc hygiene.
- [release/](release/): release and installer metadata automation used locally
  and by GitHub workflows.
- [agents/](agents/): skill/plugin maintenance helpers and local agent smoke
  tests.

Scripts that are useful locally should stay here even when GitHub Actions also
calls them. Workflow-only YAML stays in `.github/workflows/`.
