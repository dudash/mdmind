# Import And Ingestion

## Goal

Let `mdmind` pull useful structure in from common map tools, outline formats, and best-effort external content.

This should be a CLI-first feature:

- `mdm import ...` does parsing, preview, normalization, and file creation
- `mdmind` opens the imported result and helps refine it

The TUI should not become a format-ingestion surface.

## Scope

Import and ingestion cover two different jobs:

- faithful import from real map and outline formats
- best-effort ingestion from non-map content like web pages and PDFs

Those should be documented and implemented differently.

## Product Model

Treat this as one umbrella feature with three source buckets.

### 1. Real Map And Outline Import

These are the highest-value formats because they already carry tree structure.

Recommended priority:

- OPML
- FreeMind `.mm`
- Markdown outline / headings
- XMind `.xmind`
- later: MindManager `.mmap`

### 2. Structured Document Import

These are not always mind maps, but they still have usable hierarchy.

Recommended priority:

- Markdown headings
- Markdown bullet outlines
- plain text outlines

### 3. Best-Effort Content Ingestion

These sources are valuable, but lossy.

Recommended priority:

- web pages / HTML article extraction
- PDF text and outline extraction

These should be described as `ingestion`, not as faithful `import`.

## CLI Shape

Recommended command shape:

```bash
mdm import notes.opml --from opml -o imported.md
mdm import notes.mm --from freemind -o imported.md
mdm import notes.xmind --from xmind -o imported.md
mdm import notes.md --from markdown -o imported.md
mdm import article.html --from html -o imported.md
mdm import https://example.com/article --from web -o imported.md
mdm import report.pdf --from pdf -o imported.md
```

Useful follow-on flags:

- `--preview` (prints the generated native map to stdout without writing)
- `--report` (prints import summary stats to stderr)
- `--root "Imported Notes"`
- `--merge-into existing.md`
- `--details-from-body`
- `--keep-source-meta`

## Mapping Rules

Normalize imported content into the native map language consistently:

- source topic / heading -> node label
- source note / body text -> `| detail` lines
- source markers / labels -> `#tags`
- source properties / attributes -> `@key:value`
- source stable ids -> `[id:...]` when trustworthy
- source links / topic references -> `[[...]]` or `[[rel:...]]` when resolvable

When something cannot be represented cleanly:

- prefer preserving it in detail text or metadata
- do not silently drop it if it is important to interpretation

## Implementation Order

Phase 1:

- design `mdm import` (baseline command now exists)
- implement OPML import (baseline tree import now exists)
- implement Markdown outline import (baseline heading and bullet import now exists)

Phase 2:

- implement FreeMind `.mm` (baseline node import now exists)
- add preview and import report output

Phase 3:

- implement XMind `.xmind`
- decide whether MindManager is worth first-class support

Phase 4:

- add web ingestion (local HTML and rough remote web baselines now exist)
- add PDF ingestion

## Risks

- overpromising “import” when the real behavior is lossy extraction
- format-specific quirks leaking into the native model
- source metadata creating noisy imported maps
- ambiguous source links producing broken relations

## Product Test

A user should be able to take an existing outline or mind map from another tool and get a clean native `.md` map without manual restructuring.

Separately, a user should be able to pull a web page or PDF into a usable outline while understanding that the result is best-effort ingestion rather than faithful conversion.

## Current Baseline

`mdm import` currently supports OPML, Markdown, and FreeMind as faithful import paths. It also supports local HTML files as a lossy web-page ingestion path:

```bash
mdm import notes.opml
mdm import outline.md
mdm import map.mm
mdm import article.html
mdm import outline.md --preview --report
```

The format is inferred from `.opml`, `.mm`, `.html`, `.htm`, `.md`, `.markdown`, `.mdown`, `.mkd`, `.xmind`, `.mmap`, and `.pdf` extensions. Remote `http://` and `https://` sources are recognized as web ingestion. Use `--from opml`, `--from freemind`, `--from html`, or `--from markdown` when the extension is ambiguous. When `-o/--output` is omitted for a writing import, `mdm` writes beside the source as `<source-stem>-mind.md`.

The OPML importer preserves hierarchy, `text`/`title` labels, mdmind OPML fields such as `mdm_id`, `mdm_tags`, `mdm_task`, `mdm_detail`, and `mdm_refs`, safe extra attributes as metadata, and URL fields as external references.

The Markdown importer preserves heading hierarchy, bullet nesting, paragraph text as details, and native mdmind inline tokens in headings or bullets, including task markers, tags, metadata, ids, links, and relations.

The FreeMind importer preserves `<node TEXT>` hierarchy, safe `ID` values as mdmind ids, `LINK` values as references, NOTE richcontent as detail lines, child `<attribute NAME VALUE>` entries as metadata, and basic icon/source attributes as metadata or detail text.

The HTML importer preserves heading hierarchy, list item nesting, paragraph text as details, and basic HTML entity decoding. It is intentionally lossy.

Remote web sources are fetched and passed through the same rough HTML structural extractor:

```bash
mdm import https://example.com --preview --report
```

The CLI prints a warning because agents will usually produce better mdmind maps from messy pages by reading the source, choosing the useful structure, authoring the map directly, and validating it.

XMind `.xmind`, MindManager `.mmap`, and PDF sources are recognized as planned paths and return guided errors instead of generic file-read or unsupported-format failures. XMind needs archive-reader support, MindManager needs real samples and a first-class support decision, and PDF ingestion should be agent-authored into mdmind maps before validation.

`--preview` renders the generated native map to stdout without writing a file. `--report` renders an import summary to stderr with source, format, destination, structural counts, detail counts, preservation counts, duplicate id count, reference type counts, task state counts, validation error/warning counts, and compact tag/metadata breakdowns.

The test fixture pack includes generated corner cases plus realistic exporter shapes for RSS OPML, desktop outliner OPML, FreeMind/Freeplane `.mm`, Markdown headings/bullets, and browser-saved HTML.

Full archive readers and lossy document ingestion automation still belong to later slices.
