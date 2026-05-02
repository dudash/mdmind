<p align="center">
  <img src="docs/assets/mdmind-logo.png" alt="mdmind" width="560">
</p>

<p align="center">
  <a href="https://github.com/dudash/mdmind/actions/workflows/ci.yml"><img src="https://img.shields.io/github/actions/workflow/status/dudash/mdmind/ci.yml?branch=main&label=ci" alt="CI status"></a>
  <a href="https://github.com/dudash/mdmind/releases"><img src="https://img.shields.io/github/v/release/dudash/mdmind?label=release" alt="Latest release"></a>
  <a href="https://crates.io/crates/mdmind"><img src="https://img.shields.io/crates/v/mdmind" alt="Crates.io version"></a>
  <img src="https://img.shields.io/badge/rust-1.85%2B-f05f40" alt="Rust 1.85+">
  <a href="LICENSE"><img src="https://img.shields.io/badge/license-Apache--2.0-blue" alt="Apache-2.0 license"></a>
  <a href="https://github.com/sponsors/dudash"><img src="https://img.shields.io/github/sponsors/dudash?label=sponsor&logo=githubsponsors" alt="Sponsor on GitHub"></a>
</p>

`mdmind` is a local-first thinking tool for structured maps in plain text.

It gives you two interfaces over the same file:

- `mdm`: a CLI for viewing, searching, validating, exporting, and copying examples
- `mdmind`: a full-screen TUI for navigating, filtering, editing, and reshaping maps

Large idea trees stay calm, searchable, and safe to edit with a keyboard.

License: Apache-2.0

![mdmind screenshot](docs/assets/novel-focus-branch.png)

## Why mdmind

`mdmind` is good for:

- product and feature planning
- research and writing maps
- prompt libraries
- project breakdowns
- backlog shaping
- keyboard-first personal planning

It is not trying to be:

- a rich document editor
- a team wiki
- a freeform diagram canvas

## What A Map Looks Like

Maps are plain-text tree files with lightweight inline structure:

- `#tag` for grouping and workflow markers
- `@key:value` for structured metadata
- `[id:path/to/node]` for stable deep links
- `[[target/id]]` or `[[rel:kind->target/id]]` for cross-branch references
- `| detail text` for longer notes attached to a node

Example:

```text
- Onboarding Research #project @status:active [id:onboarding]
  | Turn scattered notes, interviews, and generated research into a decision map.
  - Core Question #question [id:onboarding/question]
    - Where do new users lose momentum first?
  - Evidence #source [id:onboarding/evidence] [[rel:informs->onboarding/decision]]
    - Interview notes mention setup vocabulary friction
    - Support tickets cluster around first-map examples
  - Decision #decision @owner:jason [id:onboarding/decision]
    - Ship a guided starter map before adding more settings
  - Follow-ups #todo @status:active [id:onboarding/follow-ups] [[onboarding/evidence]]
    - Review five more sessions
    - Draft release note
```

These files stay readable in normal Markdown tools. `mdmind` adds structure and navigation on top of that plain-text shape.

## Install

For public installs, GitHub Releases are the source of truth.

- macOS:

```bash
brew tap dudash/tap
brew install mdmind
```

- Linux: install from the release tarball
- Windows: install from the release zip

For local development from this repo:

```bash
cargo install --path .
```

That installs both:

- `mdm`
- `mdmind`

More install and release detail lives in [docs/INSTALL_AND_RELEASE.md](docs/INSTALL_AND_RELEASE.md).

For Codex, Claude, or other agent clients, see [skills/README.md](skills/README.md).

## Quick Start

Create a map from a starter template:

```bash
mdm init roadmap.md --template product
```

Open the TUI:

```bash
mdmind roadmap.md
```

Inspect a map from the CLI:

```bash
mdm view roadmap.md
mdm find roadmap.md "#todo"
mdm links roadmap.md
```

Copy bundled example maps onto your machine:

```bash
mdm examples list
mdm examples copy all
```

## Core Ideas

- one plain-text map format, two interfaces
- local-first files with small sidecars for session state, views, checkpoints, navigation memory, and UI settings
- focused views for working inside large maps without losing structure
- built-in search, browse, saved views, ids, relations, and detail notes
- safe editing with undo, redo, checkpoints, and autosave/manual save modes

## Read Next

If you are new:

- [docs/README.md](docs/README.md)
- [docs/USER_GUIDE.md](docs/USER_GUIDE.md)
- [docs/TUI_WORKFLOWS.md](docs/TUI_WORKFLOWS.md)
- [docs/USING_MDMIND_AS_OUTLINER.md](docs/USING_MDMIND_AS_OUTLINER.md)
- [docs/AGENT_USAGE.md](docs/AGENT_USAGE.md)

If you want specific features:

- [docs/QUERY_LANGUAGE.md](docs/QUERY_LANGUAGE.md)
- [docs/IDS_AND_DEEP_LINKS.md](docs/IDS_AND_DEEP_LINKS.md)
- [docs/CROSS_LINKS_AND_BACKLINKS.md](docs/CROSS_LINKS_AND_BACKLINKS.md)
- [docs/NODE_DETAILS.md](docs/NODE_DETAILS.md)
- [docs/SKILLS.md](docs/SKILLS.md)
- [docs/SAFETY_AND_HISTORY.md](docs/SAFETY_AND_HISTORY.md)
- [docs/TEMPLATES.md](docs/TEMPLATES.md)
- [examples/README.md](examples/README.md)

If you want product status and roadmap shelves:

- [docs/product/README.md](docs/product/README.md)
- [docs/product/features/finished/README.md](docs/product/features/finished/README.md)
- [docs/product/features/inwork/README.md](docs/product/features/inwork/README.md)
- [docs/product/features/future/README.md](docs/product/features/future/README.md)

If you are working on the repo:

- [DEVELOPER.md](DEVELOPER.md)
