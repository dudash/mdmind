# Import And Ingestion

This is the most important missing “get existing work into mdmind” feature.

Why it belongs in the future shelf:

- export already ships
- import does not
- the design is clear enough to document, but not implemented yet

The plan is to make `mdm` own it:

- `mdm import` for parsing, preview, and normalization
- `mdmind` for refining the imported result after it lands as a native map

The feature should cover two different jobs:

- real import from map and outline formats like OPML, FreeMind, Markdown outline, and XMind
- best-effort ingestion from non-map content like web pages and PDFs

That distinction matters because web and PDF sources are useful, but inherently lossy.

Related docs:

- [IMPORT_AND_INGESTION.md](../../../IMPORT_AND_INGESTION.md)
- [EXPORT_TARGETS.md](../../../EXPORT_TARGETS.md)
