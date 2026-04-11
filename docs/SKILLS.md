# SKILLS.md Format

This is a practical format spec for a `SKILLS.md` file that guides agents working with `mdmind`.

It is not meant to be a heavy standard. It is meant to be easy for both humans and agents to read.

The goal is simple:

- describe when a workflow should be used
- describe what good output looks like
- describe what the agent should validate before handing work back

## What A Skill File Is For

A good `SKILLS.md` file helps an agent produce better, more consistent map output.

Typical use cases:

- project planning maps
- research synthesis maps
- writing outlines
- decision trees
- meeting follow-up maps
- status-review maps

The skill file should be narrow enough to guide real work, not a giant policy dump.

## Core Structure

Recommended sections:

```md
# Skill Name

Short purpose sentence.

## Use For

- concrete situations where this skill should be used

## Do Not Use For

- situations where this skill is the wrong tool

## Output Shape

- what the resulting mdmind map should generally contain

## Map Conventions

- preferred tags
- preferred metadata keys
- when to add ids
- when to add details
- when to add relations

## Workflow

1. step-by-step process for the agent

## Validation

- checks to run before returning the result

## Example

Short example map snippet or prompt
```

That structure is enough for most skills.

## Section Guidance

### Use For

Be concrete. Good examples:

- turning meeting notes into a decision map
- turning source notes into a research outline
- turning a backlog dump into a prioritized work map

Avoid vague text like “use for planning.”

### Do Not Use For

This matters more than many people think.

It keeps the agent from forcing the wrong output shape onto the wrong task.

Examples:

- not for short prose summaries
- not for raw transcript cleanup
- not for visual diagram generation
- not for tasks that do not need durable structure

### Output Shape

Describe the branch types you expect.

Example:

- root branch for the project
- branches for goals, workstreams, risks, decisions, and open questions
- children under workstreams for concrete tasks

This gives the agent a structural target without making every map identical.

### Map Conventions

This is where `mdmind` specifics belong.

Recommended things to specify:

- preferred tags like `#todo`, `#risk`, `#decision`
- preferred metadata keys like `@owner`, `@status`, `@priority`
- whether ids should be added only to major branches
- whether details should hold rationale or quotes
- whether relations are expected or should stay rare

Good convention blocks keep the format consistent across many runs.

### Workflow

Describe the steps the agent should follow.

Example:

1. read the source notes
2. identify major branches
3. keep labels concise
4. add details only where prose matters
5. add ids to durable branches
6. validate the map

### Validation

Tell the agent how to check its own work.

Useful checks:

- map parses cleanly
- metadata keys are consistent
- ids are stable and not duplicated
- relations target real ids
- labels are readable
- the result passes `mdm validate`

## Recommended mdmind Conventions

These defaults are usually sensible:

- use readable labels first
- use a few stable metadata keys, not many one-off keys
- add ids only where humans are likely to revisit or link
- keep relations meaningful and sparse
- use detail lines for real prose, not everything

## Example SKILLS.md

```md
# Research Synthesis Map

Turn source notes into a structured mdmind research map.

## Use For

- summarizing article notes into themes and evidence
- preparing a literature review outline

## Do Not Use For

- short prose summaries
- raw quote extraction without structure

## Output Shape

- root branch for the topic
- branches for themes, evidence, open questions, and follow-up work

## Map Conventions

- use #theme, #evidence, and #todo where helpful
- use @source and @status when the source material supports them
- add ids to theme branches and major evidence clusters
- use detail lines for quoted material or rationale

## Workflow

1. identify repeated themes
2. group evidence under the right theme branches
3. keep node labels short
4. put longer notes into details
5. validate before returning

## Validation

- run mdm validate on the output
- avoid duplicate ids
- keep metadata keys consistent

## Example

- Climate Adaptation Review [id:research/climate]
  - Heat resilience #theme [id:research/climate/heat]
    | Several sources agree that heat planning fails most often at neighborhood granularity.
```

## What Not To Do

Avoid these patterns:

- giant skill files that try to cover every domain
- vague instructions with no structural target
- too many mandatory metadata keys
- requiring ids on every node
- turning every branch into a relation-heavy graph

## Related Docs

- [AGENT_USAGE.md](AGENT_USAGE.md)
- [QUERY_LANGUAGE.md](QUERY_LANGUAGE.md)
- [IDS_AND_DEEP_LINKS.md](IDS_AND_DEEP_LINKS.md)
- [NODE_DETAILS.md](NODE_DETAILS.md)
