# Project Templates

Templates are a real part of the product.

They are the starter maps used by:

```bash
mdm init roadmap.md --template product
```

The live template files are in the repo `templates/` directory. They are not just docs examples.

## Available Templates

- `product`: roadmap, scope, requirements, and open questions
- `feature`: one feature plan with flow, acceptance, rollout, and risks
- `prompts`: prompt library, variants, eval loop, and open issues
- `backlog`: a practical now/next/blocked/done working map
- `writing`: premise, characters, places, plot lines, themes, and chapters

## When To Use Them

Templates are useful when:

- you want a quick first map instead of starting from a blank file
- you are teaching someone the format
- you want a consistent scaffold for recurring work

They are intentionally small. The goal is to give you a useful starting shape, not to generate a giant opinionated document.

Each template demonstrates the core map language too:

- stable ids for deep links and exports
- tags for grouping
- `@key:value` metadata for filtering
- a small cross-link or typed relation where it adds real value

## Examples

```bash
mdm init roadmap.md --template product
mdm init auth-feature.md --template feature
mdm init prompt-library.md --template prompts
mdm init weekly-backlog.md --template backlog
mdm init novel.md --template writing
```

## Picking A Good Starter

- use `product` when you are shaping direction, scope, and launch questions
- use `feature` when one workflow or delivery slice is the center of the work
- use `prompts` when you are iterating on model behavior and review loops
- use `backlog` when you mostly need a calm execution board
- use `writing` when you are connecting cast, locations, themes, and chapter plans

## What Happens Next

After creating a map from a template, open it in `mdmind` and shape it into something real:

- add or edit branches
- assign tags and metadata
- give important branches ids
- save useful filters
- add relations only where lateral structure matters

Templates are a starting point, not a special document type.
