# Using mdmind With Agents

`mdmind` is a good format for agent output when the result needs to stay useful for a human.

That is the key distinction.

If you just need a one-off answer, plain Markdown is usually enough. If you want a structured plan, research synthesis, story outline, decision map, or project breakdown that a human will keep working in, a native `mdmind` map is often the better target.

## When mdmind Is Better Than Plain Markdown

Use an `mdmind` map when the output needs:

- real hierarchy
- durable branches you can revisit later
- lightweight workflow markers like `#todo`
- structured fields like `@owner:mira` or `@status:active`
- stable addresses with `[id:...]`
- cross-links between distant branches
- attached notes without turning the whole file into prose

If those things do not matter, a normal Markdown note is usually simpler.

## Good Agent Use Cases

Agents are especially useful for:

- turning messy notes into a cleaner outline
- turning a meeting transcript into action branches and open questions
- turning research into themes, evidence, quotes, and follow-up tasks
- turning product requirements into roadmap, risks, and dependencies
- turning writing notes into cast, places, scenes, and thematic branches

The better pattern is to ask the agent for a structured first draft, then use `mdmind` yourself to refine it.

## Recommended Workflow

1. Ask the agent for a native `mdmind` map, not a prose summary.
2. Validate the result with `mdm validate`.
3. Inspect it quickly with `mdm view`, `mdm find`, or `mdm links`.
4. Open it in `mdmind` and clean up the shape, labels, ids, and details.
5. Export to JSON later if another tool needs structured output.

Example:

```bash
mdm validate plan.md
mdm view plan.md
mdm find plan.md "#todo"
mdmind plan.md
```

## What To Ask The Agent For

The best prompt is usually explicit about the map language.

Good constraints:

- keep node labels short and readable
- use `#tags` only for meaningful grouping
- use a few stable metadata keys like `@owner`, `@status`, `@priority`
- add `[id:...]` only on durable branches
- use `| detail` lines only when a branch needs real prose
- use `[[target]]` or `[[rel:kind->target]]` only when the cross-link is worth preserving

Example prompt:

```text
Turn these notes into an mdmind map.
Keep labels short.
Use #todo for open work.
Use @owner and @status when the source supports them.
Add [id:...] only on major branches I am likely to revisit.
Use | detail lines for rationale or quoted material that should stay attached to one branch.
```

## When Not To Over-Structure

Agents tend to invent more structure than humans actually need.

That usually makes the result worse, not better.

Good rule:

- labels should still read well as plain text
- metadata should stay small and repeated
- ids should be reserved for branches worth linking to
- relations should be meaningful, not everywhere

If the map starts looking like a schema instead of a working outline, the agent probably overdid it.

## Details, Ids, And Relations

Three features matter most for agent handoff quality:

### Details

Use `| detail` lines for:

- rationale
- research excerpts
- scene notes
- quoted source material
- meeting context

Do not turn every node into a note block.

### Ids

Use ids on branches that humans or tools will probably revisit:

- main sections
- major work items
- stable topics
- reusable anchors

Ids are what make deep links and cross-links reliable later.

### Relations

Use relations when the tree alone is not enough.

Examples:

- a risk blocks a delivery branch
- a chapter points to a character and a place
- a research note supports a thesis branch

Start with plain `[[target]]`. Typed relations are useful, but they are the advanced case.

## Useful mdm Commands Around Agent Work

```bash
mdm init notes.md --template product
mdm validate notes.md
mdm view notes.md
mdm find notes.md "@status:blocked"
mdm links notes.md
mdm relations notes.md#product/mvp --plain
mdm export notes.md --format json
mdm examples list
mdm examples copy all
```

## Examples Help

The bundled examples are useful prompt references because they show the format being used for real work instead of toy snippets.

Use:

```bash
mdm examples copy all
```

Then inspect:

- `examples/agent-research-handoff.md`
- `examples/meeting-notes-action-map.md`
- `examples/lantern-studio-map.md`
- `examples/game-world-moonwake.md`
- `examples/novel-research-writing-map.md`

## Where SKILLS.md Fits

If you want repeatable agent behavior, pair maps with a `SKILLS.md` file that tells the agent:

- when to use the workflow
- when not to use it
- what structure to emit
- which metadata keys to prefer
- how to validate the result

See [SKILLS.md](SKILLS.md) for a practical format spec.

## Related Docs

- [USING_MDMIND_AS_OUTLINER.md](USING_MDMIND_AS_OUTLINER.md)
- [QUERY_LANGUAGE.md](QUERY_LANGUAGE.md)
- [IDS_AND_DEEP_LINKS.md](IDS_AND_DEEP_LINKS.md)
- [CROSS_LINKS_AND_BACKLINKS.md](CROSS_LINKS_AND_BACKLINKS.md)
- [NODE_DETAILS.md](NODE_DETAILS.md)
