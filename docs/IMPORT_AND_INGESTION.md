# Import And Ingestion

## Goal

Let `mdmind` pull useful structure in from common map tools, outline formats, and best-effort external content.

This should be a CLI-first feature:

- `mdm import ...` does parsing, preview, normalization, and file creation
- `mdmind` opens the imported result and helps refine it

The TUI should not become a format-ingestion surface.

## Why This Matters

Right now the product is strong at:

- native editing
- native navigation
- export

The bigger missing capability is getting existing work into the format without manual copy-paste.

That includes two different jobs:

1. faithful import from real map and outline formats
2. best-effort ingestion from non-map content like web pages and PDFs

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
mdm import https://example.com/article --from web -o imported.md
mdm import report.pdf --from pdf -o imported.md
```

Useful follow-on flags:

- `--preview`
- `--report`
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

## Delivery Recommendation

Phase 1:

- design `mdm import`
- implement OPML import
- implement Markdown outline import

Phase 2:

- implement FreeMind `.mm`
- add preview and import report output

Phase 3:

- implement XMind `.xmind`
- decide whether MindManager is worth first-class support

Phase 4:

- add web ingestion
- add PDF ingestion

## Risks

- overpromising “import” when the real behavior is lossy extraction
- format-specific quirks leaking into the native model
- source metadata creating noisy imported maps
- ambiguous source links producing broken relations

## Product Test

A user should be able to take an existing outline or mind map from another tool and get a clean native `.md` map without manual restructuring.

Separately, a user should be able to pull a web page or PDF into a usable outline while understanding that the result is best-effort ingestion rather than faithful conversion.
