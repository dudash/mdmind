# Safety And History

The app now has a real safety layer instead of relying only on manual save discipline.

Shipped pieces:

- undo / redo
- automatic safety checkpoints before major structural edits
- named checkpoints
- recent history browsing in the palette
- restore previews that explain what will change
- autosave-aware restore behavior

Why this matters:

- users can restructure large maps with less hesitation
- editing feels reversible
- destructive changes feel inspectable instead of scary

This is one of the most important “trust” features in the product.

Related docs:

- [SAFETY_AND_HISTORY.md](../../../SAFETY_AND_HISTORY.md)
