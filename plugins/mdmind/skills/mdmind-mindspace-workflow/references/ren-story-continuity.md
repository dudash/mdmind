# Ren: Story Continuity

Use `story-continuity` when the user wants editorial continuity help without
giving up authorship of prose drafts.

## Starting Prompt

```text
Use the story continuity template. Check this chapter against character, place,
timeline, and theme maps. Do not rewrite the draft. Create review items for
continuity risks and suggest map updates where the story bible is stale.
```

## Expected Workspace

```text
maps/book.md
maps/characters.md
maps/places.md
maps/timeline.md
pages/chapter-08-draft.md
pages/research-notes.md
inbox/
log.md
```

## Useful Branches

- `book/chapters`
- `characters/main`
- `places/active`
- `timeline/current`
- `themes/open`
- `continuity/risks`

## Workflow

1. Scan and identify maps versus prose pages.
2. Keep drafts as pages unless Ren explicitly asks to import or rewrite.
3. Gather only linked character, place, timeline, and theme branches.
4. Produce continuity review items with target, rationale, and suggested fix.
5. Propose story-bible map updates separately from draft changes.

## mdmind Review

The human should see the chapter or draft page, linked characters and places,
continuity warnings, proposed story-bible updates, and recent scenes.

## Success

Ren sees risks without the agent flattening the prose voice, and the story bible
becomes easier to maintain.
