# Mindspace

Mindspace is the optional folder-level workspace layer for mdmind.

The corrected UX bet is agent-first but still local-first: users ask Claude
Code, Codex, Hermes, or another capable agent to organize, link, generate,
move, and maintain a workspace; `mdmind .` is where humans inspect, edit,
navigate, and review the result; `mdm mindspace ...` is the deterministic
contract underneath.

It is deliberately self-contained. The core product remains:

- one file is a useful map
- one line is one node
- the tree is the primary structure
- `mdm` and `mdmind` work well on a single `.md` map

Mindspace starts only when someone wants a folder-level workspace around maps,
ordinary Markdown, sources, indexes, logs, agent sessions, reviews, and
checkpoints.

Mindspace also needs to teach users how to guide their agents. The product
model includes persona/job templates that translate vague requests like
"organize this research folder" into safe, useful Claude Code, Codex, Hermes,
or other agent workflows.

Current implemented substrate:

```bash
mdm mindspace scan . --json
mdm mindspace lint . --json
mdm mindspace setup . --preview --json
mdm mindspace setup . --write --json
mdm mindspace context maps/roadmap.md#roadmap/current --json
mdm mindspace template list --json
mdm mindspace template show launch-planning --json
```

`scan` inventories a folder without writing. `lint` reports deterministic
Mindspace diagnostics with stable issue codes and exits non-zero only on
errors. `setup --preview` prints the proposed manifest without writing, while
`setup --write` writes only `.mdmind/mindspace.json`. `context` exports bounded,
provenance-rich branch bundles for agents without writing. Template helpers
expose built-in persona/job templates for agents and scripts. Session, review,
and `mdmind .` workspace behavior are still planned slices.

## Start Here

- [VALUE.md](VALUE.md): user value, real-world examples, and the product wedge.
- [EXPERIENCE_MODEL.md](EXPERIENCE_MODEL.md): the agent-first, `mdmind`-reviewed, CLI-backed UX model.
- [JOB_TEMPLATES.md](JOB_TEMPLATES.md): persona-shaped job templates, customization knobs, and helper command targets.
- [AGENT_SKILL.md](AGENT_SKILL.md): packaged Mindspace workflow skill and reference-file structure.
- [USER_STORIES.md](USER_STORIES.md): detailed persona journeys, aha moments, and delight tests for the target Mindspace experience.
- [REFERENCE.md](REFERENCE.md): manifest schema, core objects, command vocabulary, JSON formats, phases, and safety model.
- [BOUNDARIES.md](BOUNDARIES.md): how Mindspace stays optional and avoids bleeding into core map behavior.
- [NAVIGATION.md](NAVIGATION.md): how the TUI should handle multiple files without becoming a vault browser.
- [TIPS.md](TIPS.md): practical guidance for future Mindspace docs, CLI, TUI, and agent work.
- [../product/prds/mindspace-user-need-and-ecosystem-fit.md](../product/prds/mindspace-user-need-and-ecosystem-fit.md): product rationale and ecosystem fit.
- [../design/LLM_WIKI_MINDSPACE_STRATEGY.md](../design/LLM_WIKI_MINDSPACE_STRATEGY.md): historical strategy input.

## Repo Rule

Mindspace docs, design notes, tips, and contracts live here by default.

Core docs may link here, but they should not absorb Mindspace concepts unless
the concept is also true for single-file maps.

Examples:

- A new `.mdmind/mindspace.json` field belongs in this section.
- A `mdm mindspace scan` JSON format belongs in this section and the agent CLI
  contract, but user-facing docs should lead with the agent or `mdmind` flow
  unless they are explicitly about scripting.
- A new map syntax feature belongs in `docs/reference/`, with a note here only
  if Mindspace uses it.
- A TUI workspace switcher or cross-file navigation design belongs here; a
  general focus-mode keybinding belongs in the TUI/manual docs.
