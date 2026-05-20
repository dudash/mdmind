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
tools, and can use `mdm commands --json` as the local command discovery layer.

## Recommended Split

Use `mdmind-map-authoring` when the core task is content creation.

Use `mdm-cli-inspection` when the core task is CLI-based inspection or export.

If both are needed, author first and inspect second.

## Evaluation

Outcome evals for these skills live in
[evals/skill-workflows](../evals/skill-workflows). The local harness initializes
clean workspaces, lets an agent write requested artifacts, and grades the
results with deterministic checks and `mdm` commands.

## Installation

Skill folders:

- `mdmind-map-authoring`
- `mdm-cli-inspection`

Each skill is a portable package: one directory containing a required `SKILL.md`
and optional `references/`, `examples/`, and `agents/` metadata.

### Recommended: Skills CLI

Use Vercel Labs' open `skills` CLI for normal local installs. It can preview,
install, update, and remove skills without hand-copying agent directories.

Preview the mdmind skills:

```bash
npx skills add dudash/mdmind --list
```

Install both skills for the current project in Claude Code and Codex:

```bash
npx skills add dudash/mdmind \
  --skill mdmind-map-authoring \
  --skill mdm-cli-inspection \
  -a claude-code \
  -a codex
```

Install globally for your user account:

```bash
npx skills add dudash/mdmind \
  --skill mdmind-map-authoring \
  --skill mdm-cli-inspection \
  -g \
  -a claude-code \
  -a codex
```

For non-interactive setup, add `-y`. The CLI symlinks by default so updates can
flow through one canonical copy; add `--copy` only when symlinks are not
acceptable in your environment.

Useful maintenance commands:

```bash
npx skills list
npx skills update
npx skills remove mdmind-map-authoring mdm-cli-inspection
```

The checked-in skill folders under `skills/` remain the development source of
truth for mdmind. The CLI is the recommended installation and update path.

### Agent Plugins

mdmind also ships a shared plugin package at [plugins/mdmind](../plugins/mdmind)
for agents that support plugins as the reusable distribution layer.

Claude Code marketplace install:

```bash
claude plugin marketplace add dudash/mdmind --sparse .claude-plugin plugins
claude plugin install mdmind@mdmind
```

Codex reads the repo marketplace from `.agents/plugins/marketplace.json`. Restart
Codex after pulling this repo, or register the repo marketplace explicitly:

```bash
codex plugin marketplace add .
```

Then install or enable the `mdmind` plugin from the `mdmind` marketplace in the
plugin directory. The `.agents` folder is shared agent workspace convention, but
`.agents/plugins/marketplace.json` is the Codex plugin marketplace path.

Use `npx skills` when you only want standalone skills. Use the plugin when you
want the agent-native plugin surface, namespaced skills, and bundled skill
assets.

### Manual Fallback

If the CLI is unavailable, clone the repo and copy the skill folders into your
agent's supported skills directory.

```bash
git clone https://github.com/dudash/mdmind ~/mdmind
```

Codex and other agents that support the shared Agent Skills convention:

```bash
mkdir -p .agents/skills
cp -R ~/mdmind/skills/mdmind-map-authoring .agents/skills/
cp -R ~/mdmind/skills/mdm-cli-inspection .agents/skills/
```

Claude Code native skills directory:

```bash
mkdir -p .claude/skills
cp -R ~/mdmind/skills/mdmind-map-authoring .claude/skills/
cp -R ~/mdmind/skills/mdm-cli-inspection .claude/skills/
```

Restart the agent if the skills do not appear.

For maintainers: there is no separate skills.sh registration flow. Keep the
skills in a public repo, make sure each `SKILL.md` has valid `name` and
`description` frontmatter, and verify discovery with `npx skills add dudash/mdmind
--list`. The skills can appear on skills.sh after users install them through the
CLI's telemetry-backed directory.

### Other Agents

Most modern coding agents use the same package shape but different search paths.
Prefer `npx skills` with a specific `-a <agent>` target when that agent is
supported. If you must install manually, put the two mdmind skill folders
directly under the target skills directory.

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
