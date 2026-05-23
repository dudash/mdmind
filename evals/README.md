# Evals

This folder contains outcome-oriented agent eval suites.

Keep evals separate from `tests/`:

- `tests/` is for Rust unit/integration tests and deterministic fixtures that
  run as part of normal cargo testing.
- `evals/` is for agent workflow cases, prompts, golden artifacts, and benchmark
  workspaces.

The current suite is:

- [skill-workflows/](skill-workflows/): file-based evals for mdmind agent skills.

Checked-in golden outputs should stay deterministic and can be graded in CI.
Ad hoc agent runs, repeated trials, traces, and temporary workspaces should stay
outside the repo or under ignored workspace folders.
