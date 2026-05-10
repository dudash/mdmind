- Codex TODO Workflow #todo-map @status:active [id:codex-todo]
  - Read Me First #guide [id:codex-todo/start]
    - What this map is
      - a local-first TODO map for one developer working with Codex
      - a durable handoff surface across sessions
      - a complement to Linear or GitHub issues, not a replacement for team coordination
    - Working loop #guide [id:codex-todo/start/loop]
      - inspect active work
      - choose one task
      - edit code and map together
      - validate before handoff
      - summarize what changed
    - Useful commands #reference [id:codex-todo/start/commands]
      | mdm find examples/agent-todo-workflow.md "task:open" --plain
      | mdm find examples/agent-todo-workflow.md "#todo @status:active" --plain
      | mdm kv examples/agent-todo-workflow.md --keys status,owner,priority,area --plain
      | mdm view examples/agent-todo-workflow.md#codex-todo/current
      | mdm validate examples/agent-todo-workflow.md
  - Current Session #todo @status:active [id:codex-todo/current]
    - [x] Ship TODO template adoption slice #done @status:done @owner:jason @priority:high @area:templates [id:codex-todo/current/template-slice] [[rel:supports->codex-todo/decisions/local-first]]
      | Done means `mdm init work.md --template todo` works, docs mention it, and the example validates.
      - [x] Add bundled TODO template #done @status:done @owner:codex @area:templates [id:codex-todo/current/template-slice/template]
      - [x] Add agent workflow example #done @status:done @owner:codex @area:examples [id:codex-todo/current/template-slice/example]
      - [x] Update docs and skills #done @status:done @owner:codex @area:docs [id:codex-todo/current/template-slice/docs]
      - [x] Run parser and CLI validation #done @status:done @owner:codex @area:validation [id:codex-todo/current/template-slice/validation]
    - [x] Design first-class checkbox model #done @status:done @owner:jason @priority:high @area:tui [id:codex-todo/current/checklist-model] [[rel:informed-by->codex-todo/risks/conflicting-state]]
      | This is the deeper engine slice: parser state, toggle behavior, rollups, and conflict warnings.
      - [x] Decide canonical write strategy #done @status:done
      - [x] Add task state parser tests #done @status:done
      - [x] Add TUI toggle command #done @status:done
  - Blocked #todo @status:blocked [id:codex-todo/blocked]
    - [ ] Parent rollup visualization #todo @status:blocked @owner:jason @area:tui [id:codex-todo/blocked/rollups] [[rel:blocked-by->codex-todo/current/checklist-model]]
      | Blocked until the task-state read model is explicit enough to avoid misleading parent summaries.
  - Decisions #decision [id:codex-todo/decisions]
    - Keep TODO maps local-first #decision @status:active [id:codex-todo/decisions/local-first]
      | mdmind should support local decomposition and agent memory without replacing issue trackers.
    - Prefer sparse metadata #decision @status:active [id:codex-todo/decisions/sparse-metadata]
      - use `@owner`, `@status`, `@priority`, and `@area` when they help filtering
      - do not require every task to carry every field
  - Risks #risk [id:codex-todo/risks]
    - Conflicting task state #risk @status:active [id:codex-todo/risks/conflicting-state]
      | Future checkbox support should warn when one line says both done and active.
      | Example conflict: [x] Update docs #todo @status:active
    - Over-structured maps #risk @status:active [id:codex-todo/risks/over-structure]
      - too many ids make small task lists harder to scan
      - too much metadata turns the map into a spreadsheet
  - Handoff Notes #guide [id:codex-todo/handoff]
    - For Codex
      - validate the map before editing
      - update only the task branch you are working inside unless asked otherwise
      - add details when acceptance criteria or blockers matter
      - summarize open, blocked, and done work at handoff
    - Active inspection query
      | mdm find examples/agent-todo-workflow.md "task:open" --plain
      | mdm find examples/agent-todo-workflow.md "#todo @status:active" --plain
    - Blocked inspection query
      | mdm find examples/agent-todo-workflow.md "task:blocked" --plain
      | mdm find examples/agent-todo-workflow.md "@status:blocked" --plain
  - Done Log #done @status:done [id:codex-todo/done]
    - [x] Identify Linear TODO issue cluster #done @status:done @owner:codex @area:planning [id:codex-todo/done/linear-cluster]
      | Relevant issues: MDM-5, MDM-27, MDM-13, and MDM-31.
