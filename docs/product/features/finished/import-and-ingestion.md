# Import And Ingestion

`mdm import` gives mdmind a CLI-first path for getting existing outlines and rough web structure into native `.md` maps.

What ships now:

- OPML import with hierarchy, labels, mdmind OPML fields, metadata, references, details, tags, tasks, and safe ids
- FreeMind `.mm` import with node hierarchy, links, note richcontent, attributes, icons, and safe ids
- Markdown outline import from headings, bullets, details, and native mdmind inline syntax
- local HTML import as intentionally lossy structural extraction from headings, lists, and paragraphs
- remote `http://` and `https://` web import through the same rough HTML extractor, with an agent guidance warning
- output format inference from URLs and common extensions
- default output naming as `<source-stem>-mind.md`
- `--preview` for stdout review without writing
- `--report` for structural and preservation stats
- guided errors for recognized-but-unimplemented XMind, MindManager, and PDF paths

Core examples:

```bash
mdm import notes.opml
mdm import map.mm
mdm import outline.md
mdm import article.html --preview --report
mdm import https://example.com --preview --report
```

Design boundary:

- OPML, FreeMind, and Markdown are faithful outline import paths.
- HTML and web are best-effort ingestion paths, useful for rough structure but not a substitute for agent-authored maps when meaning matters.
- `mdmind` stays the refinement surface after import lands as native Markdown.

Still future:

- first-class `.xmind` archive reader support
- a decision and samples for `.mmap`
- PDF ingestion beyond agent-authored map creation
- merge-into-existing-map workflows

Related docs:

- [IMPORT_AND_INGESTION.md](../../../IMPORT_AND_INGESTION.md)
- [cli-inspection-and-export.md](cli-inspection-and-export.md)
