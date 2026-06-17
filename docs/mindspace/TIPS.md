# Mindspace Tips

These notes are for future implementation and docs work.

## Keep Adoption Explicit

Start with inspection:

```bash
mdm mindspace scan .
mdm mindspace setup . --preview
```

Write a manifest only when the user asks:

```bash
mdm mindspace setup . --write
```

Do not auto-adopt a folder just because a user opened `mdmind .`.

## Keep Files Ordinary

Mindspace should organize local files, not hide them.

- Native maps stay normal `.md` map files.
- Ordinary Markdown pages stay ordinary Markdown.
- Sources remain readable local files.
- Generated reports should be inspectable text or JSON.
- `.mdmind/` sidecars should be small and explainable.

## Prefer One Workspace Namespace

Use `mdm mindspace ...` for workspace operations:

- `scan`
- `setup`
- `lint`
- `context`
- `session`
- `review`

Avoid scattering workspace concepts across unrelated command groups.

## Protect The Core Editor

Mindspace may enrich `mdmind .`, but it should not make `mdmind file.md`
busier.

Prefer palette entries, status messages, workspace landing states, and optional
review surfaces over permanent chrome.

For multiple files, prefer a workspace switcher, peek/open flow, back/forward
stack, recents, and pinned maps over a permanent file tree. See
[NAVIGATION.md](NAVIGATION.md).

## Make Safety Visible

For workspace writes, show what will change before it changes:

- preview setup manifests
- checkpoint before risky writes
- turn stale-digest applies into review items
- keep source folders read-only by default
- separate trusted instruction files from untrusted source content

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

Avoid whole-folder dumps as the default.
