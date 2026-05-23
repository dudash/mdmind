# Template Variables

## Goal

Make `mdm init` templates feel reusable for real projects by allowing a small set of prompted variables.

## Problem

The current templates are useful scaffolds, but they still require repetitive manual edits for the first few details:

- owner
- project name
- default status
- repository or area names

## UX

When running `mdm init`, the user can either:

- accept the template as-is
- pass variables with flags
- or be prompted interactively for missing values

Example:

- `mdm init roadmap.md --template product --var owner=jason --var status=todo`

Template placeholders might look like:

- `{{project_name}}`
- `{{owner}}`
- `{{default_status}}`

## Rules

- keep the variable syntax minimal
- support defaults
- avoid full template-language complexity

Optional syntax:

- `{{owner}}`
- `{{owner|jason}}`

## Architecture

- template parser for placeholders
- prompt layer in CLI only when values are missing
- deterministic rendering step before file write

## Delivery

Phase 1:

- `--var key=value`
- placeholder substitution with defaults

Phase 2:

- interactive prompt for missing values
- template metadata describing variables and help text

## Risks

- template language scope creep
- too much logic leaking into static starter templates

## Product Test

A user should be able to create a project-specific starter map with one command and minimal cleanup.
