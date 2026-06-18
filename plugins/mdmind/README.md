# mdmind Agent Plugin

This plugin bundles the two mdmind skills for agents:

- `mdmind-map-authoring`
- `mdm-cli-inspection`

Mindspace will likely add a third skill, `mdmind-mindspace-workflow`, for
folder-level job templates, scan/lint/setup/context/session/review workflows,
and `mdmind .` human review handoff. It should compose the two existing skills
rather than duplicating map authoring or CLI inspection guidance.

> Install with `mdm skills install`, then pick the agent or agents where you
> want the mdmind skills available.

The plugin-bundled skill folders under `skills/` are the canonical mdmind skill
sources. The same plugin root is used for Claude Code and Codex:

- Claude Code manifest: `.claude-plugin/plugin.json`
- Codex manifest: `.codex-plugin/plugin.json`
- Bundled skills: `skills/`

## Included Skills

### `mdmind-map-authoring`

Use when the main job is creating or reshaping native `mdmind` map content:
meeting notes, research synthesis, project planning, writing outlines, decision
maps, or normalization of an existing `.md` map.

### `mdm-cli-inspection`

Use when the main job is validating, querying, deep-linking, listing external
refs, auditing metadata, or exporting an existing map with `mdm`.

Use `mdmind-map-authoring` when the core task is content creation. Use
`mdm-cli-inspection` when the core task is CLI-based inspection or export. If
both are needed, author first and inspect second.

### Planned: `mdmind-mindspace-workflow`

Use when the main job is organizing or maintaining a folder-level Mindspace
through an agent. This skill should help Claude Code, Codex, Hermes, or another
agent choose a persona/job template, run `mdm mindspace scan` and `lint`,
preview setup, keep sources read-only, produce bounded context, create
reviewable changes, and tell the human what to inspect in `mdmind .`.

Design target: [Mindspace agent skill](../../docs/mindspace/AGENT_SKILL.md).

## Skills CLI

Run the installer and choose your agent target when prompted:

```bash
mdm skills install
```

To print the underlying command without running it:

```bash
mdm skills install --print
```

That wrapper runs:

```bash
npx skills add dudash/mdmind
```

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

## Claude Code

Validate the marketplace and plugin from the repository root:

```bash
claude plugin validate .
claude plugin validate plugins/mdmind
```

Add this repository as a marketplace from GitHub:

```bash
claude plugin marketplace add dudash/mdmind --sparse .claude-plugin plugins
claude plugin install mdmind@mdmind
```

For local testing before pushing:

```bash
claude plugin marketplace add . --scope local
claude plugin install mdmind@mdmind --scope local
```

Claude Code exposes plugin skills with the plugin namespace, for example:

```text
/mdmind:mdmind-map-authoring
/mdmind:mdm-cli-inspection
```

## Codex

Codex reads the repo marketplace from:

```text
.agents/plugins/marketplace.json
```

Restart Codex after changing the plugin or marketplace metadata, or register
the repo marketplace explicitly:

```bash
codex plugin marketplace add .
```

Then install or enable the `mdmind` plugin from the `mdmind` marketplace in the
plugin directory. The `.agents` folder is shared agent workspace convention, but
`.agents/plugins/marketplace.json` is the Codex plugin marketplace path.

## Manual Fallback

If the Skills CLI is unavailable, clone the repo and copy the skill folders into
your agent's supported skills directory.

```bash
git clone https://github.com/dudash/mdmind ~/mdmind
```

Codex and other agents that support the shared Agent Skills convention:

```bash
mkdir -p .agents/skills
cp -R ~/mdmind/plugins/mdmind/skills/mdmind-map-authoring .agents/skills/
cp -R ~/mdmind/plugins/mdmind/skills/mdm-cli-inspection .agents/skills/
```

Claude Code native skills directory:

```bash
mkdir -p .claude/skills
cp -R ~/mdmind/plugins/mdmind/skills/mdmind-map-authoring .claude/skills/
cp -R ~/mdmind/plugins/mdmind/skills/mdm-cli-inspection .claude/skills/
```

Restart the agent if the skills do not appear.

## Customization And Testing

Customize the installed copy your agent actually loads. For project-specific
rules, add a short `## Project Overrides` section near the top of the relevant
`SKILL.md`.

Related docs:

- [Agent usage](../../docs/agents/AGENT_USAGE.md)
- [Agent context snippet](../../docs/agents/AGENTS_SNIPPET.md)
- [Customizing skills](../../docs/agents/SKILLS_CUSTOMIZING.md)
- [Skill eval harness](../../docs/agents/SKILL_EVAL_HARNESS.md)

Outcome evals live in [../../evals/skill-workflows](../../evals/skill-workflows).
For an isolated Codex test home, run:

```bash
scripts/agents/test-skills.sh --skill mdmind-map-authoring
scripts/agents/test-skills.sh --skill mdm-cli-inspection
```
