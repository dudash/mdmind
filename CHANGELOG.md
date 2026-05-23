# Changelog

All notable user-facing changes to mdmind are tracked here.

Future entries should be curated before release. Older entries are a best-effort
backfill from local tags, product docs, and commit history, so they summarize
the release shape rather than every internal commit.

## [0.8.0] - 2026-05-23

### Features

- **Curated release notes now live with the product** instead of being scattered
  across commits, so users, agents, GitHub releases, and the TUI can all tell
  the same story.
- The TUI now has a "What's New" help topic, giving users a lightweight way to
  discover the most relevant changes without leaving their map.
- The TUI can now check for a newer mdmind release on demand from the palette,
  then quietly highlights "New version X available" at the top of Help when
  there is something to upgrade to.
- **Agent mode is now one command to try**: install the mdmind skill with
  `npx skills add dudash/mdmind`, pick your agent, and ask it to build research
  maps, decompose problems, draft plans, or turn messy notes into a shape humans
  can keep editing.
- Agent-facing output is getting sturdier: JSON envelopes and command
  discovery make it much easier for tools to decide what is safe to run.

### New Commands

- `mdm import`: bring OPML, FreeMind, Markdown outlines, local HTML, and rough
  web pages into native mdmind maps, with preview/report modes for safer review.
- `mdm commands`: expose the CLI command catalog, output modes, safety metadata,
  and examples so agents and scripts can discover the tool instead of guessing.
- `mdm changelog`: read bundled release notes from the CLI, including versioned
  output and JSON for automation.

### Changed

- Agent skill install is now centered on one memorable command:
  `npx skills add dudash/mdmind`.
- `mdm version` now accepts `--check` and `--json`, giving users and agents a
  manual update check without making startup depend on the network.
- The plugin skill package is the source of truth now, so marketplace installs
  and local plugin packaging do not drift away from each other.
- The docs folder is calmer: manuals, embedded help, reference material, agent
  docs, design notes, and shipped-product summaries each have a clearer home.

## [0.7.0] - 2026-05-13

### Features

- **TODO maps became first-class**, so project plans can carry real `[ ]` and
  `[x]` work without losing the calm outline shape.
- **Metadata Table View** turned repeated `@key:value` fields into a scannable
  column surface, making owner/status/source comparisons much easier inside
  large maps.
- Table View kept leveling up across the release: selectable columns, scoped
  views, remembered choices, and data-focused defaults made it feel like a real
  working lens instead of a debug table.
- Agent TODO guidance, a TODO template, and a model benchmark example gave teams
  concrete patterns for shared decomposition, handoff, and structured review.
- External references became easier to see and trust, with TUI previews plus
  help and validation polish around files, URLs, links, and images attached to
  nodes.

### New Commands

- `mdm refs`: inspect Markdown links, local files, URLs, and image references
  attached to map nodes.
- `mdm check-keys`: diagnose terminal Alt/Option-arrow behavior when movement
  shortcuts are not being reported correctly.

### Changed

- Table defaults became more useful out of the box, favoring repeated data
  fields instead of noisy derived columns.

## [0.6.0] - 2026-05-09

### Features

- **Spatial Canvas arrived as the first serious visual navigation surface**,
  letting users move through a map spatially while staying connected to the
  outline.
- **The official mdmind format spec landed in `spec/`**, with fixtures and a
  conformance manifest so the plain-text format could start becoming a real
  contract instead of only an implementation detail.
- Public adoption got a stronger face: the landing site, character assets, and
  agent-facing docs made mdmind easier to understand before cloning the repo.
- Agent-facing docs and skill packaging made it much easier to teach tools how
  to author and inspect mdmind maps without a long prompt lecture.

### Changed

- Spatial navigation became smoother around layout, link handling, and leaf-node
  behavior, which made the canvas feel more like a working surface and less like
  an experiment.
- Prompt editing got friendlier around inline syntax, ids, and relation targets,
  giving users more useful feedback before they committed a line.
- Generated example sidecars stopped cluttering the repo.

## [0.5.0] - 2026-05-02

### Features

- **The theme set grew from seven to twelve built-in surfaces**, adding
  Amethyst, Atelier, Archive, Signal, and Tokyo Mind alongside Workbench, Paper,
  Blueprint, Calm, Violet, Monograph, and Terminal Neon.
- Themes became more than background colors: tags, metadata, ids, query text,
  relations, counts, selections, and attention cues all got semantic colors, so
  dense maps read faster at a glance.
- The public website and theme presentation got a stronger first impression, so
  mdmind felt more approachable as a product rather than only a repo.

## [0.4.1] - 2026-04-30

### Features

- **AI agents could make mdmind maps for the first time**: the portable skill
  pack taught Codex, Claude, and other skill-aware agents how to author maps for
  research, problem decomposition, planning, and handoff.
- The CLI inspection skill gave agents a dedicated way to inspect existing maps
  without relying on ad hoc prompt memory.

### Changed

- The website was tightened up so unfinished surfaces stayed out of the way.

## [0.4.0] - 2026-04-28

### Features

- mdmind got a public Pages site, giving new users a real landing point for what
  the tool is and how to install it.
- Release and Homebrew automation started to feel like a normal install story
  instead of a local-build-only project.
- **Branch Index added a Sections shelf to the palette**, giving large maps a
  fast "show me the major rooms" jump surface without creating another view
  mode.
- The startup and open flow got friendlier, so opening examples, templates, and
  docs felt less like spelunking through repo internals.
- Agent usage docs and research handoff examples made mdmind easier to explain
  as a shared human-agent artifact, not just a personal outline file.
- The landing experience became friendlier on mobile.

### Fixed

- Cleaned up early public-site and lint issues that made release confidence
  noisier than it needed to be.

### Closed Issues

- Closed [#1](https://github.com/dudash/mdmind/issues/1): startup could now
  create the file/path structure a new user needed, making first launch feel
  like an invitation instead of a scavenger hunt.

## [0.3.1] - 2026-04-11

### Fixed

- Tightened the release process after the `v0.3.0` push.

## [0.3.0] - 2026-04-11

### Features

- Bundled examples made mdmind easier to learn by opening real maps instead of
  starting from a blank file.
- Reading mode improved the long-note experience, making node details feel less
  cramped when a branch needs real prose.
- Release automation became less manual, which made shipping binaries more
  repeatable.

### New Commands

- `mdm examples list`: see the example maps bundled with the release.
- `mdm examples path`: locate installed examples when they are available on
  disk.
- `mdm examples copy`: materialize one example, or the whole gallery, into a
  local folder.

### Fixed

- Fixed GitHub Actions release setup so published builds were easier to trust.

## [0.2.0] - 2026-04-11

### Features

- **Visual mindmap display and PNG export** gave users a second lens on their
  outline when hierarchy was easier to understand spatially.
- **The command palette became the "take me there" surface**, pulling branch
  jumps, ids, actions, saved views, relations, history, help, recipes, and
  settings into one searchable place.
- View modes made large maps less overwhelming by letting users switch between
  full-map orientation and focused work.
- **Relations, ids, node details, and richer search** started turning plain
  outlines into durable knowledge maps.
- The first real theme system arrived with Workbench, Paper, Blueprint, Calm,
  Violet, Monograph, and Terminal Neon, plus live palette preview, per-map UI
  settings, minimal mode, ASCII accents, and attention-guiding motion toggles.
- Undo, redo, manual checkpoints, automatic safety snapshots, and autosave/manual
  save mode made bold restructuring feel much less scary.
- Mermaid and OPML exports opened the door to sharing maps with diagram and
  outliner tools.
- Reading mode made longer detail text more comfortable to review inside the
  terminal.

### New Commands

- `mdm relations`: inspect outgoing links across a map, or incoming/outgoing
  context for a deep-linked branch.

### Changed

- Built-in help grew alongside the TUI, so users could discover new surfaces
  without leaving the app.

## [0.1.2] - 2026-04-06

### Changed

- The manual release path became less fragile, which made early binaries easier
  to cut and verify.

## [0.1.1] - 2026-04-06

### Features

- **The first public shape of mdmind shipped**: a plain-text map format, a
  focused terminal UI, and a companion CLI over the same files.
- Search, filters, facets, and saved views made early maps useful beyond simple
  scrolling.
- The repo gained the license, build setup, and release scaffolding needed to
  become something people could actually install and try.

### New Commands

- `mdm view`: render a readable tree, including deep-linked subtrees.
- `mdm find`: search labels, tags, metadata, and ids from the shell.
- `mdm tags`: summarize the tag vocabulary in a map.
- `mdm kv`: inspect inline `@key:value` metadata as rows.
- `mdm links`: list stable ids and deep-linkable targets.
- `mdm validate`: check parser structure, duplicate ids, and metadata
  conventions before trusting a map.
- `mdm export`: export normalized map data, starting with JSON.
- `mdm init`: create new maps from starter templates.
- `mdm open`: jump from the CLI into the interactive TUI.
- `mdm version`: print the installed CLI version.
