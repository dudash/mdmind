# Archive And Document Ingestion

Baseline `mdm import` is now current product behavior. This future shelf tracks the remaining deeper ingestion work that should not be implied as shipped.

What is still future:

- first-class `.xmind` archive reader support
- a support decision for MindManager `.mmap`, backed by real samples
- PDF ingestion beyond agent-authored map creation
- merge-into-existing-map workflows
- richer preservation controls such as source metadata, body-detail policies, and import roots

The distinction still matters:

- OPML, FreeMind, and Markdown outlines are faithful import paths.
- HTML and web sources are lossy structural extraction.
- PDFs and messy prose-heavy sources are often better handled by an agent that reads the source, authors a native mdmind map, and validates it.

Related docs:

- [finished/import-and-ingestion.md](../finished/import-and-ingestion.md)
- [IMPORT_AND_INGESTION.md](../../../IMPORT_AND_INGESTION.md)
