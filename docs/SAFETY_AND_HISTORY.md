# Safety And History

`mdmind` should feel safe to work quickly in.

That means recovery is not an edge-case feature. It is part of the normal editing loop.

## Undo And Redo

Use:

- `u` to undo
- `U` to redo

This is not only about text.

Undo and redo restore:

- structural edits
- focus
- view state
- filter state
- other nearby workspace context

That makes recovery feel like returning to a known working state, not just rolling back one primitive operation.

## Checkpoints

Checkpoints are for bigger moments.

Use them when:

- you are about to restructure a large branch
- you want a named restore point
- you expect to compare two possible versions of a branch

There are two kinds:

- manual checkpoints you create on purpose
- automatic safety snapshots created before larger structural changes

## Recent History In The Palette

The palette is also part of the safety system.

Type:

- `undo`
- `redo`
- `checkpoint`
- `safety`

That lets you browse recovery targets without pressing `u` or `U` repeatedly.

This is especially useful in longer sessions where “go back one step” is not enough.

## Autosave And Manual Save

You can work either way:

- `s` saves now
- `S` toggles autosave

If autosave is on, restored history still writes back to disk so the file and the TUI stay aligned.

If manual save is on, remember to save intentionally after edits you want to keep.

## Reloading From Disk

Use:

- `r` to reload from disk

That is useful when you want to discard the current unsaved in-memory state and return to the last saved file version.

## A Good Safety Habit

For everyday work:

- use undo for recent mistakes
- use redo when you changed your mind again
- use checkpoints before bigger experiments
- use recent history in the palette when the restore target is a few steps away

The goal is not to edit cautiously. The goal is to edit confidently because recovery is nearby.

## Related Docs

- [TUI_WORKFLOWS.md](TUI_WORKFLOWS.md)
- [PALETTE_AND_HELP.md](PALETTE_AND_HELP.md)
- [USER_GUIDE.md](USER_GUIDE.md)
