# PRD: Contextual AI Assistance for `mdmind.io` TUI

Status: Draft
Product surface: `mdmind.io` TUI
Primary promise: AI help that is opt-in, contextual, reviewable, and never in the way.

## Summary

`mdmind.io` should support an optional AI assistance layer inside the TUI.

The goal is not to bolt a chatbot onto a terminal map editor. The goal is to make `mdmind` feel like a sharper, calmer thinking environment for users who explicitly choose AI help.

AI should understand the selected map context, infer the likely map type and user intent, then offer a small number of useful next moves. The user stays in control. AI can suggest, explain, reorganize, extract, summarize, or draft changes, but it should never silently rewrite the map.

The first release should focus on the feeling:

```text
I am working in a map.
I ask for help here.
mdmind understands the shape of this work.
It suggests useful next steps.
I review everything before it changes.
Nothing happens behind my back.
```

AI should be:

```text
Optional by default.
Quiet by design.
Contextual when invoked.
Reviewable before apply.
Safe to undo.
```

## Product Thesis

AI in `mdmind` should feel like a thoughtful collaborator standing nearby, not an invisible coauthor and not a loud assistant panel.

The user should not have to choose from a long list of fixed AI commands.

Instead:

```text
User invokes AI in context.
mdmind inspects the selected scope.
AI infers the map type, branch state, and likely user intent.
AI suggests a few useful next actions.
The user chooses one.
AI returns reviewable suggestions.
The user accepts, rejects, refines, or ignores.
```

The core product model:

```text
AI does not own the map.
AI helps the user think through the map.
```

## Problem

`mdmind` is a tool for structured thinking: outlines, product plans, research maps, meeting notes, task trees, brainstorms, decision maps, and agent handoff plans.

Users often need help at moments like:

* “This branch is too messy.”
* “What am I missing?”
* “Turn this into a clearer plan.”
* “Pull out the TODOs and open questions.”
* “Summarize this branch so I can hand it off.”
* “Does this product plan have holes?”
* “What should I think about next?”
* “Can this become a better map?”

Today, AI can help with these jobs, but only outside the flow. A user has to copy content into another tool, ask for help, then manually bring the result back. That breaks the calm, map-first experience.

At the same time, many users do not want networked AI access. Some want local-only models. Some want no AI at all. Others may want to bring their own OpenAI-compatible endpoint, local model server, or local agent CLI.

So the product needs a careful balance:

```text
Powerful enough to be useful.
Quiet enough to ignore.
Transparent enough to trust.
Flexible enough to support many runtimes.
Constrained enough to stay safe.
```

## What This Is Not

This is not:

* a generic chatbot;
* an always-open AI sidebar;
* an autonomous agent that edits files;
* a hidden network feature;
* a hosted AI service;
* a replacement for the human’s judgment;
* a reason to make the TUI noisy.

Bad AI integration would make `mdmind` feel worse.

The product must avoid:

* hardcoded AI command clutter;
* surprise API calls;
* hidden cost;
* unclear context sharing;
* overconfident rewrites;
* vague prose that cannot be applied to a map;
* a sense that AI is watching everything;
* a sense that the user is now managing an agent instead of thinking.

## Design North Star

The best version of this feature should feel like this:

```text
I press one key.
mdmind understands where I am.
It offers exactly the kind of help I was about to ask for.
I choose.
It shows me the result clearly.
I accept only what I want.
Then it disappears.
```

The AI should feel “there but never in the way.”

## Goals

### User Goals

Users should be able to:

* use `mdmind` fully without ever enabling AI;
* enable AI only when they explicitly choose;
* connect a cloud or local AI provider;
* understand whether AI is off, ready, working, unavailable, or waiting for review;
* invoke AI from the current node, branch, visible map, or whole map;
* see what context will be sent before any networked call;
* receive contextual suggestions based on the map shape;
* review proposed changes before they are applied;
* accept, reject, partially accept, or refine suggestions;
* undo accepted AI changes like normal edits;
* keep secrets out of map files, sidecars, logs, and shell history.

### Product Goals

This feature should:

* make `mdmind` feel more intelligent without making it feel busier;
* preserve the local-first, keyboard-first editing loop;
* make AI map-native rather than chat-native;
* support cloud and local providers through a clean adapter model;
* prioritize confidence, privacy, and delight over breadth;
* create a future path for agent handoffs, research workflows, and custom recipes.

## Non-Goals

The first release will not include:

* AI enabled by default;
* passive background suggestions without explicit user configuration;
* autonomous edits;
* arbitrary shell execution;
* whole-workspace agents;
* automatic web browsing;
* team AI accounts;
* hosted `mdmind` AI billing;
* training on user maps;
* telemetry without explicit opt-in;
* a persistent general-purpose chat application.

## Delight Principles

### 1. The quiet magic moment

The first delightful moment should not be flashy.

It should be:

```text
Select a messy branch.
Open AI Chat.
mdmind says: “I’m looking at this map context. Ask a question, find gaps, or request reviewable map edits.”
```

That is the magic: the system understands the shape of the work without taking over.

### 2. One obvious door

There should not be a long list of AI commands in the command palette.

The primary entry should be simple:

```text
AI: Open Chat
```

AI Chat is the obvious door. AI Settings is nearby for setup, providers, and
review controls, but those choices should not become separate global commands.

### 3. Smart suggestions, not command overload

After AI is invoked, `mdmind` should suggest a small number of contextual actions.

Usually 3–5 options is enough.

Example:

```text
This looks like a PRD branch.

Useful next steps:
› Find missing requirements
  Extract open questions
  Suggest acceptance criteria
  Split into implementation slices
  Summarize for handoff
```

This feels much better than forcing users to pick from a large static menu.

### 4. Visible boundaries

Before any networked call, the product should make the boundary visible:

```text
Provider: OpenAI-compatible endpoint
Scope: Current branch
Sending: 14 nodes, 3 detail blocks
```

The user should never wonder, “What did it send?”

### 5. Review is the interface

The review experience is where trust is earned.

AI output should appear as proposed map edits, not just prose.

Good:

```text
Add 4 child nodes under “Launch Readiness”
Edit 2 labels for clarity
Add 3 open questions
Create 1 summary note
```

Bad:

```text
Here is a rewritten version of your whole plan.
```

### 6. Disappear after helping

After the user accepts, rejects, or dismisses a suggestion, the AI surface should get out of the way.

No lingering assistant panel.
No “what next?” nagging.
No chatty follow-up unless the user asks.

### 7. Terminal-native delight

The experience should feel beautiful inside a terminal.

Delight should come from:

* crisp keyboard flow;
* subtle status indicators;
* fast preview;
* clean diffs;
* elegant text layout;
* excellent empty states;
* minimal ceremony.

Not from animations, avatars, or novelty.

## Core Interaction Model

### Primary Flow

```text
1. User selects a node or branch.
2. User invokes “AI: Open Chat.”
3. mdmind opens AI Chat, or AI Settings first when setup is needed.
4. AI Chat shows provider, model, token estimate, and the active map context.
5. User asks a freeform question or chooses a quick prompt.
6. Conversational answers stay in AI Chat.
7. Explicit edit requests return structured suggestions.
8. mdmind validates the output.
9. User reviews proposed changes.
10. User accepts, partially accepts, rejects, or refines.
11. mdmind applies accepted changes with undo/checkpoint support.
```

### Core UX Rule

AI should not begin with a command list.

AI should begin with the current context.

## Context Understanding

When invoked, `mdmind` should inspect the selected scope and classify it.

### Map Type Detection

Possible detected map types:

```text
PRD
Project plan
Meeting notes
Research map
Writing outline
Decision tree
Task breakdown
Brainstorm
Debugging notes
Learning notes
Agent handoff map
Personal planning map
Unknown / mixed
```

### Branch State Detection

Possible detected branch states:

```text
Sparse
Messy
Dense
Task-heavy
Decision-heavy
Risk-heavy
Question-heavy
Well-structured
Contradictory
Incomplete
Repetitive
Unclear
```

### User Intent Inference

Possible inferred intents:

```text
Create structure
Improve structure
Extract useful items
Review for gaps
Summarize
Prepare handoff
Ask a question
Continue thinking
Compare options
Clarify decisions
```

These classifications do not need to be exposed as formal labels unless useful. They exist to make suggestions better.

The user-facing output should be human:

```text
This looks like rough meeting notes.
```

Not robotic:

```text
Map type: MEETING_NOTES. Branch state: TASK_HEAVY.
```

## Dynamic Suggestions

After context detection, `mdmind` should show suggested actions.

### Example: PRD Branch

```text
This looks like product planning.

Useful next steps:
› Find missing requirements
  Suggest MVP scope
  Draft acceptance criteria
  Extract open questions
  Split into implementation phases
```

### Example: Messy Notes

```text
This looks like rough notes.

Useful next steps:
› Clean up into clearer child nodes
  Extract TODOs
  Extract decisions
  Rewrite labels for clarity
  Summarize the main idea
```

### Example: Meeting Notes

```text
This looks like meeting notes.

Useful next steps:
› Extract action items
  Extract decisions
  Create follow-up questions
  Summarize for sharing
```

### Example: Sparse Branch

```text
This branch looks underdeveloped.

Useful next steps:
› Suggest child nodes
  Ask clarifying questions
  Create a starter structure
  Identify missing sections
```

### Example: Research Map

```text
This looks like research or analysis.

Useful next steps:
› Summarize key claims
  Find weak evidence
  Identify contradictions
  Suggest research questions
```

### Example: Agent Handoff Map

```text
This looks like agent handoff context.

Useful next steps:
› Create a concise handoff brief
  Extract implementation tasks
  Identify missing constraints
  Draft acceptance criteria
```

## Fallback Behavior

If AI cannot confidently infer the context, it should not fake certainty.

Fallback:

```text
What would you like help with?

› Create structure
  Clean this up
  Find gaps
  Extract tasks/questions
  Summarize
  Ask a freeform question
```

This keeps the experience useful without pretending.

## AI Surfaces

### 1. Panel State And Status Row

AI state should live in AI Settings, AI Chat/Review headers, and the normal
status area when there is active work to explain. The main map header should not
show ordinary provider/chat/review lamps.

Panel/status examples:

```text
Provider Ollama Local
Provider NVIDIA NIM
Provider Codex Local
Provider Claude Local
NIM response · "Find gaps..."
Review Suggestions · 3 staged map edits
NIM request timed out
```

The status row should be subtle. It should not dominate the UI.

The active/default profile decides which provider AI actions use. Users switch
explicitly inside AI Settings:

```text
Use Ollama Local
Use NVIDIA NIM
Use Codex Local
Use Claude Local
```

Recommendation:

* show off/provider state inside AI Settings, not as a persistent main-header lamp;
* show active request/review context in AI Chat, Review Suggestions, or the status row.

### 2. AI Chat Overlay

Invoking `AI: Open Chat` opens a compact transcript overlay. If no callable
provider is ready yet, AI Settings opens first and focuses provider setup.

It should show:

* active chat context;
* current focus context summary;
* quick prompts;
* provider/model indicator;
* review-action entry when staged edits exist;
* settings entry for provider setup and on/off;
* option to cancel.

Example:

```text
AI Chat

Context: Root
Provider: local-llama
Sends: whole map on the first turn when it fits; focus, context, and recent chat every turn

Quick prompts:
› Find missing requirements
  Clean up structure
  Extract open questions
  Draft acceptance criteria

[Enter] run  [Tab] preview context  [Esc] cancel
```

### 3. Context Preview

Before sending context to a networked provider, the user should be able to preview what will be sent.

Example:

```text
Context Preview

Scope: Current branch
Nodes: 18
Details: 4
Provider: OpenAI-compatible endpoint

Included:
- Launch Plan
- Goals
- Non-goals
- Risks
- Open Questions

Not included:
- Other branches
- Files outside this map
- Shell environment
```

For local-only providers, context preview can be optional but still available.

### 4. Review Tray

AI results appear in a review tray.

The tray should show:

* action performed;
* proposed changes;
* rationale when helpful;
* validation status;
* accept/reject controls;
* partial accept support;
* regenerate/refine option.

Example:

```text
Review Suggestions: Find missing requirements

Proposed additions under “Requirements”:

[+] Add “Offline behavior”
    Rationale: The PRD says AI is optional but does not define offline UX.

[+] Add “Provider error handling”
    Rationale: Users need clear recovery when endpoints fail.

[+] Add “Context preview”
    Rationale: Required for trust before networked calls.

Actions:
[A] accept all  [Space] toggle  [R] refine  [Esc] reject
```

### 5. Diff Preview

For edits to existing nodes, show a small before/after diff.

Example:

```text
Edit label

Before:
“AI command thing”

After:
“Contextual AI assistance”

Reason:
Clearer and more product-specific.
```

### 6. Calm Empty States

If AI is not configured:

```text
AI is off.

mdmind works fully without AI.
To enable optional assistance, connect a local model or OpenAI-compatible endpoint.

[Configure] [Not now]
```

If no useful suggestion is available:

```text
I do not see a strong next move yet.

You can still ask a question, summarize this branch, or choose a larger scope.
```

This should feel honest, not broken.

## V1 Scope

### Must Have

* AI off by default
* AI configuration flow
* Provider profile support
* One primary command: `AI: Open Chat`
* Context detection for selected scope
* Dynamic suggested actions
* Context preview for networked providers
* Structured suggestions
* Review tray
* Accept/reject suggestions
* Undo accepted changes
* Safe secret handling

### Should Have

* Local model endpoint support
* Provider/model switching
* Partial accept
* Regenerate/refine suggestion
* Basic branch classification
* Diff preview for label/detail edits
* Checkpoint before larger AI batches

### Could Have

* Local CLI bridge
* Custom user recipes
* Saved prompt styles
* Source-backed research action
* “Ask a freeform question” mode
* Passive suggestions behind explicit opt-in

### Not Yet

* Autonomous agents
* Background watchers
* Whole-workspace editing
* Automatic web browsing
* Team provider management
* Hosted `mdmind` AI service
* Always-open chat sidebar

## V1 Internal Actions

Even though the user should experience contextual suggestions, the system can rely on a small set of internal action types.

Internal action types:

```text
Generate structure
Transform structure
Extract items
Review for gaps
Summarize
Answer question
Prepare handoff
```

These are implementation primitives, not necessarily command palette items.

### Generate Structure

Used when the branch is sparse or the user wants to continue building.

Examples:

* add child nodes;
* suggest missing sections;
* create a planning skeleton;
* draft next steps.

### Transform Structure

Used when the branch is messy or unclear.

Examples:

* clean up child nodes;
* rewrite labels;
* group related items;
* move detail text into better nodes.

### Extract Items

Used when details contain useful buried structure.

Examples:

* extract TODOs;
* extract open questions;
* extract decisions;
* extract risks;
* extract requirements;
* extract assumptions.

### Review for Gaps

Used when the map appears mature enough to critique.

Examples:

* find missing requirements;
* find contradictions;
* find risks without mitigations;
* find tasks without owners;
* find vague decisions.

### Summarize

Used when the user wants a portable view of the map.

Examples:

* summarize current branch;
* create executive summary;
* create changelog-style summary;
* summarize decisions and open questions.

### Answer Question

Used when the user asks directly about the selected context.

Examples:

* “What is weak here?”
* “What should I do next?”
* “Is this enough for a PRD?”
* “What would an engineer need?”

### Prepare Handoff

Used when the map is intended for another person or agent.

Examples:

* create implementation brief;
* draft agent prompt;
* extract acceptance criteria;
* list constraints and unknowns.

## Provider Model

`mdmind` should support provider profiles.

### Provider Types

Initial provider types:

```text
OpenAI-compatible HTTP endpoint
Local HTTP model endpoint
Local agent CLI
```

### Quick-Add Profiles

The setup flow should include a few quick-add profiles so users can try the
feature without learning every provider field first.

Initial quick-add profiles:

| Profile | Type | Defaults | Secret handling |
| --- | --- | --- | --- |
| NVIDIA NIM | OpenAI-compatible HTTP | `https://integrate.api.nvidia.com/v1`, `nvidia/llama-3.3-nemotron-super-49b-v1.5`, Chat Completions defaults from NVIDIA's example request | Use `env:NVIDIA_API_KEY` by default; get keys from <https://build.nvidia.com/settings/api-keys>; never store the raw key in mdmind config |
| Ollama Local | Local HTTP | detect running local Ollama, list `/api/tags`, choose a chat-capable installed model, use `http://127.0.0.1:11434/v1` | No mdmind API key |
| Codex Local | Local agent CLI | `codex exec` | No mdmind API key; relies on the installed Codex CLI's own auth/config |
| Claude Local | Local agent CLI | `claude -p` in stream-json mode with plan permissions, disabled tools, one turn, and no session persistence | No mdmind API key; relies on the installed Claude CLI's own auth/config |

TUI-first slice:

```text
AI: Open Chat
› Use Ollama Local
› Set up NVIDIA NIM
  Use Codex Local
  Use Claude Local
  Use NVIDIA NIM
```

The command palette has one AI entry point. A user should be able to enable AI
without knowing that `mdm ai` exists, but AI-specific choices should happen in
AI Chat or AI Settings rather than scattered through the global palette.

Initial palette/menu entries:

```text
AI: Open Chat
```

Initial AI Settings actions:

```text
Chat
  Open AI Chat
  Context
  Cancel response
Map edits
  Review map edits
Providers
  Use Ollama Local
  Set up NVIDIA NIM
  Use NVIDIA NIM
  Use Codex Local
  Use Claude Local
Settings
  Turn AI off
```

Setup actions should write only non-secret profile config. NVIDIA NIM
should open a small masked input prompt that links to
<https://build.nvidia.com/settings/api-keys>, accepts a pasted API key, stores
the key outside the profile JSON, and points the profile at a local secret
reference such as `local:mdmind.ai.nvidia-nim`. Codex Local and Claude Local
should not need separate setup actions; choosing either inside AI Settings
should record/select that profile because it relies on the installed CLI's own
auth/config.

CLI support may still exist for scripts, tests, and debugging:

```text
mdm ai presets
mdm ai quick-add nvidia-nim --secret-ref env:NVIDIA_API_KEY --default
mdm ai quick-add codex-local --default
```

The CLI is not the onboarding experience.

AI TUI surfaces are experimental. They should be hidden by default and exposed
only when `mdmind` is launched with `--experimental-ai` or
`--experimental=ai`. When enabled, the header should show a warning-colored
`EXPERIMENTAL AI` badge so users know they are in a provisional workflow.
The implementation should use named runtime experiments rather than a single
"turn on every unfinished thing" flag; future experiments can join the same
mechanism without sharing AI-specific state.

First usable TUI slice:

```text
AI: Open Chat
  P Open AI Settings
  X Cancel response
  S Review map edits
  T Change chat context in AI Chat
```

AI Chat should be a session surface, not a single Q/A review bin. Opening it
shows the current chat transcript for this TUI session. Pressing Enter opens a
compact composer inside the chat; submitting sends the new message, recent chat
turns, current focus branch, and chat context to the active provider. On the
first turn, it also sends the whole map when the map is reasonably sized; large
maps get an omission note instead of spending the token budget on the full tree.
The chat header should show the active provider, model name, and estimated total
tokens for the current TUI session, so `NVIDIA NIM` is not ambiguous once
multiple models or providers exist.
Responses stream back into AI Chat. Conversational answers stay in the panel.
Structured edit suggestions become Review Suggestions rows only when the user
explicitly asks for map edits or changes they can review and apply.
The chat composer should teach stable phrases: use `suggest new ...`,
`suggest missing ...`, or `suggest` with map, branch, node, or outline edits
when the user wants staged Review Suggestions.
The AI Chat selected-turn action should live in the highlighted action strip,
not the footer, and call the action `suggest`, not `convert`; the action asks
the model for staged edit suggestions based on that answer.
Review Suggestions should display the triggering prompt as a distinct
`Prompt > ...` provenance line so users can tell whether they are reviewing the
right request.
AI Chat behaves like a chat log, not a review queue: it does not show a Dismiss
action for conversational turns. `[ ]` moves between turns, arrows scroll the
transcript, Home/End jump to first/latest, and the header makes the current
turn position visible. Users can clear AI Chat to start a fresh conversation;
that clears in-memory chat context for future asks without applying or deleting
currently staged Review Suggestions.
It is intentionally smaller than the final structured workflow, but it gives
users a clear way to try the configured provider from inside the map.

If the user writes another chat message while a response is streaming, the
composer should explain the replacement behavior: pressing Enter cancels the
previous response, keeps any partial output in AI Chat as cancelled, and starts
the new request. Pressing Esc closes only the composer and leaves the current
response running.

The TUI must never block while waiting for a provider. When a chat response
starts, `mdmind` should keep AI Chat open, stream answer text into the chat
body, and keep the status line available. The header should not
add provider, working, or review lamps for ordinary AI Chat. When the response
completes, it should stay staged under the branch that was originally asked
about, and the user should review, accept, or dismiss it before the map changes.
Failures should clear the working state and explain which provider failed.

Provider selection:

* the active provider is stored as the default profile in `ai-profiles.json`;
* setup may add multiple profiles, but only one is active at a time;
* if the active local provider becomes unavailable, AI Chat should not silently
  fall back to a different configured provider;
* switching active providers starts a fresh AI Chat conversation so prior chat
  turns are not sent to a different provider; staged Review Suggestions are kept;
* AI Chat uses the active callable profile;
* Codex Local is callable through `codex exec` in read-only, ephemeral mode and
  receives the same prompt contract through stdin;
* Claude Local is callable through `claude -p` in stream-json mode with plan
  permissions, disabled tools, one turn, no session persistence, and the same
  prompt contract through stdin.

## AI UI Flows

The UI should make AI feel present but never in the way. The map remains the
primary surface. AI lives in a thin status strip, compact prompts, and a
review-focused popup only when there is something to decide.

Normal INFO/status text and AI state should not share the same sentence. When
AI is active, the bottom chrome should stack them as separate rows:

```text
INFO  Saved. Autosave is on.
AI    NIM response · "Find gaps..." · open AI Chat
FOCUS Product Plan   SCOPE Current branch
```

In minimal mode, the same separation still holds, just with less context:

```text
info  Saved.
ai    NIM answer · "Find gaps..."
```

This prevents AI from feeling like it has taken over normal interface feedback.

Turn Off should be an explicit AI Settings action. Turning AI off should:

* stop any in-flight TUI AI job;
* mark the active job as cancelled so late chunks/results are ignored;
* clear active AI Chat state while keeping staged Review Suggestions available;
* keep staged Review Suggestions available without lighting the main header;
* keep provider profiles and stored keys intact;
* allow users to turn AI back on by choosing a provider inside AI Settings.

### Provider Setup Flow

1. User opens the palette and chooses `AI: Open Chat`; because setup is needed,
   AI Settings opens first.
2. Inside settings, the user chooses setup or provider actions.
3. For NVIDIA NIM, mdmind opens a masked key prompt with one visible key-page
   link: <https://build.nvidia.com/settings/api-keys>.
4. On save, mdmind writes the key to the local secret store, writes only a
   `local:...` secret reference into the profile JSON, and activates the
   provider.
5. AI Settings marks NVIDIA NIM as active; the main header remains quiet.

Codex Local and Claude Local should be selected through AI Settings, not
separate global commands. They should not require an mdmind API key; they rely
on the installed CLI's own auth/config. When a local CLI bridge is active, the
AI Settings and AI Chat metadata should be explicit:

```text
Provider Codex Local | Model Codex CLI default
Provider Claude Local | Model Claude CLI default
```

### AI Chat Flow

1. User focuses a branch and opens `AI: Open Chat`.
2. Optionally, the user presses T in AI Chat to pin AI Chat to
   another branch. The target prompt accepts partial names, ids, breadcrumbs,
   tags, and small typos; Tab completes the best match to a stable id when
   available.
   Without an explicit target, AI Chat uses the current root. Empty input keeps
   the root default; `.` pins the currently selected node.
3. AI Chat opens as a transcript. Pressing Enter opens a compact composer that
   shows target and provider:

```text
Provider: Ollama Local, NVIDIA NIM, Codex Local, or Claude Local
Context: Current chat context
Message:
[________________________________________]
```

4. Submit returns immediately to AI Chat on a streaming response. AI Chat shows
   the provider, target, and activity:

```text
AI Chat | streaming | NIM | Product Plan | "Find gaps..."
```

5. The model response is staged in AI Chat unless it contains structured
   edit suggestions. The map is unchanged.
6. If structured map edits are staged, Review Suggestions becomes available
   from AI Chat or AI Settings:

```text
Review Suggestions | NIM | 3 suggestions
```

Conversational answers should stay inside AI Chat. They should not be written
into the map just because the user sent a message. Only structured edit
suggestions should become checkable/applicable rows.

Cancelled chat turns and dismissed/applied Review Suggestions should move into
in-memory AI session history. That history is visible from AI Chat until the TUI
session ends, but is not written to map files or sidecars.

### Review Suggestions Flow

Review Suggestions is the decision surface for AI Chat and background
automation outputs that propose map edits. It should open from AI Chat, AI Settings,
or a future single-key focus action. It should not show conversational Q/A
entries.

```text
┌─ Review Suggestions: Product Plan ──────────────────────┐
│ NIM · AI Chat · "Find gaps in this branch"               │
│                                                          │
│ [x] ADD     Product Plan / Requirements                  │
│     Add child: "Offline behavior #risk"                  │
│     Rationale: offline behavior is not specified.        │
│                                                          │
│ [x] UPDATE  Product Plan / Milestones                    │
│     Change detail: add owner/date reminder.              │
│     Preview: + Owner and target date are missing.        │
│                                                          │
│ [ ] REMOVE  Product Plan / Duplicate notes               │
│     Remove duplicate node.                               │
│     Destructive suggestions start unchecked.             │
│                                                          │
│ [Space] toggle  [PgUp/PgDn] scroll  [A] apply  [D] dismiss│
│ [Tab]/] next item  [ previous item  [Esc] close           │
└──────────────────────────────────────────────────────────┘
```

Review rules:

* additions may default checked when confidence is high;
* add-child suggestions may omit a target to apply under the chat context branch,
  or name an existing descendant/chat context branch target when the new node belongs
  deeper in the opened context;
* the header summarizes placement across branches, and each add-child row says
  `Place under ...` so the landing spot is explicit before apply;
* updates may default checked only when the diff is small and target resolution
  is exact;
* removals should default unchecked;
* every row shows operation, target path, preview, rationale, and source run;
* users can jump to the target without applying;
* applying selected suggestions creates one undoable edit group and checkpoint
  label;
* unsupported suggestion types remain visible but disabled with an explanation;
* answer bodies and long suggestion lists are scrollable;
* Review Suggestions browses only currently staged map-edit items.
* AI Chat browses streaming/current conversational answers and read-only
  in-memory session history.

### Error And Offline Flow

AI failures must return control with visible state, not a frozen prompt:

```text
NIM request timed out | retry from AI Chat or Review Suggestions
```

If AI is off:

```text
mdmind works fully without AI
```

If a provider is configured but unavailable, the UI should show whether the
issue is missing credentials, unsupported adapter, endpoint failure, or pending
local-agent bridge.

## Model Contract And Tool Strategy

The app should prefer structured suggestions over free-form text whenever the
requested output could change the map. The model can still answer in prose, but
map edits should be represented as reviewable operations.

Internal suggestion shape:

```json
{
  "target": {
    "path": [0, 2],
    "id": "product/risks",
    "label": "Risks"
  },
  "profile_label": "NVIDIA NIM",
  "question": "Find gaps in this branch.",
  "changes": [
    {
      "operation": "add_child",
      "target": {
        "path": [0, 2],
        "id": "product/risks",
        "label": "Risks"
      },
      "fragment": "Offline behavior #risk",
      "detail": "Offline behavior is not specified."
    },
    {
      "operation": "update_node",
      "target": {
        "path": [0, 2, 1],
        "id": "product/risks/provider-errors",
        "label": "Provider errors"
      },
      "detail": "Add recovery guidance for timeout and auth failures."
    },
    {
      "operation": "remove_node",
      "target": {
        "path": [0, 2, 3],
        "id": "product/risks/duplicate",
        "label": "Duplicate note"
      }
    }
  ]
}
```

For add-child rows, `target` is optional. When absent, the row applies under the
chat context branch for that AI Chat turn. When present, the app resolves the
target by id, path, or label/path and only applies it when it is inside the chat
chat context branch scope.

If a provider supports tool calling, mdmind should expose tools that propose
changes, not tools that mutate the map:

```text
propose_add_child(target, fragment, detail, rationale, confidence)
propose_update_node(target, fragment?, detail?, rationale, confidence)
propose_remove_node(target, rationale, confidence)
ask_clarifying_question(question)
```

Tool calls enqueue suggestions in `AiReviewQueue`. They do not call
`Editor` directly. This keeps local undo, checkpoints, and user review as the
only mutation path.

If a provider does not support tool calling, the adapter should request strict
JSON with the same shape for edit-producing tasks. If parsing fails, mdmind
should keep the prose answer in AI Chat and clearly mark it as non-applicable
instead of inventing a map edit.

Prompt context for edit-producing tasks should include the current focus branch,
the chat context branch, and compact context style notes: task-marker use, common
tags, metadata keys, stable id examples, detail-line usage, relation styles, and
external reference styles. On the first chat turn, include the whole map when it
is reasonably sized; for large maps, omit it and say so rather than wasting the
token budget.
The system prompt should incorporate the local `mdmind-map-authoring` guidance:
build a readable tree first; keep labels short; preserve the user's framing;
use tags, metadata, ids, details, relations, and external refs only when useful
or already established locally; add ids only for durable branches; keep
relations sparse and meaningful. Suggestions should align with the target
branch's nearby siblings and descendants before inventing new structure.

Validation before review:

* target resolves by stable id when possible, then by path fallback;
* node fragments parse with mdmind's normal parser;
* updates show a real before/after preview;
* removals require an exact target and start unchecked;
* suggestions outside the requested scope are rejected or marked disabled;
* every accepted change uses the normal editor mutation path.

Library strategy:

* keep provider code isolated inside the AI adapter layer;
* use raw OpenAI-compatible HTTP for NIM and other compatible endpoints while it
  is simple and portable;
* evaluate an OpenAI Rust client or other SDK only behind the adapter boundary
  when it materially improves streaming, tool calls, retries, or structured
  output handling;
* do not let SDK-specific types leak into `interactive.rs` or `editor.rs`;
* require any chosen library to support custom base URLs so NVIDIA NIM, local
  OpenAI-compatible servers, and future providers remain first-class.
* show local model-server and local CLI providers only when mdmind detects that
  they are actually available on the local machine.

### Provider Profile Fields

Each profile should include:

```text
Display name
Provider type
Base URL or command
Model name
Secret reference
Enabled/disabled state
Capabilities
Default scope behavior
```

### Bring Your Own Model

`mdmind` should not hardcode one model ecosystem.

Users should be able to bring:

* OpenAI-compatible endpoints;
* local model servers such as Ollama or LM Studio;
* authenticated local CLIs such as Codex or Claude Code;
* future custom adapters.

## Privacy and Trust

### AI Off by Default

AI must be disabled by default.

The user should be able to use the full core TUI without AI setup.

### No Hidden Network Access

No map content should be sent to a provider unless the user explicitly invokes AI.

Networked AI calls should be clearly marked.

### Scope Before Send

Every AI request should have an explicit scope:

```text
Current node
Current branch
Visible map
Whole map
```

The default should be conservative: current branch.

### Context Preview

For networked providers, users should be able to preview the context before sending.

The preview should show map content included and excluded.

### Secret Storage

API keys must not be stored in:

* map files;
* sidecar files;
* logs;
* shell history.

Preferred secret storage order:

```text
1. OS keychain or credential manager
2. Environment variable reference
3. Explicit local secret store
```

If secure storage is unavailable, `mdmind` should warn clearly before saving anything sensitive.

### Local CLI Warning

Local CLI agents may have permissions outside `mdmind`.

If the user configures a local CLI bridge, the setup flow should explain:

```text
Local agent CLIs may access files, tools, or networks according to their own configuration.
mdmind can limit what context it sends, but it cannot control every permission of the external CLI.
```

## Structured Suggestion Model

AI output that modifies the map should be structured.

Each suggestion should include:

```text
Suggestion ID
Target node
Operation type
Proposed content
Optional rationale
Validation status
```

Initial operation types:

```text
Add child
Edit label
Edit details
Add tag
Add metadata
Add TODO
Add note
Move node
Group nodes
Create summary node
```

For v1, `Move node` and `Group nodes` can be behind a stricter review path because they are riskier.

## Validation

`mdmind` must validate AI suggestions before they can be applied.

Validation should check:

* target node exists;
* operation is supported;
* resulting map remains parseable;
* metadata is valid;
* no unsupported fields are introduced;
* no hidden file writes are requested;
* no conflicting patch is applied without review.

Invalid suggestions should be shown as failed output, not applied.

Example:

```text
This suggestion could not be applied safely.

Reason:
Target node no longer exists.

Options:
[Retry with current branch] [Dismiss]
```

## Apply, Undo, and Checkpoints

Accepted AI changes should behave like normal map edits.

Requirements:

* every accepted suggestion is undoable;
* partial acceptance is supported;
* larger batches create a checkpoint;
* rejected suggestions leave no trace unless the user chooses to save them.

Before applying larger AI batches:

```text
Checkpoint created: before-ai-cleanup-2026-05-30
```

The user should feel safe experimenting.

## Error Handling

Errors should be calm and useful.

### Provider Unavailable

```text
AI provider unavailable.

local-llama did not respond.
Check that the local server is running or switch providers.
```

### Invalid API Key

```text
Provider authentication failed.

The saved key for “work-openai-compatible” was rejected.
Update the key or choose another provider.
```

### Model Returned Invalid Output

```text
The model returned a response that mdmind could not safely apply.

You can retry, ask for a simpler suggestion, or view the raw response.
```

### Context Too Large

```text
This branch is too large for the selected model.

Try:
- current node only
- visible branch
- summarize first
- choose a larger-context model
```

## Delightful Design Details

### 1. The AI “glance”

When AI help opens, the first line should show that `mdmind` understands the current work.

Examples:

```text
This looks like rough product planning.
```

```text
This branch is task-heavy but missing owners.
```

```text
This looks like a handoff map for implementation work.
```

```text
This branch is sparse. I can help you build it out.
```

This creates the “oh, nice” moment.

### 2. Suggestion chips

Use compact suggestion chips, not a giant menu.

Example:

```text
Useful next steps

› Find gaps
  Extract TODOs
  Draft acceptance criteria
  Summarize for handoff
```

### 3. Confidence without overclaiming

Avoid fake certainty.

Good:

```text
This looks like meeting notes.
```

Better when uncertain:

```text
This may be meeting notes or rough planning.
```

Bad:

```text
Detected MEETING_NOTES with 94% confidence.
```

### 4. “Why this?” affordance

Each suggested action can optionally explain itself.

Example:

```text
Find missing requirements
Why? This branch has goals and non-goals, but no acceptance criteria.
```

This makes the AI feel thoughtful, not magical in a suspicious way.

### 5. Beautiful review language

The review tray should use human, map-native language.

Instead of:

```text
Patch operation: add_node
```

Use:

```text
Add child under “Requirements”
```

Instead of:

```text
Mutation failed
```

Use:

```text
This suggestion could not be applied safely.
```

### 6. Low-friction dismissal

The user should be able to close AI instantly.

```text
[Esc] dismiss
```

Dismissal should not feel like canceling a workflow or losing state.

### 7. No guilt

If AI is off, the product should not imply the user is missing out.

Good:

```text
AI is off. mdmind works fully without it.
```

Bad:

```text
Unlock smarter productivity with AI.
```

### 8. Local-first pride

For local providers, the UI can make that feel good.

Example:

```text
Provider Ollama Local
Model llama3.2:latest
Local HTTP only
```

That is a trust-building detail.

### 9. Soft presence, sharp action

The AI indicator can be quiet, but the suggestions should be precise.

This balance matters:

```text
Small surface.
High usefulness.
```

### 10. Make accepted changes feel satisfying

After applying suggestions:

```text
Applied 4 suggestions.
Added 3 nodes.
Updated 1 detail block.
Undo available.
```

No celebration confetti. Just crisp confirmation.

## TUI Experience Sketches

### AI Off

```text
┌──────────────────────────────────────────────┐
│ mdmind                                       │
│                                              │
│  Launch Plan                                 │
│  ├─ Goals                                    │
│  ├─ Requirements                             │
│  └─ Risks                                    │
│                                              │
│                                              │
└──────────────────────────────────────────────┘
```

### AI Settings Setup Action

```text
┌──────────────────────────────────────────────┐
│ mdmind                                       │
│                                              │
│  Launch Plan                                 │
│  ├─ Goals                                    │
│  ├─ Requirements                             │
│  └─ Risks                                    │
│                                              │
│                                              │
└──────────────────────────────────────────────┘
```

Setup should feel like part of the TUI, not like a separate developer workflow.
The main map should expose a single AI Chat entry point without persistent
provider state; AI Settings owns setup so provider controls do not clutter map
navigation.

### NVIDIA NIM Key Prompt

```text
┌─ NVIDIA NIM API Key ─────────────────────────┐
│ Current key: nvapi-xxxxx3eda                 │
│ Paste a new key only if replacing it.         │
│                                              │
│ API key                                      │
│ [****************____________]               │
│                                              │
│ Key page:                                    │
│ https://build.nvidia.com/settings/api-keys   │
│                                              │
│ Enter stores  Esc cancels                    │
└──────────────────────────────────────────────┘
```

The pasted key should be masked. If a key is already stored and readable, the
prompt should show only the beginning and final four characters. The profile
should store only the secret reference, not the raw key.

### AI Chat Opened

```text
┌─ AI Chat ────────────────────────────────────┐
│ Context: Root                                │
│ Provider: local-llama                         │
│                                              │
│ No turns yet.                                 │
│                                              │
│ Quick prompts:                                │
│ › Find missing requirements                   │
│   Extract open questions                      │
│   Draft acceptance criteria                   │
│                                              │
│ [Enter] message  [P] settings  [T] context    │
└──────────────────────────────────────────────┘
```

### Context Preview

```text
┌─ Context Preview ────────────────────────────┐
│ Provider: work-endpoint                       │
│ Input: Current branch                         │
│                                              │
│ Sending:                                     │
│ - Launch Plan                                │
│ - Goals                                      │
│ - Requirements                               │
│ - Risks                                      │
│ - Open Questions                             │
│                                              │
│ Not sending:                                 │
│ - Other map branches                          │
│ - Local files                                 │
│ - Shell environment                           │
│                                              │
│ [Enter] send  [Esc] cancel                    │
└──────────────────────────────────────────────┘
```

### Review Tray

```text
┌─ Review Suggestions: Missing Requirements ───┐
│ Proposed additions                            │
│                                              │
│ [x] Add “Offline behavior”                    │
│     AI is optional, but offline behavior       │
│     is not defined.                           │
│                                              │
│ [x] Add “Provider errors”                     │
│     Users need recovery paths when AI fails.  │
│                                              │
│ [ ] Add “Billing limits”                      │
│     May be useful later, but not required     │
│     for local-first v1.                       │
│                                              │
│ [A] apply selected  [R] refine  [Esc] reject  │
└──────────────────────────────────────────────┘
```

## Architecture

### Core Components

```text
AiProviderProfile
AiSecretRef
AiAdapter
AiContextBuilder
AiContextClassifier
AiSuggestionEngine
AiSuggestion
AiSuggestionValidator
AiReviewQueue
AiPatchApplier
AiAuditLogger
```

### Request Flow

```text
Selected scope
→ Context builder
→ Context classifier
→ Suggested actions
→ User chooses action
→ Provider adapter
→ Raw AI response
→ Structured parser
→ Validator
→ Review tray
→ User accepts
→ Patch applier
→ Undo/checkpoint
```

### Adapter Layer

Adapters should normalize different runtimes into one internal interface.

Initial adapters:

```text
OpenAI-compatible HTTP adapter
Local HTTP adapter
Local CLI adapter
```

The adapter should hide provider-specific details from the rest of the product.

### Capability Detection

Provider profiles may expose capabilities such as:

```text
Text generation
Structured output
Large context
Local only
Tool use
Streaming
```

The UI can use these capabilities to decide which suggestions are available.

Example:

* source-backed research should not appear unless browsing/tool support exists;
* very large branch review should warn if the model context is small;
* structured patch actions should prefer providers with reliable structured output.

## MVP Recommendation

Build v1 around one primary command:

```text
AI: Open Chat
```

Behind that, implement:

```text
Context detection
Dynamic suggestions
Structured suggestion generation
Review tray
Safe apply
Undo/checkpoint
Provider setup
```

Recommended v1 contextual suggestions should map to these internal actions:

```text
Generate structure
Transform structure
Extract items
Review for gaps
Summarize
Answer question
Prepare handoff
```

Do not ship a large static AI command list as the primary UX.

A static list may exist as an advanced fallback, but the product should lead with contextual assistance.

## Success Criteria

### Qualitative

Users should say:

```text
“It understood what kind of map I was working on.”
```

```text
“It helped without taking over.”
```

```text
“I trusted what it was sending.”
```

```text
“The review flow made me comfortable applying changes.”
```

```text
“I can ignore AI when I do not want it.”
```

### Behavioral

Useful local metrics:

```text
AI configured successfully
AI help invoked
Suggestion shown
Suggestion accepted
Suggestion partially accepted
Suggestion rejected
Suggestion refined
Undo after AI apply
Provider error
Validation error
```

Telemetry should be opt-in only.

### Product Health

The feature is working if:

* users can configure AI without confusion;
* contextual suggestions are accepted more often than rejected;
* users rarely undo accepted suggestions;
* network/context boundaries are understood;
* non-AI users do not feel bothered;
* the command palette does not become cluttered.

## Risks and Mitigations

### Risk: AI feels noisy

Mitigation:

* one primary entry point;
* no passive suggestions in v1;
* no always-open assistant panel;
* subtle status rows only when there is active AI state;
* easy dismissal.

### Risk: AI feels unsafe

Mitigation:

* AI off by default;
* explicit scope;
* context preview;
* structured suggestions;
* validation;
* review before apply;
* undo/checkpoint.

### Risk: AI feels too generic

Mitigation:

* classify map type;
* classify branch state;
* show contextual suggestions;
* generate map-native patches;
* avoid vague prose where structured output is expected.

### Risk: Provider setup is confusing

Mitigation:

* simple provider profiles;
* clear local vs networked labels;
* compatibility check;
* useful error messages;
* secure secret handling.

### Risk: Local CLI bridge is dangerous

Mitigation:

* make it advanced;
* require explicit configuration;
* warn about external permissions;
* do not allow arbitrary shell command execution in the friendly setup path.

### Risk: The product becomes an agent platform too early

Mitigation:

* v1 focuses on assistance, not autonomy;
* no unreviewed background agents;
* no workspace edits;
* no tool execution beyond configured provider calls;
* all output goes through review.

## Open Questions

* Should context preview be mandatory for every networked request?
* How much of the internal classifier output should be visible to users?
* Should local CLI bridge ship in v1 or wait until HTTP providers are excellent?
* Should custom user recipes be available in v1?
* Which freeform chat affordances belong in the first release versus later?
* Should accepted AI suggestions be stored in map history as AI-authored changes?

## Recommended Product Decisions

### Decision 1: Use one primary AI entry point

Use:

```text
AI: Open Chat
```

Do not lead with a long command list.

While AI is experimental, that entry point is visible only with the AI
experiment flag.

### Decision 2: Make AI contextual first

AI should inspect the selected scope and suggest next moves.

The interaction should feel intelligent before the model generates anything.

### Decision 3: Keep passive suggestions out of v1

Passive suggestions may be powerful later, but they are also the easiest way to make the TUI annoying.

Start with explicit invocation.

### Decision 4: Make review the core interface

The review tray is more important than chat.

If the review experience is excellent, users will trust AI-assisted editing.

### Decision 5: Treat providers as user-owned

Support bring-your-own endpoint and local runtime workflows.

Do not create a hosted AI dependency for v1.

### Decision 6: Optimize for calm confidence

The design should feel:

```text
plain
fast
safe
useful
quiet
```

Not:

```text
flashy
chatty
agentic
mysterious
salesy
```

Current TUI design commitments:

* the only global palette action is `AI: Open Chat`;
* AI Chat is the default AI destination; Review Suggestions, provider setup, provider switching, on/off, and local provider choices live inside AI Settings;
* provider state lives in AI Settings and AI Chat/Review headers, not in the main map header or scattered global commands;
* working state is specific: provider, elapsed time, and context branch;
* AI Chat accepts freeform requests for input, critique, cleanup, or reviewable map edits;
* conversational responses stay in the panel; structured edit suggestions are staged before they become normal editable map content;
* duplicate requests are blocked with a helpful message rather than queued silently;
* the user can continue navigating while the request runs.

## Final Product Shape

The best version of this feature is not “AI commands in a TUI.”

It is:

```text
A map-aware thinking assistant that appears only when invited,
understands the shape of the current work,
offers useful next moves,
turns model output into reviewable map suggestions,
and then gets out of the way.
```

That is the product promise.

That is the delight.

That is what makes AI feel native to `mdmind`.
