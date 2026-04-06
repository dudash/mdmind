# Format Specification

This file defines the source-of-truth file format for v2.

## 1. Tree structure

Hierarchy is defined by indentation under markdown-style bullet items.

Example:

```md
- Root
  - Child A
    - Grandchild
  - Child B
```

Each line beginning with `- ` defines one node.

## 2. Node content

A node line consists of:
- visible text label
- optional inline tags
- optional inline key:value metadata
- optional inline id anchor

Example:

```md
- API Design #backend @status:todo @owner:jason [id:product/api-design]
```

## 3. Tags

Tags are freeform topic or workflow markers.

Syntax:

```text
#tag
```

Examples:
- `#idea`
- `#todo`
- `#prompt`
- `#blocked`

Rules:
- Multiple tags allowed per node
- Tags are case-insensitive in search, but original case may be preserved for display

## 4. Key:value metadata

Metadata adds structured, queryable attributes.

Syntax:

```text
@key:value
```

Examples:
- `@status:todo`
- `@owner:jason`
- `@priority:high`
- `@prompt_type:system`

Rules:
- Key names should be lowercase
- Initial version supports simple values without spaces
- Quoted values may be added later if needed

## 5. Node ids

Ids provide stable references and deep-link targets.

Syntax:

```text
[id:path/to/node]
```

Example:

```md
- Prompt Refinement [id:prompts/refinement]
```

Rules:
- Ids must be unique per file
- Ids should be path-like and stable
- Ids are not user-facing labels

## 6. Deep links

CLI and TUI should accept:

```text
file.md#path/to/node
```

Resolution order:
1. Match `[id:path/to/node]`
2. Future: optional fallback path resolution by node labels

## 7. Parser expectations

Parser should return a normalized node model with:
- `text`
- `tags[]`
- `kv{}`
- `id`
- `children[]`

## 8. Serializer expectations

Serialization should preserve:
- hierarchy
- label text
- tags
- kv metadata
- id

Preference:
- preserve source order when reasonable
- avoid unnecessary reformat churn

## 9. Example

```md
- Product Idea [id:product]
  - MVP Scope #todo @status:active
    - CLI commands
    - File format
  - Prompt Library #prompt @owner:jason [id:prompts/library]
```
