# Customizing mdmind Skills

`mdmind` ships base skills in [../skills](../skills). Use those first:

- `mdmind-map-authoring` for creating or reshaping native mdmind maps
- `mdm-cli-inspection` for validating, querying, deep-linking, or exporting maps

This guide is for tailoring those skills to your work: preferred tags, metadata
keys, id patterns, branch shapes, validation habits, and repeatable agent
behavior. Customize the skill copy your agent actually loads. That copy can be
global, user-level, or project-local.

## Pick The Right Layer

Use the smallest layer that solves the problem:

| Need | Best fit |
|---|---|
| Agents should always know basic mdmind syntax | add [AGENTS_SNIPPET.md](AGENTS_SNIPPET.md) to a project `AGENTS.md` |
| You have domain-specific tags, metadata, ids, or map shapes | edit your installed skill copy and add `## Project Overrides` |
| A workflow is repeated across several projects | put the same `## Project Overrides` section in your user-level or global skill copy |
| The base mdmind behavior should change for everyone | update the shipped skills in [../skills](../skills) |
| The skill should be distributed as a product artifact | package the customized skill as an agent plugin or reusable skill pack |

Skills are best for action-specific workflows. Passive `AGENTS.md` context is
better for tiny rules the agent should always remember.

## Recommended Customization Path

For most teams, edit the copy of the base skill that your agent loads. Add a
short `## Project Overrides` section near the top of `SKILL.md`, after the
defaults and before the long workflow detail.

Example:

```md
## Project Overrides

Apply these conventions for oncology trial operations maps:

- Prefer tags: `#protocol`, `#site`, `#cohort`, `#safety`, `#regulatory`, `#deviation`, `#decision`.
- Prefer metadata keys: `@trial`, `@phase`, `@site`, `@pi`, `@region`, `@status`, `@risk`, `@source`.
- Allowed `@status` values: `screening`, `enrolling`, `paused`, `closed`, `submitted`, `approved`.
- Allowed `@risk` values: `low`, `medium`, `high`, `critical`.
- Use ids on protocol sections, amendment decisions, site risks, cohort definitions, and regulator-facing deliverables.
- Use ids like `trial/<code>/protocol/amendment-03`, `trial/<code>/site/<site-id>`, or `trial/<code>/cohort/<name>`.
- Use `[[rel:blocks->...]]` for enrollment blockers and `[[rel:requires-approval->...]]` for regulatory dependencies.
- After editing, run `mdm validate <file>` and `mdm kv <file> --keys trial,phase,site,pi,region,status,risk,source`.
```

This keeps the trigger behavior of the base skill and adds only the local
vocabulary the agent should apply.

Avoid having two active copies with the same skill `name`, such as one global
`mdmind-map-authoring` and one project-local `mdmind-map-authoring`, unless you
have verified your agent's precedence behavior. If you customize a project copy,
do not also leave a competing global copy active in the same agent environment.

## What To Customize

Most projects only need a few local conventions.

### Tags

Use tags for lightweight categories or workflow state:

- `#protocol`, `#cohort`, `#endpoint`, `#site`
- `#safety`, `#adverse-event`, `#deviation`, `#regulatory`
- `#decision`, `#risk`, `#question`, `#source`

Keep tag vocabularies small. If every node has three tags, the tree usually
needs better labels or branch structure.

### Metadata Keys

Use `@key:value` metadata when the same field appears across many branches.

Good project-specific keys:

- `@trial:mx-104`
- `@phase:2b`
- `@site:ucsf-03`
- `@pi:chen`
- `@region:us-west`
- `@status:enrolling`
- `@risk:high`
- `@source:protocol-amendment-03`

Prefer a short allowed-value list for fields like `status`, `risk`, `phase`, and
`region`. Avoid one-off keys that only appear once.

### Id Patterns

Use ids for durable anchors humans or tools will revisit:

- `trial/mx-104/protocol/amendment-03`
- `trial/mx-104/site/ucsf-03`
- `trial/mx-104/cohort/dose-expansion`
- `trial/mx-104/safety/dlt-review`

Do not put ids on every node. Put them on durable branches, link targets,
export anchors, and important work items.

### Details

Use `| detail` lines for slower-reading context:

- rationale
- quotes
- meeting notes
- source excerpts
- decision history

Keep labels scannable and move the prose into details.

### Relations

Use relations only when lateral meaning matters outside the tree:

- `[[rel:blocks->trial/mx-104/cohort/dose-expansion]]`
- `[[rel:requires-approval->trial/mx-104/regulatory/irb-amendment-03]]`
- `[[rel:derived-from->trial/mx-104/source/protocol-amendment-03]]`

Prefer hierarchy first. Relations should preserve important cross-links, not
replace basic organization.

## Where To Put Overrides

Put the override section in the skill copy whose scope matches your need:

- global/admin copy when every user should share the same domain conventions
- user-level copy when the conventions are personal or span many projects
- project-local copy when the conventions belong to one repository or team

The content model is the same in every scope: add `## Project Overrides` to the
existing base skill instead of creating a separate project skill.

## Customization Checklist

Before adding custom instructions, decide:

- What task should trigger it?
- What tasks should not trigger it?
- Which tags are allowed?
- Which metadata keys are allowed?
- Which metadata values should stay consistent?
- Which branches deserve ids?
- Which relations are meaningful enough to preserve?
- Which `mdm` commands prove the result is valid?

If the answers are still vague, start with a short `AGENTS.md` note. If the
answers are specific to one base workflow, add a `## Project Overrides` section
to the loaded copy of that base skill.

## Validation Loop

For custom skill changes, test with real prompts:

1. Run the same prompt with only the base mdmind skills.
2. Run it again with the project customization available.
3. Compare structure, metadata consistency, id quality, and validation behavior.
4. Keep the customization only if it improves real output.

Useful commands:

```bash
mdm validate path/to/map.md
mdm kv path/to/map.md --keys trial,phase,site,pi,region,status,risk,source
mdm tags path/to/map.md
mdm links path/to/map.md
mdm relations path/to/map.md --plain
```

## Related Docs

- [AGENTS_SNIPPET.md](AGENTS_SNIPPET.md)
- [AGENT_USAGE.md](AGENT_USAGE.md)
- [QUERY_LANGUAGE.md](QUERY_LANGUAGE.md)
- [IDS_AND_DEEP_LINKS.md](IDS_AND_DEEP_LINKS.md)
- [NODE_DETAILS.md](NODE_DETAILS.md)
- [../skills/README.md](../skills/README.md)
