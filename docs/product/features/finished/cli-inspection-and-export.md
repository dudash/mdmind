# CLI Inspection And Export

`mdm` is not just a helper binary. It is a real product surface.

What it already does well:

- open or view maps from the terminal
- inspect tags, metadata keys, ids, and relations
- validate map structure
- export current maps or deep-linked subtrees
- import existing outlines and rough web structure into native maps
- seed new work from starter templates

Core commands:

- `mdm view`
- `mdm find`
- `mdm tags`
- `mdm kv`
- `mdm links`
- `mdm relations`
- `mdm validate`
- `mdm export`
- `mdm import`
- `mdm init`
- `mdm open`

What export already ships:

- JSON
- Mermaid
- OPML
- subtree export through deep links
- filtered export through `--query`

Why it matters:

- it makes maps scriptable
- it gives a low-friction read-only surface for exploration
- it is useful even for people who never open the TUI

The remaining future import work is archive/PDF expansion, not the baseline CLI import path.

Related docs:

- [README.md](../../../../README.md)
- [QUERY_LANGUAGE.md](../../../QUERY_LANGUAGE.md)
- [TEMPLATES.md](../../../TEMPLATES.md)
- [EXPORT_TARGETS.md](../../../EXPORT_TARGETS.md)
- [IMPORT_AND_INGESTION.md](../../../IMPORT_AND_INGESTION.md)
