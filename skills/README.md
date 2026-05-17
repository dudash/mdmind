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
- attach external files, URLs, and images with normal Markdown refs

### `mdm-cli-inspection`

Use when the main job is inspecting, validating, querying, deep-linking, listing external refs, or exporting an existing map with `mdm`.

Good fits:

- validating generated maps
- finding tags, metadata, ids, or relations
- listing attached local files, web links, or image refs
- auditing `@owner`, `@status`, or similar fields
- exporting JSON, Mermaid, or OPML
- answering questions about one branch via deep links

This skill teaches agents how to choose the right `mdm` command for the job.
It also distinguishes command `--json` response envelopes from
`mdm export --format json`, which remains raw map document JSON for downstream
tools.

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

Each skill is a portable package: one directory containing a required `SKILL.md`
and optional `references/`, `examples/`, and `agents/` metadata.

### Install For One Project

If your agent supports the shared Agent Skills convention, copy the skills into
the current project:

```bash
mkdir -p .agents/skills
cp -R ~/mdmind/skills/mdmind-map-authoring .agents/skills/
cp -R ~/mdmind/skills/mdm-cli-inspection .agents/skills/
```

This keeps mdmind behavior scoped to one repository or workspace.

### Install For All Your Work

If your agent supports user-level shared Agent Skills, copy the skills into your
home directory:

```bash
mkdir -p ~/.agents/skills
cp -R ~/mdmind/skills/mdmind-map-authoring ~/.agents/skills/
cp -R ~/mdmind/skills/mdm-cli-inspection ~/.agents/skills/
```

This is the most portable user-level layout for Codex, Cursor, Gemini CLI,
Copilot-compatible tools, OpenCode, Warp, Pi, Windsurf, and similar agents.
Claude Code currently uses `.claude/skills/` and `~/.claude/skills/` instead.

For a dated agent-specific table, see
[docs/AGENT_SKILL_INSTALLS.md](../docs/AGENT_SKILL_INSTALLS.md).

### Alternative: Skills CLI

If you already use Vercel Labs' open `skills` CLI, install mdmind from the
public GitHub repo:

```bash
npx skills add dudash/mdmind --skill mdmind-map-authoring --skill mdm-cli-inspection
```

Preview the skills first:

```bash
npx skills add dudash/mdmind --list
```

Install globally instead of into the current project:

```bash
npx skills add dudash/mdmind --skill mdmind-map-authoring --skill mdm-cli-inspection --global
```

This is a convenience path, not the source of truth. The source of truth remains
the checked-in skill folders under `skills/`.

For maintainers: there is no separate skills.sh registration flow. Keep the
skills in a public repo, make sure each `SKILL.md` has valid `name` and
`description` frontmatter, and verify discovery with `npx skills add dudash/mdmind
--list`. The skills can appear on skills.sh after users install them through the
CLI's telemetry-backed directory.

### Codex

Codex can load shared Agent Skills from `~/.agents/skills/` and project-local
`.agents/skills/` directories. Use project installs when the skills should
travel with one repository, or user installs when the skills should apply across
your work.

Then restart Codex if the skills do not appear.

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

### Other Agents

Most modern coding agents use the same package shape but different search paths.
Install the two mdmind skill folders directly under the target skills directory.

| Agent | Good install target |
|---|---|
| Cursor | `~/.agents/skills/` or `~/.cursor/skills/` |
| Gemini CLI | `~/.agents/skills/` or `~/.gemini/skills/` |
| GitHub Copilot / VS Code Copilot | `~/.agents/skills/` or `~/.copilot/skills/` |
| OpenCode | `~/.agents/skills/` or `~/.config/opencode/skills/` |
| Warp | `~/.agents/skills/` |
| Pi Coding Agent | `~/.agents/skills/` or `~/.pi/agent/skills/` |
| Windsurf Cascade | `~/.agents/skills/` or `~/.codeium/windsurf/skills/` |

Prefer `~/.agents/skills/` first when the agent supports it. Add native paths or
symlinks only when an agent does not discover the shared location.

### Passive Project Context

Skills load on demand. For projects that frequently use mdmind, add a short
always-on agent note too:

- [docs/AGENTS_SNIPPET.md](../docs/AGENTS_SNIPPET.md)
- [docs/SKILLS_CUSTOMIZING.md](../docs/SKILLS_CUSTOMIZING.md)

The snippet explains the basic map format and validation loop without duplicating
the full skills. The customizing guide shows how to tailor the base skills with
domain-specific tags, metadata keys, id patterns, relation types, and validation
commands. Edit the global, user-level, or project-local copy your agent loads and
add a short `## Project Overrides` section instead of creating a separate skill.

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
- Avoid duplicate skill `name` values in the same agent environment unless you
  have tested that agent's precedence behavior.
