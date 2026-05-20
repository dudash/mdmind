# Agent Skill Install Notes

Last reviewed: 2026-05-18

Agent skill directory conventions are still changing. For normal local installs,
prefer the open `skills` CLI because it can target agent-native locations,
preview available skills, update installed skills, and remove stale installs.
Keep manual directory copies as an offline fallback only.

## Recommended mdmind Installs

Preview the skills in the public repo:

```bash
npx skills add dudash/mdmind --list
```

Install both mdmind skills for the current project in Claude Code and Codex:

```bash
npx skills add dudash/mdmind \
  --skill mdmind-map-authoring \
  --skill mdm-cli-inspection \
  -a claude-code \
  -a codex
```

Install them globally for your user account:

```bash
npx skills add dudash/mdmind \
  --skill mdmind-map-authoring \
  --skill mdm-cli-inspection \
  -g \
  -a claude-code \
  -a codex
```

Use `-y` for non-interactive setup. The CLI symlinks by default; use `--copy`
only when symlinks are not acceptable.

Maintain installed skills:

```bash
npx skills list
npx skills update
npx skills remove mdmind-map-authoring mdm-cli-inspection
```

## Plugin Installs

Use standalone `npx skills` installs when you only want the two skills. Use an
agent plugin when you want a versioned, agent-native package with namespaced
skills, bundled skill assets, and install-surface metadata.

### Claude Code

mdmind ships a Claude Code marketplace at `.claude-plugin/marketplace.json` and
a plugin package at `plugins/mdmind`.

Install from GitHub:

```bash
claude plugin marketplace add dudash/mdmind --sparse .claude-plugin plugins
claude plugin install mdmind@mdmind
```

Local validation before publishing:

```bash
claude plugin validate .
claude plugin validate plugins/mdmind
```

Claude Code exposes plugin skills with the plugin namespace:

```text
/mdmind:mdmind-map-authoring
/mdmind:mdm-cli-inspection
```

### Codex

mdmind ships a Codex marketplace at `.agents/plugins/marketplace.json` and the
same shared plugin package at `plugins/mdmind`.

After pulling the repo, restart Codex or register the repo marketplace from the
repo root:

```bash
codex plugin marketplace add .
```

Then install or enable the `mdmind` plugin from the `mdmind` marketplace in the
plugin directory. Codex resolves the marketplace entry's `source.path` relative
to the repo root. The `.agents` folder is shared agent workspace convention, but
`.agents/plugins/marketplace.json` is the Codex plugin marketplace path; keep
other agent-specific marketplaces in their own namespaced folders.

## Manual Fallback

If the CLI is unavailable or you are working offline, clone the repo and copy the
skills into the path your agent supports.

```bash
git clone https://github.com/dudash/mdmind ~/mdmind
```

Project-local shared Agent Skills:

```bash
mkdir -p .agents/skills
cp -R ~/mdmind/skills/mdmind-map-authoring .agents/skills/
cp -R ~/mdmind/skills/mdm-cli-inspection .agents/skills/
```

User-level shared Agent Skills:

```bash
mkdir -p ~/.agents/skills
cp -R ~/mdmind/skills/mdmind-map-authoring ~/.agents/skills/
cp -R ~/mdmind/skills/mdm-cli-inspection ~/.agents/skills/
```

Claude Code native directories:

```bash
mkdir -p .claude/skills
cp -R ~/mdmind/skills/mdmind-map-authoring .claude/skills/
cp -R ~/mdmind/skills/mdm-cli-inspection .claude/skills/
```

```bash
mkdir -p ~/.claude/skills
cp -R ~/mdmind/skills/mdmind-map-authoring ~/.claude/skills/
cp -R ~/mdmind/skills/mdm-cli-inspection ~/.claude/skills/
```

## Agent Paths

These paths are useful when debugging a manual fallback. Prefer `npx skills` when
the agent is supported by the CLI.

| Agent or tool | Project or workspace skills | User or global skills | Notes |
|---|---|---|---|
| OpenAI Codex | `.agents/skills/` from the current working directory up to repo root | `~/.agents/skills/`; admin `/etc/codex/skills`; bundled system/plugin skills | Codex supports symlinked skill folders. For reusable distribution, prefer the Codex plugin package. |
| Claude Code | `.claude/skills/` | `~/.claude/skills/` | Claude Code skills are filesystem-based when installed standalone; for reusable distribution, prefer the Claude Code plugin package. |
| Cursor | `.agents/skills/` or `.cursor/skills/` | `~/.agents/skills/` or `~/.cursor/skills/` | Verify current Cursor docs if the skill does not appear. |
| Gemini CLI | `.agents/skills/` or `.gemini/skills/` | `~/.agents/skills/` or `~/.gemini/skills/` | Some Gemini-specific installs may prefer `.gemini/skills/`. |
| GitHub Copilot / VS Code Copilot | `.github/skills/`, `.agents/skills/`, or configured locations | `~/.copilot/skills/` or `~/.agents/skills/` | VS Code can configure additional locations. |
| OpenCode | `.opencode/skills/`, `.claude/skills/`, or `.agents/skills/` | `~/.config/opencode/skills/`, `~/.claude/skills/`, or `~/.agents/skills/` | Walks upward from the working directory for project-local skills. |
| Warp | `.agents/skills/` preferred; also several agent-native paths | same directory names under home | Warp is compatibility-friendly and recommends `.agents/skills/`. |
| Pi Coding Agent | `.pi/skills/` or `.agents/skills/` | `~/.pi/agent/skills/` or `~/.agents/skills/` | Supports repeated CLI `--skill <path>` installs too. |
| Windsurf Cascade | `.windsurf/skills/` or `.agents/skills/` | `~/.codeium/windsurf/skills/` or `~/.agents/skills/` | Enterprise paths vary by platform. |
| Kiro | `.kiro/skills/` | `~/.kiro/skills/` | Custom agents may need explicit skill resources. |

## Notes

- The `skills` CLI currently documents agent-targeted install paths that can
  differ from individual agent docs. Prefer the CLI's `-a <agent>` behavior over
  hard-coding paths in user-facing setup commands.
- OpenAI Codex docs describe `.agents/skills` and `~/.agents/skills` for direct
  skill folders, and `.agents/plugins/marketplace.json` for repo-scoped plugins.
- Claude Code plugins are cached after install, so mdmind's plugin package
  includes the skill folders under `plugins/mdmind/skills` instead of pointing
  at `../skills`.

## Sources

- OpenAI Codex skills docs: <https://developers.openai.com/codex/skills>
- OpenAI Codex plugin docs: <https://developers.openai.com/codex/plugins/build>
- Claude Code plugin docs: <https://code.claude.com/docs/en/plugins>
- Claude Code plugin marketplace docs: <https://code.claude.com/docs/en/plugin-marketplaces>
- Vercel Labs skills CLI: <https://github.com/vercel-labs/skills>
- mdmind skill pack: [../skills/README.md](../skills/README.md)
