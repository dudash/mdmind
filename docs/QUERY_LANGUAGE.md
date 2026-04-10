# Search And Query Language

Search is the fastest way to make a large map feel small again.

In `mdmind`, search is not a separate database or a different syntax from the rest of the product. It works directly on the same map lines you already read and edit: visible text, `#tags`, and `@key:value` metadata.

## The Easiest Way To Start

If you are new, use search in this order:

1. plain text
2. tags
3. metadata
4. combinations

Examples:

```text
rate limit
#todo
@status:active
#todo @owner:jason
```

You do not need to learn every pattern up front. Even a simple text search is enough to make big maps usable.

## In The TUI

Use:

- `/` to open search on the query tab
- `b` to open browse for tags, metadata, and ids
- `w` to open search on saved views
- `Tab` to switch between Query, Browse, and Saved Views
- `Enter` to apply the current query or selection
- `n` / `N` to move between matches
- `c` to clear the active filter

The important behavior is this: when you apply a query, `mdmind` lands you on the first useful match instead of dropping you into a detached result list.

## Query Building Blocks

### Plain Text

Use plain text when you know the word or phrase you want:

```text
rate limit
release notes
audio pipeline
```

This is the best place to start if you are unsure how the map is structured.

### Tags

Use tags when you want to group by lightweight markers:

```text
#todo
#blocked
#backend
```

Tags are good for fast workflows and broad status buckets.

### Metadata

Use metadata when the map has consistent structured fields:

```text
@status:active
@owner:mira
@region:glass-harbor
```

Metadata becomes especially useful when a map has stable keys like `status`, `owner`, `priority`, `region`, or `surface`.

### Combined Queries

You can combine terms to narrow the working set:

```text
#todo @owner:jason
rate limit #backend
@status:blocked #release
```

This is the fastest way to turn a large map into a focused working surface.

## Facets

Facets are for discovery.

Use them when you do not know what tags or metadata already exist in the current map or filtered scope.

They are especially good for:

- learning a new example map
- browsing available `@owner` or `@status` values
- tightening a search after starting from broad text

## Saved Views

Saved views are for recurring filters.

Good saved views:

- “my active work”
- “blocked items”
- “story beats”
- “open launch tasks”

Bad saved views:

- one-off ad hoc searches you will never use again

## Examples

From the example maps:

```bash
mdm find examples/lantern-studio-map.md "@owner:mira" --plain
mdm find examples/game-world-moonwake.md "#quest @status:active" --plain
mdm find examples/novel-research-writing-map.md "#theme" --plain
```

These are useful because they show the same search language from the CLI side.

## Practical Search Advice

- start broad, then tighten
- prefer a few consistent tags and metadata keys over many one-off variants
- use browse when you do not know the map’s vocabulary yet
- save only recurring searches
- if you already know the exact branch or id, use the palette instead of search

## Related Docs

- [USER_GUIDE.md](USER_GUIDE.md)
- [IDS_AND_DEEP_LINKS.md](IDS_AND_DEEP_LINKS.md)
- [CROSS_LINKS_AND_BACKLINKS.md](CROSS_LINKS_AND_BACKLINKS.md)
