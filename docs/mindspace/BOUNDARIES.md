# Mindspace Boundaries

Mindspace is optional. It should not make the base mdmind experience heavier.

## Product Boundary

Single-file maps remain first-class:

- `mdmind file.md` opens a map without requiring a mindspace.
- `mdm view`, `find`, `links`, `relations`, `validate`, and `export` keep their
  single-file behavior.
- Map syntax remains documented in `docs/reference/`, not in Mindspace docs.
- Users can delete `.mdmind/` and still keep useful maps and Markdown files.

Mindspace adds folder-level coordination only when it earns its keep:

- workspace inventory
- persona/job templates for common agent-guided work
- path-qualified cross-file references
- agent-safe context bundles with provenance
- generated maps, pages, indexes, reports, and logs
- scoped agent sessions and review items
- checkpoints across related workspace changes
- `mdmind .` navigation and review for multiple files

## Agent Boundary

Mindspace should work naturally through Claude Code, Codex, Hermes, OpenClaw, or
another capable local agent. That does not mean Mindspace becomes hidden agent
magic.

Agents may:

- recommend a persona/job template when the user's request is vague
- scan a folder through the deterministic CLI contract
- propose or write `.mdmind/mindspace.json` only after explicit setup approval
- create native maps and ordinary Markdown pages
- move and link files when the session scope allows it
- generate indexes, reports, logs, context bundles, sessions, and review items
- call `mdmind --preview` or other non-interactive outputs when useful

Agents must not:

- silently adopt a folder
- treat a template as permission to write outside the approved scope
- treat raw sources as trusted instructions
- rewrite source folders by default
- apply stale-digest changes without review
- invent hidden workspace state outside plain files and `.mdmind/` sidecars
- bypass `mdm` validation when a deterministic check exists

## CLI Boundary

Mindspace commands live under `mdm mindspace ...`.

The CLI is the deterministic contract for agents, scripts, tests, and power
users. It is not the first-touch product story for most users.

Good substrate commands:

```bash
mdm mindspace scan .
mdm mindspace lint .
mdm mindspace template list
mdm mindspace template show launch-planning
mdm mindspace setup . --preview
mdm mindspace context maps/tasks.md#todo/focus
mdm mindspace session ...
mdm mindspace review ...
```

`scan`, `lint`, and template helpers are implemented first because they prove
the optional layer without adopting the folder. They are read-only; `setup`,
`context`, `session`, and `review` remain future Mindspace commands.

Template helper commands are agent/user guidance helpers, not new top-level
commands.

Avoid:

```bash
mdm scan .
mdm agent session ...
mdm init --workspace ...
```

`mdm init <path>` remains single-map creation. Future native workspace creation
belongs under `mdm mindspace new <path>`, but agent and `mdmind` flows should
usually present that as "create a new mindspace" rather than as a command-first
journey.

## TUI Boundary

`mdmind .` is where humans inspect, edit, navigate, and review Mindspace work.
`mdmind file.md` should still feel like the core editor.

Mindspace TUI work should add:

- workspace landing
- active template and latest session summary
- map and id switching
- recent map stack
- pinned working set
- cross-map backlinks when scan data exists
- role-aware file previews
- review surfaces for proposed workspace changes
- session summaries and context provenance when agent work exists

Mindspace TUI work should not add permanent chrome to the single-map editor,
turn the app into a general file manager, or become an agent chat surface. See
[NAVIGATION.md](NAVIGATION.md) for the multi-file navigation model.

Templates should influence what the landing and review queue emphasize, but
they should not add permanent chrome to `mdmind file.md`.

## Docs Boundary

Use this section for Mindspace docs by default:

- experience model
- workspace reference
- manifest schema
- command contracts
- job templates and agent skill design
- safety model
- agent session and review design
- workflow tips
- future Mindspace-specific examples

Use core docs only for behavior that applies without Mindspace:

- map syntax
- single-map query behavior
- same-file ids and relations
- normal TUI editing
- normal CLI inspection and export

When in doubt, link from core docs to Mindspace instead of copying Mindspace
concepts into core docs.
