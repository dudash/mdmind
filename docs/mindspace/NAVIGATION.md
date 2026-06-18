# Mindspace Navigation

Mindspace needs stronger multi-file navigation than the single-file TUI has
today, because agents will be creating, moving, linking, and generating files
across a workspace. The TUI is where humans inspect and edit that structured
result, but it should not become an Obsidian-style vault browser.

Obsidian is page-first and many-files-first. `mdmind` is map-first and
tree-first. Agent conversation is request-first. Mindspace should connect those
realities: the agent can organize the folder, and `mdmind .` should make the
result navigable while preserving one active editing surface.

## Current Baseline

The current TUI is centered on one map file:

- `mdmind file.md` opens one native map for editing.
- Relations and backlinks jump inside that map.
- Attached refs can be previewed and opened externally.
- Ordinary Markdown can open in a read-only document view.

That is a good base, but it is not enough for a mindspace. A mindspace may
contain multiple native maps, ordinary Markdown pages, sources, logs, indexes,
reports, sessions, reviews, and checkpoints.

## Product Direction

Yes, Mindspace should strengthen external-file navigation. The right model is
workspace navigation, not general file browsing.

The TUI should help users move among known workspace objects, especially after
an agent has proposed or written changes:

- maps
- branch ids
- cross-file relations
- backlinks
- recent maps
- pinned maps
- source references
- index/log/report files
- session and review records

It should avoid making the left side of the product a permanent file tree.

The user should be able to ask an agent to create a map, link a source, move an
inbox item, or generate a report, then open `mdmind .` and immediately see where
that work landed. If the work used a job template, the landing should also make
the template legible: what job was requested, which files were touched, what the
template expected, and what still needs review.

## Interaction Model

### 1. One Active Map

There is still one active editable map at a time.

Opening another map switches the active map and pushes the previous file and
branch onto a navigation stack. Back/forward should restore file plus branch
focus.

This keeps editing calm and avoids multi-pane state.

### 2. Peek Before Open

Cross-file targets should support a quick peek when possible:

- native map branch: show label, breadcrumbs, details preview, file path, and
  validation status
- ordinary Markdown page: show title/heading preview and role
- source file: show path, type, digest/mtime when available, and read-only
  status
- agent-generated file: show generator/session, target, validation state, and
  whether it is review-only
- missing target: show a clear unresolved-target message and suggested
  follow-up

Peek answers "is this the thing I mean?" without changing the active map.

### 3. Open Or Switch

When the user commits:

- native map branch opens in the map editor, focused on the target branch
- native map file without an id opens at its remembered or first meaningful
  focus
- ordinary Markdown opens in read-only Markdown view
- source files open in read-only preview when supported, or externally
- generated reports open read-only unless an explicit write/rebuild command is
  used
- session and review records open in dedicated workspace views when those
  features exist

The user should always know whether they are editing a map, reading a page, or
reviewing a generated object. Agent-created content should never feel
indistinguishable from human-authored content until the user accepts it.

### 4. Workspace Switcher

`mdmind .` should introduce a universal workspace switcher over the manifest or
scan inventory.

Switcher entries should include:

- native maps
- branch ids
- tags and saved searches across maps
- recent map and branch targets
- pinned maps
- index/log/report files
- open review items
- sessions when they exist

This is the primary many-file affordance. It should be searchable and keyboard
first.

When an agent has just worked on the mindspace, the switcher should make the
agent's touched files and review items easy to find without turning them into
permanent chrome.

### 5. Role-Aware Browse

A light browse surface is still useful, but it should be role-aware rather than
a general filesystem tree.

Good browse groups:

- Maps
- Pages
- Sources
- Inbox
- Indexes And Logs
- Reports
- Sessions
- Reviews

This lets users orient in a mindspace without competing with dedicated file
managers or Obsidian's vault tree.

Current slice:

- `mdmind --preview .` prints a read-only workspace landing with manifest
  health, role counts, review summary, session summary, recent sessions, open
  reviews, and maps.
- `mdmind .` opens a searchable workspace switcher in an interactive terminal.
- Switcher entries include reviews, recent sessions, maps, and role-aware files.
- Enter opens the selected target through the existing single-file map editor
  or read-only Markdown view.
- Forced single-file modes remain strict: `mdmind --as map .` and
  `mdmind --as markdown .` do not become hidden Mindspace shortcuts.

### 6. Working Set

Users need a small remembered set more than they need a full file browser:

- recent map stack
- pinned maps
- last focused branch per map
- back/forward across file and branch jumps
- files touched by the latest agent session
- optional "workspace landing" that shows current work, warnings, and open
  reviews
- active or recent job template with its review checklist

The working set is the calm replacement for leaving a file tree open all the
time.

## Template-Aware Landing

Job templates should shape the workspace landing without making the TUI busy.

Examples:

- Priya's launch planning template emphasizes current work, blocked branches,
  risks, decisions, and review items.
- Mateo's project memory template emphasizes the active task branch, bounded
  context, decisions, debug notes, and memory update proposals.
- Ren's story continuity template emphasizes the chapter or scene, linked
  characters and places, continuity risks, and story-bible updates.
- Nova's claims and evidence template emphasizes claims, source previews, weak
  evidence, open questions, and stale-source review.

The landing should answer "what did the agent try to do, and where should I
look first?" It should not become a dashboard framework for every possible
template. The template provides ranking and grouping hints for existing
workspace objects: maps, pages, sources, reports, sessions, and review items.

## What Happens To Existing Reference Preview?

The current referenced-link preview should become one layer of Mindspace
navigation:

- same-file refs keep using the relation/backlink picker
- path-qualified map refs can peek and then switch maps
- ordinary file refs can peek or open read-only
- non-text files remain external-open or metadata-only preview
- proposed agent write targets can show diff, rationale, validation, and stale
  digest status

The preview should answer "what is this target?" The switcher and navigation
stack answer "move me there and let me come back."

## Non-Goals

Mindspace TUI navigation should not initially provide:

- a permanent Obsidian-style file tree
- multi-pane document layouts
- freeform vault tagging for every Markdown file
- background ingestion when opening a folder
- automatic conversion of ordinary Markdown to mdmind maps
- an embedded agent chat UI

Those features would make Mindspace feel like a second notes app instead of an
optional workspace layer. Agent conversation can stay in Claude Code, Codex,
Hermes, or another agent surface; `mdmind` should be the place for local
inspection, editing, navigation, and review.

## MVP Navigation Slice

The first useful navigation slice should include:

1. `mdmind .` opens a workspace landing from scan/manifest inventory.
2. The landing can show the active/recent job template and latest agent-touched
   files when that information exists.
3. A searchable switcher opens maps and branch ids.
4. Cross-file map refs can peek, open, and return via back navigation.
5. Recent maps and last focused branch per map are remembered.
6. Missing targets show a helpful unresolved-target state.
7. Ordinary Markdown and sources remain read-only unless explicitly imported or
   edited through a future review flow.
8. Agent-touched files and proposed review items are discoverable from the
   landing and switcher.

This gives Mindspace its own many-file feel without losing the single-map heart
of mdmind.
