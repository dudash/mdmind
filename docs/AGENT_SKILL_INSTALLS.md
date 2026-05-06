# Agent Skill Install Notes

Last reviewed: 2026-05-06

Agent Skill directory conventions are still changing. Prefer the shared
`.agents/skills/` layout when your agent supports it, but check your agent's
current docs before assuming one path works everywhere.

## Recommended mdmind Installs

Install mdmind skills for one project:

```bash
mkdir -p .agents/skills
cp -R ~/mdmind/skills/mdmind-map-authoring .agents/skills/
cp -R ~/mdmind/skills/mdm-cli-inspection .agents/skills/
```

Install mdmind skills for all your work:

```bash
mkdir -p ~/.agents/skills
cp -R ~/mdmind/skills/mdmind-map-authoring ~/.agents/skills/
cp -R ~/mdmind/skills/mdm-cli-inspection ~/.agents/skills/
```

These shared paths work well for agents that support the Agent Skills convention.
Claude Code currently uses `.claude/skills/` and `~/.claude/skills/` instead.

## Agent Paths

| Agent or tool | Project or workspace skills | User or global skills | Notes |
|---|---|---|---|
| OpenAI Codex | `.agents/skills/` from the current working directory up to repo root | `~/.agents/skills/`; admin `/etc/codex/skills`; bundled system/plugin skills | Codex supports symlinked skill folders. Duplicate skill `name` values are not merged; both can appear. |
| Claude Code | `.claude/skills/` | `~/.claude/skills/` | Claude Code skills are filesystem-based and separate from Claude.ai/API skills. The community has requested `.agents/skills/` support, but official docs still name `.claude/skills/`. |
| Cursor | `.agents/skills/` or `.cursor/skills/` | `~/.agents/skills/` or `~/.cursor/skills/` | Verify current Cursor docs if the skill does not appear. |
| Gemini CLI | `.agents/skills/` or `.gemini/skills/` | `~/.agents/skills/` or `~/.gemini/skills/` | Some Gemini-specific installs may prefer `.gemini/skills/`. |
| GitHub Copilot / VS Code Copilot | `.github/skills/`, `.agents/skills/`, or configured locations | `~/.copilot/skills/` or `~/.agents/skills/` | VS Code can configure additional locations. |
| OpenCode | `.opencode/skills/`, `.claude/skills/`, or `.agents/skills/` | `~/.config/opencode/skills/`, `~/.claude/skills/`, or `~/.agents/skills/` | Walks upward from the working directory for project-local skills. |
| Warp | `.agents/skills/` preferred; also several agent-native paths | same directory names under home | Warp is compatibility-friendly and recommends `.agents/skills/`. |
| Pi Coding Agent | `.pi/skills/` or `.agents/skills/` | `~/.pi/agent/skills/` or `~/.agents/skills/` | Supports repeated CLI `--skill <path>` installs too. |
| Windsurf Cascade | `.windsurf/skills/` or `.agents/skills/` | `~/.codeium/windsurf/skills/` or `~/.agents/skills/` | Enterprise paths vary by platform. |
| Kiro | `.kiro/skills/` | `~/.kiro/skills/` | Custom agents may need explicit skill resources. |

## Alternatives

If you already use Vercel Labs' open `skills` CLI, preview or install mdmind
directly from GitHub:

```bash
npx skills add dudash/mdmind --list
npx skills add dudash/mdmind --skill mdmind-map-authoring --skill mdm-cli-inspection
```

This is a convenience path. The checked-in skill folders under `skills/` remain
the source of truth.

## Sources

- OpenAI Codex skills docs: <https://developers.openai.com/codex/skills#where-to-save-skills>
- Claude Agent Skills sharing scope: <https://platform.claude.com/docs/en/agents-and-tools/agent-skills/overview#sharing-scope>
- Claude Code community request for `.agents/skills/`: <https://github.com/anthropics/claude-code/issues/31005>
- mdmind skill pack: [../skills/README.md](../skills/README.md)
