# mdmind Skills

Portable skill packages for agents that work with `mdmind` maps and the `mdm` CLI.

## Included Skills

### `mdmind-map-authoring`

Use when the main job is creating or reshaping native `mdmind` map content.

Good fits:

- meeting notes to action/decision maps
- research synthesis maps
- project and product planning maps
- writing and story maps
- decision maps
- cleaning up or normalizing an existing `mdmind` file

This skill teaches agents how to:

- write outline-first node labels
- move nuance into `| detail` lines
- use `#tags` and `@key:value` metadata deliberately
- add `[id:...]` only on durable branches
- use `[[...]]` relations sparingly and meaningfully

### `mdm-cli-inspection`

Use when the main job is inspecting, validating, querying, deep-linking, or exporting an existing map with `mdm`.

Good fits:

- validating generated maps
- finding tags, metadata, ids, or relations
- auditing `@owner`, `@status`, or similar fields
- exporting JSON, Mermaid, or OPML
- answering questions about one branch via deep links

This skill teaches agents how to choose the right `mdm` command for the job.

## Recommended Split

Use `mdmind-map-authoring` when the core task is content creation.

Use `mdm-cli-inspection` when the core task is CLI-based inspection or export.

If both are needed, author first and inspect second.

## Installation

The exact install path depends on the agent product.

Portable default:

1. Copy one or both skill folders into the agent’s global or user-level skills directory.
2. Keep the folder names intact:
   - `mdmind-map-authoring`
   - `mdm-cli-inspection`
3. Preserve the internal structure:
   - `SKILL.md`
   - `references/`
   - `agents/openai.yaml` when supported

For Codex-style setups, that usually means placing the skill folders under a configured skills root and restarting the tool if needed.

For other agent systems, install them according to that system’s skill/package instructions.

## Quick Validation

Before sharing the skills broadly, test them with realistic prompts.

Suggested prompts live in:

- `mdmind-map-authoring/examples/prompts.md`
- `mdm-cli-inspection/examples/prompts.md`

You should test both:

- whether the right skill triggers
- whether the resulting output shape is useful

## Notes

- These skills are instruction-first and intentionally tool-light.
- They are designed to be portable across agent systems, not tied only to this repo.
- The `agents/openai.yaml` files are optional UI metadata for systems that support them.
