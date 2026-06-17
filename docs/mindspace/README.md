# Mindspace

Mindspace is the optional folder-level workspace layer for mdmind.

It is deliberately self-contained. The core product remains:

- one file is a useful map
- one line is one node
- the tree is the primary structure
- `mdm` and `mdmind` work well on a single `.md` map

Mindspace starts only when someone wants a folder-level workspace around maps,
ordinary Markdown, sources, indexes, logs, agent sessions, reviews, and
checkpoints.

## Start Here

- [VALUE.md](VALUE.md): user value, real-world examples, and the product wedge.
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
  contract.
- A new map syntax feature belongs in `docs/reference/`, with a note here only
  if Mindspace uses it.
- A TUI workspace switcher or cross-file navigation design belongs here; a
  general focus-mode keybinding belongs in the TUI/manual docs.
