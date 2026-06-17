# Mindspace Tips

These notes are for current and future implementation and docs work.

## Lead With The Agent Loop

Mindspace docs should usually start with what the user asks their agent to do,
then explain how `mdmind` and `mdm` support that request.

Good:

```text
Organize this research folder. Keep sources read-only, build a claims map, link
each claim to evidence, and leave risky edits for review.
```

Then show the supporting surfaces:

- the agent uses `mdm mindspace scan`, `lint`, `context`, `session`, and
  `review`
- the human opens `mdmind .` to inspect, edit, navigate, and approve
- files remain ordinary Markdown, mdmind maps, and small sidecars

Avoid making the first user story "run five commands."

## Keep Adoption Explicit

Start with inspection, whether a human or agent triggers it:

```bash
mdm mindspace scan .
mdm mindspace setup . --preview
```

Write a manifest only when the user approves:

```bash
mdm mindspace setup . --write
```

Do not auto-adopt a folder just because a user opened `mdmind .` or an agent
entered the directory.

Today, `scan` and `lint` are the only implemented Mindspace commands. Treat
their read-only output as the foundation for every later setup, context,
session, review, or TUI workspace slice.

## Keep Files Ordinary

Mindspace should organize local files, not hide them.

- Native maps stay normal `.md` map files.
- Ordinary Markdown pages stay ordinary Markdown.
- Sources remain readable local files.
- Generated reports should be inspectable text or JSON.
- `.mdmind/` sidecars should be small and explainable.
- Agent-created pages and maps should look hand-editable afterward.

## Treat The CLI As Contract, Not Main UX

Use `mdm mindspace ...` for workspace operations:

- `scan`
- `setup`
- `lint`
- `context`
- `session`
- `review`

But in user-facing Mindspace stories, the CLI should usually be "under the
hood." Direct CLI examples are still useful for Mateo-style power users,
scripts, docs contracts, and tests.

Avoid scattering workspace concepts across unrelated command groups or inventing
a broad `mdm agent` namespace.

## Protect The Core Editor

Mindspace may enrich `mdmind .`, but it should not make `mdmind file.md`
busier.

Prefer palette entries, status messages, workspace landing states, and optional
review surfaces over permanent chrome.

For multiple files, prefer a workspace switcher, peek/open flow, back/forward
stack, recents, and pinned maps over a permanent file tree. See
[NAVIGATION.md](NAVIGATION.md).

## Make Agent Work Visible

When an agent moves, links, generates, or edits files, Mindspace should leave a
human-readable trail:

- what request started the work
- what target branch or file was used
- which context bundle was included
- what files changed or were proposed
- which validation checks passed or failed
- whether a target digest went stale
- what needs review in `mdmind .`

The user should be able to reconstruct the agent's working set without reading
the whole chat.

## Make Safety Visible

For workspace writes, show what will change before it changes:

- preview setup manifests
- checkpoint before risky writes
- turn stale-digest applies into review items
- keep source folders read-only by default
- separate trusted instruction files from untrusted source content
- prefer scoped sessions for agent writeback

## Keep Agent Context Bounded

Context bundles should explain why each item was included.

Good bundles include:

- target branch or file
- relevant children
- incoming backlinks
- selected outgoing relations
- source references
- provenance and omission reasons
- explicit budgets

Avoid whole-folder dumps as the default. The agent should ask Mindspace for the
right slice, not build its own private model from arbitrary file reads.
