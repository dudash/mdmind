# mdmind Skills

Portable skill packages for agents that work with `mdmind` maps and the `mdm` CLI.

These skills track the `mdmind` repo release and include an informational `metadata.version`
field in each `SKILL.md`.

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

Clone the repo once, then install one or both skill folders:

```bash
git clone https://github.com/dudash/mdmind ~/mdmind
```

Skill folders:

- `mdmind-map-authoring`
- `mdm-cli-inspection`

### Portable Layout

If your agent supports the shared Agent Skills convention:

```bash
mkdir -p ~/.agents/skills
cp -R ~/mdmind/skills/mdmind-map-authoring ~/.agents/skills/
cp -R ~/mdmind/skills/mdm-cli-inspection ~/.agents/skills/
```

This is the most portable layout.

### Codex

Copy the folders to:

```bash
mkdir -p ~/.codex/skills
cp -R ~/mdmind/skills/mdmind-map-authoring ~/.codex/skills/
cp -R ~/mdmind/skills/mdm-cli-inspection ~/.codex/skills/
```

Then restart Codex.

Optional: install from GitHub with Codex's built-in installer:

```text
$skill-installer install https://github.com/dudash/mdmind/tree/main/skills/mdmind-map-authoring
$skill-installer install https://github.com/dudash/mdmind/tree/main/skills/mdm-cli-inspection
```

### Claude Code

Claude Code expects each skill folder directly under the skills directory.

Install for all projects:

```bash
mkdir -p ~/.claude/skills
cp -R ~/mdmind/skills/mdmind-map-authoring ~/.claude/skills/
cp -R ~/mdmind/skills/mdm-cli-inspection ~/.claude/skills/
```

Install for one project only:

```bash
mkdir -p .claude/skills
cp -R ~/mdmind/skills/mdmind-map-authoring .claude/skills/
cp -R ~/mdmind/skills/mdm-cli-inspection .claude/skills/
```

Claude usually picks up skill edits live once the skills directory already exists.

## Requirements

- `mdmind-map-authoring` has no required tools, but works best when `mdm` is available for validation.
- `mdm-cli-inspection` expects `mdm` on `PATH`.

## Testing

Test with realistic prompts.

For an isolated Codex test home, run:

```bash
scripts/test-skills.sh --skill mdmind-map-authoring
scripts/test-skills.sh --skill mdm-cli-inspection
```

That script copies the selected skill into a clean `CODEX_HOME`-style directory and
prints the exact `codex` launch command plus the prompt file to use.

Prompt files:

- `mdmind-map-authoring/examples/prompts.md`
- `mdm-cli-inspection/examples/prompts.md`

Check:

- whether the right skill triggers
- whether the output is useful

## Notes

- These skills are instruction-first and intentionally tool-light.
- They are designed to be portable across agent systems, not tied only to this repo.
- The `agents/openai.yaml` files are optional UI metadata for systems that support them.
