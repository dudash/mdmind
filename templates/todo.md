- Project TODO Map #todo-map @status:active [id:todo]
  - Operating Rules #guide [id:todo/rules]
    - Keep this file in git
    - Use this map for local decomposition and agent handoff
    - Use Linear or GitHub issues for team coordination
    - Validate before and after agent edits
  - Current Focus #todo @status:active [id:todo/focus]
    - [ ] Define next slice #todo @status:active @owner:jason @priority:high [id:todo/focus/next-slice]
      | What should be true when this task is done?
      - [ ] Clarify acceptance criteria #todo @status:todo [id:todo/focus/next-slice/criteria]
      - [ ] Implement smallest working path #todo @status:todo [id:todo/focus/next-slice/implementation]
      - [ ] Run validation #todo @status:todo [id:todo/focus/next-slice/validation]
  - Backlog #todo @status:todo [id:todo/backlog]
    - [ ] Capture candidate work #todo @status:todo @priority:medium
  - Blocked #todo @status:blocked [id:todo/blocked] [[rel:needs->todo/handoff]]
    - [ ] Note the dependency and the unblock condition #todo @status:blocked
  - Decisions #decision [id:todo/decisions]
    - Keep this map local-first @status:active [id:todo/decisions/local-first]
  - Handoff Notes #guide [id:todo/handoff]
    - Active task query
      | mdm find TODO.md "task:open" --plain
      | mdm find TODO.md "#todo @status:active" --plain
    - Blocked task query
      | mdm find TODO.md "task:blocked" --plain
    - Metadata scan
      | mdm kv TODO.md --keys status,owner,priority --plain
    - Focus branch
      | mdm view TODO.md#todo/focus
  - Done Log #done @status:done [id:todo/done]
    - [x] Create the initial TODO map #done @status:done
