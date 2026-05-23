# Agent TODO And Memory Maps

`mdmind` works best as durable local memory for agent-assisted work when the map is the place where tasks, decisions, blockers, and handoff notes stay inspectable across sessions.

It should not replace the tiny always-loaded instructions an agent needs, and it should not replace team issue trackers. Treat each surface as having a different job:

| Surface | Best For | Keep It Small By |
| --- | --- | --- |
| `AGENTS.md` | Always-loaded operating rules | storing only syntax, commands, and project conventions |
| `TODO.md` mdmind map | Durable task memory and handoff context | using ids on durable branches, not every row |
| mdmind skills | Workflow-specific creation and inspection behavior | keeping reusable guidance outside one project file |
| Linear/GitHub issues | Team-visible coordination and commitments | linking back to local decomposition only when useful |

## Recommended Pattern

Use an mdmind TODO or memory map when an agent needs to resume work later, explain what changed, or hand context back to a human.

Good uses:

- local decomposition before work becomes team-tracked
- a current session branch with open, blocked, and done tasks
- decisions and risks that explain why tasks are shaped a certain way
- research memory where evidence, questions, and next actions need stable ids
- branch-level handoff notes with validation commands

Poor uses:

- one-off answers
- prose summaries that will not be edited again
- reminders, recurring tasks, calendar work, or hosted sync
- public team commitments that belong in Linear or GitHub

## Map Shapes

### Project TODO Map

Use this for implementation work. Start with `mdm init TODO.md --template todo`, then keep current work near the top:

```text
- Project TODO Map #todo-map @status:active [id:todo]
  - Current Focus #todo @status:active [id:todo/focus]
    - [ ] Ship next slice #todo @status:active @owner:jason @priority:high
      | Done means tests pass, docs are updated, and the handoff note is clear.
      - [ ] Investigate current behavior #todo @status:todo
      - [ ] Implement smallest useful path #todo @status:todo
      - [ ] Validate and summarize #todo @status:todo
  - Blocked #todo @status:blocked [id:todo/blocked]
  - Decisions #decision [id:todo/decisions]
  - Handoff Notes #guide [id:todo/handoff]
  - Done Log #done @status:done [id:todo/done]
```

Use `task:open`, `task:blocked`, and `task:done` for recurring inspection.

### Session Handoff Map

Use this when an agent is only responsible for one branch:

```text
- Session Handoff #handoff @status:active [id:handoff]
  - Goal #guide [id:handoff/goal]
  - Assigned Branch #todo @status:active [id:handoff/branch]
  - Constraints #guide [id:handoff/constraints]
  - Validation #guide [id:handoff/validation]
  - Summary #guide [id:handoff/summary]
```

Keep instructions short. Put long rationale in detail lines under the relevant branch.

### Research Or Decision Memory

Use this when the agent is synthesizing information, not shipping code:

```text
- Research Memory #research @status:active [id:research]
  - Questions #question [id:research/questions]
  - Evidence #reference [id:research/evidence]
  - Decisions #decision [id:research/decisions]
  - Follow-up Tasks #todo [id:research/tasks]
```

Relations are useful here when evidence supports a conclusion or a risk blocks an action.

## Safe Maintenance Loop

Agents should prefer inspection before rewriting:

```bash
mdm validate TODO.md
mdm find TODO.md "task:open" --plain
mdm find TODO.md "task:blocked" --plain
mdm kv TODO.md --keys status,owner,priority,area --plain
mdm view TODO.md#todo/focus
```

After edits, validate again and summarize:

- what changed in the map
- which task moved open, blocked, or done
- which branch the agent touched
- which validation commands passed

## Eval Candidates

Good future evals for agent TODO and memory support:

- create a TODO map from a messy project request and validate it
- inspect a TODO map, choose the highest-priority open task, and summarize the target branch
- update one assigned branch without rewriting sibling branches
- mark a task done and add a concise done-log note
- detect conflicting task state with `mdm validate` and propose a minimal repair
- maintain a research memory map by adding evidence, relations, and follow-up tasks

The success criterion is not maximum structure. It is whether the map stays readable, valid, local-first, and useful when the next session starts.
