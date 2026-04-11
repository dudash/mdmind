# Example Maps

These example maps are meant to be opened, searched, filtered, and deep-linked, not just skimmed.

Each one demonstrates a different style of thinking in `mdmind`:

- product and roadmap planning
- meeting notes and action follow-up
- prompt iteration and evaluation
- decision logging
- agent-to-human research handoff
- studio operating maps
- game worldbuilding and quest architecture
- novel research, themes, and drafting

You can explore them in two ways:

```bash
mdmind examples/lantern-studio-map.md
mdm view examples/lantern-studio-map.md#lantern/execution/now
```

## Quick Picks

- Want the smallest starter example: [demo.md](./demo.md)
- Want realistic product planning: [product-status.md](./product-status.md)
- Want meeting notes that still feel like an outliner: [meeting-notes-action-map.md](./meeting-notes-action-map.md)
- Want an agent-generated handoff shape: [agent-research-handoff.md](./agent-research-handoff.md)
- Want a lived-in studio operating map: [lantern-studio-map.md](./lantern-studio-map.md)
- Want worldbuilding and quests: [game-world-moonwake.md](./game-world-moonwake.md)
- Want writing and research: [novel-research-writing-map.md](./novel-research-writing-map.md)

## Maps

### `demo.md`

The smallest map in the repo. Good for learning the file shape and basic tree navigation.

Try:

```bash
mdmind examples/demo.md
mdm view examples/demo.md
mdm links examples/demo.md --plain
```

### `product-status.md`

A compact product planning map with features, active work, priorities, and docs-related branches.

What it demonstrates:

- `#feature`, `#todo`, and status-style metadata
- product breakdown by capability area
- ids on important feature branches

Good first jump:

```bash
mdm view examples/product-status.md#example/product/features/docs
```

Good CLI probes:

```bash
mdm find examples/product-status.md "#todo @status:active" --plain
mdm kv examples/product-status.md --keys status,owner,priority --plain
mdm links examples/product-status.md --plain
```

### `meeting-notes-action-map.md`

A classic meeting-notes outline with agenda, discussion notes, decisions, action items, and parking-lot topics.

What it demonstrates:

- an OmniOutliner-style use case that still benefits from tags, metadata, ids, and cross-links
- longer discussion notes attached with detail lines instead of bloating row labels
- turning one meeting into a reusable action surface instead of a dead transcript

Good first jump:

```bash
mdm view examples/meeting-notes-action-map.md#meeting/actions
```

Good CLI probes:

```bash
mdm find examples/meeting-notes-action-map.md "@owner:maya" --plain
mdm find examples/meeting-notes-action-map.md "#decision" --plain
mdm kv examples/meeting-notes-action-map.md --keys owner,status,priority,date --plain
```

### `prompt-ops.md`

A prompt-iteration workspace for variants, evaluation notes, and prompt ownership.

What it demonstrates:

- prompt families with sibling variants
- metadata for evaluation and ownership
- a structure that works for prompt libraries without turning into a spreadsheet

Good first jump:

```bash
mdm view examples/prompt-ops.md#example/prompts/variants
```

Good CLI probes:

```bash
mdm find examples/prompt-ops.md "@owner:jason" --plain
mdm tags examples/prompt-ops.md --plain
mdm kv examples/prompt-ops.md --keys owner,status,score,failure --plain
```

### `decision-log.md`

A small decision record map for architecture or product tradeoffs.

What it demonstrates:

- dated or staged decisions
- pros/cons and consequences as branches
- stable ids for citing decisions later

Good CLI probes:

```bash
mdm tags examples/decision-log.md --plain
mdm links examples/decision-log.md --plain
mdm find examples/decision-log.md "tradeoff" --plain
```

### `agent-research-handoff.md`

A research synthesis map shaped the way an agent might hand off work to a human operator.

What it demonstrates:

- agent-friendly map structure without over-structuring every branch
- themes, evidence, open questions, and actions in one file
- ids, details, and a few meaningful relations
- a practical bridge from agent output to human cleanup in `mdmind`

Good first jump:

```bash
mdm view examples/agent-research-handoff.md#agent-handoff/themes
```

Good CLI probes:

```bash
mdm find examples/agent-research-handoff.md "@source:interview" --plain
mdm find examples/agent-research-handoff.md "#todo @status:active" --plain
mdm relations examples/agent-research-handoff.md#agent-handoff/themes/ownership --plain
```

### `lantern-studio-map.md`

A large, lived-in studio operating map built around the fictional immersive nighttime experience `Drift Signal`. It mixes creative direction, route design, launch prep, audience signals, story/copy work, team roles, and risks.

What it demonstrates:

- a map that feels like two months of real work
- multiple owners across one live production surface
- suggested filters and deep links embedded in the document
- how ids, tags, metadata, and typed cross-links can support a dense operating map
- typed cross-links and backlinks across roadmap, docs, launch, and team branches

Good first jumps:

```bash
mdm view examples/lantern-studio-map.md#lantern/execution/now
mdm view examples/lantern-studio-map.md#lantern/timeline/week-8
```

Good CLI probes:

```bash
mdm find examples/lantern-studio-map.md "@owner:mira" --plain
mdm find examples/lantern-studio-map.md "#blocked" --plain
mdm kv examples/lantern-studio-map.md --keys owner,area,priority,dependency --plain
mdm links examples/lantern-studio-map.md --plain
mdm relations examples/lantern-studio-map.md#lantern/product/docs --plain
```

### `game-world-moonwake.md`

A narrative exploration game map with regions, factions, characters, quests, systems, production work, and playtest feedback.

What it demonstrates:

- worldbuilding that stays operational
- region metadata like `@region:*`
- quest arcs and production tasks in one structure
- filtering creative work by owner, region, or quest state

Good first jumps:

```bash
mdm view examples/game-world-moonwake.md#moonwake/world
mdm view examples/game-world-moonwake.md#moonwake/quests/main
mdm view examples/game-world-moonwake.md#moonwake/production/current
```

Good CLI probes:

```bash
mdm find examples/game-world-moonwake.md "#quest @status:active" --plain
mdm kv examples/game-world-moonwake.md --keys owner,region,status --plain
mdm find examples/game-world-moonwake.md "@region:glass-marsh" --plain
```

### `novel-research-writing-map.md`

A novel-development workspace with characters, places, plot lines, secondary themes, image systems, quotes, chapter planning, research notes, and revision work.

What it demonstrates:

- writing projects as structured maps instead of loose notes
- `#theme`, `#quote`, `#chapter`, and location-based metadata
- cross-links between characters, locations, themes, plot lines, and current chapters
- attached detail lines for character and location notes that are longer than a one-line label
- how research, drafting, and revision can live in one file

Good first jumps:

```bash
mdm view examples/novel-research-writing-map.md#glass-archive/themes
mdm view examples/novel-research-writing-map.md#glass-archive/chapters
```

Good CLI probes:

```bash
mdm tags examples/novel-research-writing-map.md --plain
mdm find examples/novel-research-writing-map.md "#quote" --plain
mdm find examples/novel-research-writing-map.md "@location:archive" --plain
mdm kv examples/novel-research-writing-map.md --keys pov,location,owner,status --plain
mdm relations examples/novel-research-writing-map.md#glass-archive/characters/mara --plain
```

## Suggested Workflow

If you are trying to learn the product quickly:

1. Open one example in `mdmind`.
2. Use `:` or `Ctrl+P` to jump by branch name, id, `#tag`, or `@metadata`.
3. Use `/` and `b` to compare freeform queries with browse-driven discovery.
4. Use `m` after narrowing the working set to see how the mindmap reflects the same scope.
5. Use the same file in `mdm` with `find`, `tags`, `kv`, and `links` to see the read-only CLI model.
