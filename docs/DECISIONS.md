# Decisions

This file records the current direction for the app


## 1. CLI design follows clig.dev principles

Decision:
- Use subcommands, human-friendly help, stdout/stderr separation, explicit output modes, and predictable exit codes.

Rationale:
- Better composability
- Better scripting
- Better UX for both humans and tools

## 2. Plain-text format remains core

Decision:
- Keep a markdown-like, human-readable tree format with inline annotations.

Rationale:
- Greppable
- Git-friendly
- Easy to inspect and edit anywhere
- AI-friendly as source context

## 3. Separate CLI and TUI frontends

Decision:
- `mdm` is the CLI
- `mdmind` is the full-screen TUI

Rationale:
- Clearer mental model
- Easier architecture
- Cleaner future packaging

## 4. Start with core + CLI before deep TUI investment

Decision:
- Build parser/model/search/serialization and data commands first.

Rationale:
- Strong core reduces risk
- TUI quality depends on stable model and command semantics

## 5. Core format features supported from early milestones

- `#tags`
- `@key:value`
- `[id:path/to/node]`
- deep-link open via `file.md#path/to/node`

##  8. Initial focus areas

- idea exploration
- TODO / status tracking
- prompt refinement
- project templates

## 9. Do not start with

- web service
- collaboration
- plugins
- AI integrations
- Mermaid-native editing

Those can come later if the core product becomes sticky.
