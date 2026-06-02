# AI Assisted Map UI Flows

## Goal

Make AI assistance feel like a focused collaborator inside the TUI, not a
generic chatbot bolted onto the side.

The AI feature should help users reason about a map, ask branch-local questions,
and stage reviewable map edits without taking over the outline. Conversational
answers stay in AI Chat. Suggested map changes go through Review Suggestions.
Nothing silently edits the map.

## Availability

AI is experimental. It is hidden unless `mdmind` is launched with one of:

```text
mdmind --experimental-ai <map>
mdmind --experimental=ai <map>
```

When enabled, the main header shows an experimental AI badge. AI entry points are
then available through the command palette and built-in help.

## Product Principles

- Keep the map primary. AI surfaces should be panels and prompts, not permanent
  clutter over the outline.
- Use one main AI entry point. The global palette opens the AI Panel; provider
  setup, chat, review, on/off, and future hooks live there.
- Keep chat and edits separate. AI Chat is a transcript. Review Suggestions is a
  decision surface for staged map changes.
- Make intent visible. The UI should always show what the user is chatting about,
  what input the model receives, and what will or will not change the map.
- Prefer natural language over modes. Users should be able to type "suggest new
  characters" without learning a special hotkey for edit mode.
- Stage before applying. Every map edit must be inspectable, toggleable when
  supported, and undoable after apply.
- Keep state local and reversible. Secrets are stored as local secret references,
  chat history is in memory for the TUI session, and provider profiles survive
  turning AI off.

## Main Map Surface

The main map remains the working surface. AI adds only small orientation cues:

- experimental badge when AI is enabled;
- AI lamp in the header, such as `AI OFF`, `AI NIM`, `AI CODEX`, `AI WORKING`,
  `AI REVIEW`, or `AI PENDING`;
- command palette entry for `AI: Open Panel`;
- normal status messages for AI actions.

The main map should not expose separate global commands for each provider or AI
action. That keeps the normal TUI command surface calm.

### Main Map Hotkeys

| Key | Behavior |
| --- | --- |
| `:` / `Ctrl+P` | Open command palette. Search for `AI: Open Panel`. |
| `?` | Open help. AI help appears only when experimental AI is enabled. |

## AI Panel

The AI Panel is the central workspace for AI. It is opened from the command
palette and stays under nested prompts, so `Esc` from setup or chat composition
returns to the AI workspace instead of dumping the user back into the map.

### Layout

- Header: `AI workspace`, state, provider, model, and session token estimate.
- Left pane: grouped action list.
- Right pane: contextual workspace preview.
- Footer: compact hotkey hints.

### Sections And Actions

| Section | Action | Purpose |
| --- | --- | --- |
| Chat | Open AI Chat | Open the session transcript and composer entry point. |
| Chat | Cancel response | Stop the current streaming response, if any. |
| Suggestions | Review suggestions | Inspect staged map-change suggestions. |
| Providers | Set up NVIDIA NIM | Paste or update a NIM API key. |
| Providers | Use NVIDIA NIM | Make NIM the active OpenAI-compatible provider. |
| Providers | Use Codex Local | Make read-only `codex exec` the active local provider. |
| Automation | Automation hooks | Planned slice; currently disabled. |
| Settings | Turn AI off | Stop AI activity while keeping provider profiles, stored keys, and staged Review Suggestions. |

### AI Panel Hotkeys

| Key | Behavior |
| --- | --- |
| `Up` / `Down` / `k` / `j` | Move through panel actions. |
| `Enter` | Run the selected action. |
| `A` / `H` | Open AI Chat. |
| `X` | Cancel the running response. |
| `S` | Open Review Suggestions. |
| `N` | Set up or update NVIDIA NIM. |
| `P` | Use NVIDIA NIM. |
| `C` | Use Codex Local. |
| `O` | Turn AI off. |
| `U` | Automation hooks placeholder. |
| `Esc` / `q` | Close AI Panel. |

## Provider Setup

### NVIDIA NIM

The NIM setup flow opens a masked prompt:

- title: `NVIDIA NIM API Key`;
- prompt copy links users to `https://build.nvidia.com/settings/api-keys`;
- input is masked;
- `Enter` stores the key in the local secret store;
- the profile stores only a secret reference;
- `Esc` cancels and returns to the AI Panel.

After setup, AI Chat can use the configured NIM endpoint and model.

### Codex Local

Codex Local is selected from the AI Panel with one action. It should not require a
separate setup command.

The local bridge calls `codex exec` in read-only, ephemeral mode. It receives the
selected branch snapshot, compact branch style notes, recent AI Chat context,
and the mdmind AI instruction through stdin. It must not write files, run
destructive commands, or claim it changed the map.

### Provider Switching

Switching providers starts a fresh AI Chat conversation. Prior chat turns are not
sent to the new provider. Staged Review Suggestions are kept because they are
explicit user-review work, not provider context.

## AI Chat

AI Chat is a chat log, not a review queue. It is where users ask questions,
watch streaming answers, browse in-session turns, cancel work, and turn a
conversational answer into staged suggestions.

### Header

The header should answer three questions:

- What is the session state?
- What is this chat turn about?
- What input is being sent?

Current header shape:

```text
Chat session | State Streaming | Provider NVIDIA NIM | Model ... | Tokens ~420
Chat target Moonwake Field Guide                                LATEST | turn 1/1
Input selected branch + style notes + recent chat       New text below | End to follow
```

Do not show visual map viewport state, such as `Map view Whole map`, in AI Chat
unless it changes the model input. It reads like AI scope and creates confusion.

### Empty State

Before the first message, AI Chat shows:

- no messages yet;
- target branch;
- input summary;
- prompt to press `Enter`;
- quick prompts.

The empty state should teach by example without becoming documentation text.

### Transcript

Each turn is shown as:

```text
> Turn 1 | You | Moonwake Field Guide
  suggest new characters
  AI | streaming
  ...
```

The selected turn gets the `>` marker. Older turns are browsable. Chat history is
kept in memory until the TUI exits or the user clears chat.

### Scrolling And Following

Streaming auto-follows the latest response until the user scrolls away. Once the
user scrolls away, the header/footer show `New text below` and `End to follow`.

### AI Chat Hotkeys

| Key | Behavior |
| --- | --- |
| `Enter` / `A` | Open the message composer. |
| `1` | Quick prompt: summarize this branch. |
| `2` | Quick prompt: find gaps, risks, and open questions. |
| `3` | Quick prompt: extract TODOs, risks, and open questions. |
| `4` | Quick prompt: suggest map edits I can review. |
| `C` | Clear AI Chat and start a new conversation. Keeps staged Review Suggestions. |
| `Up` / `Down` / `k` / `j` | Scroll the transcript. Stops auto-follow while streaming. |
| `PageUp` / `PageDown` | Jump-scroll the transcript. |
| `Tab` / `]` | Move to next chat turn. |
| `BackTab` / `[` | Move to previous chat turn. |
| `Home` / `g` | Jump to the first chat turn. |
| `End` / `G` | Jump to the latest chat turn and resume follow. |
| `X` | Cancel a running response. |
| `S` | Open Review Suggestions when staged changes exist. |
| `V` | Ask AI to turn the selected answer into staged suggestions. |
| `Esc` / `q` | Close AI Chat. |

## AI Chat Composer

The composer is a prompt overlay opened from AI Chat. It keeps AI Chat and the
AI Panel underneath, so the user returns to the streaming transcript after send.

### Composer Copy

- Title: `AI Chat`.
- Hint: "Send a message about the selected branch. The response streams back
  into AI Chat."
- Action text: `Enter sends | Esc returns`.
- Footer: names the message target and explains that reviewable edits appear in
  Review Suggestions.
- Feedback panel: provider, model, input summary, edit-mode detection, and
  examples.

### Composer Intent Rules

Ordinary questions stay conversational:

- `What gaps do you see?`
- `Clean up these rough notes.`
- `Suggest a better title.`
- `Suggest a new title.`

Review Suggestions mode turns on when the user asks for reviewable edits or
additive structure:

- `suggest map changes I can review`
- `suggest branch edits`
- `suggest nodes for missing risks`
- `suggest new characters`
- `suggest missing requirements`
- `suggest a few new scenes`
- `add child nodes for the missing risks`

When suggestion mode is on, the model receives the `mdmind-suggestions` contract
and should return a fenced structured block after concise prose. When suggestion
mode is off, any model-emitted suggestion block is ignored as chat text.

### Prompt Context And Style

Every AI Chat request sends:

- the user message;
- recent in-session AI Chat turns;
- the selected branch rendered as mdmind node syntax;
- compact style notes extracted from that branch.

Style notes summarize only patterns already present in the selected branch:
task markers, common tags, metadata keys, stable id examples, detail-line usage,
relation styles, and external reference styles. Suggestion prompts should ask
the model to match the target branch's nearby siblings and descendants, keep
labels short, use the smallest useful mdmind structure, and preserve the user's
local vocabulary.

This prompt contract follows the local `mdmind-map-authoring` skill guidance:
readable tree first; tags, metadata, ids, details, relations, and external refs
only when useful; ids only on durable branches; relations sparse and meaningful.

### Composer Hotkeys

| Key | Behavior |
| --- | --- |
| `Enter` | Send the message. If another response is running, cancel it first. |
| `Esc` | Return to AI Chat without sending. |
| `Left` / `Right` | Move cursor. |
| `Home` / `End` | Move to start/end. |
| `Backspace` / `Delete` | Delete text. |
| `Alt+Backspace` / `Ctrl+Backspace` | Delete previous word. |
| printable characters | Edit the prompt text. |

## Streaming And Cancellation

Submitting a message immediately returns to AI Chat. The active turn streams in
place:

- header lamp shows `AI WORKING`;
- AI Chat shows `AI | streaming`;
- empty output shows "Waiting for first tokens...";
- partial text remains visible if cancelled.

If the user sends a new message while a response is running, mdmind cancels the
previous response, archives the partial turn as cancelled, and starts the new
request. This should feel deliberate, not like the TUI hung.

Turning AI off also cancels any active response. Staged Review Suggestions stay
available because they are explicit user-review work, not an active model
session.

## Review Suggestions

Review Suggestions is the decision surface for map-changing AI output. It should
open from the AI Panel, from AI Chat when staged suggestions exist, or after a
model response stages suggestions.

### Header

The first row shows provider and target on the left, with state/count
right-aligned:

```text
NVIDIA NIM | Moonwake Field Guide                         staged 1/2
```

Additional header rows show:

- `Prompt > "..."` so the user knows what triggered the suggestions;
- source turn, model, and token estimate;
- staging reason, when present;
- placement summary, such as "6 edits across 4 targets";
- count and applyability note.

### Rows

Each change row shows:

- selection marker;
- checkbox state;
- operation label;
- placement label, such as `Place under Characters`;
- compact diff preview.

Only supported add-child rows are enabled today. Update and remove rows can be
represented and reviewed, but applying them is not wired yet.

Rows outside the selected branch scope or with unresolved targets should be
disabled or fail safely with a status message.

### Review Suggestions Hotkeys

| Key | Behavior |
| --- | --- |
| `Tab` / `]` | Move to next staged item. |
| `BackTab` / `[` | Move to previous staged item. |
| `PageUp` / `PageDown` | Scroll the review body. |
| `Up` / `Down` / `k` / `j` | Choose a suggestion row when editable; otherwise scroll. |
| `Space` | Toggle the selected supported row. |
| `Enter` / `A` | Apply checked supported rows. |
| `D` | Dismiss the current staged suggestion item. |
| `Esc` / `q` | Close Review Suggestions. |

## Review Suggestions Outcomes

### Apply

Applying checked add-child rows:

- inserts the selected rows under their resolved targets;
- creates the normal undoable edit path;
- focuses the inserted child according to existing editor behavior;
- archives the AI suggestion as applied.

### Dismiss

Dismissing a staged suggestion:

- removes it from active Review Suggestions;
- archives it in in-memory AI history for this TUI session;
- does not mutate the map.

### Unsupported

Unsupported update/remove rows remain visible but disabled. Attempting to apply
them should explain that the operation is staged for review but not wired yet.

### Invalid Structured Output

If the model returns invalid `mdmind-suggestions` JSON, mdmind keeps the answer in
AI Chat and adds a warning explaining that suggestions could not be staged.

## Status And Session State

AI session state is intentionally in-memory except provider profiles and secret
references.

| State | Behavior |
| --- | --- |
| Active provider | Loaded from AI profile config. |
| NVIDIA key | Stored as a local secret reference, not raw key material in the profile. |
| AI Chat transcript | In memory for the TUI session. |
| Staged Review Suggestions | In memory until applied, dismissed, cleared, or TUI exit. |
| Applied edits | Normal map edits with undo/checkpoint behavior. |
| Provider switch | Clears chat context, keeps staged Review Suggestions. |
| Turn AI off | Cancels active response, clears active AI panel/chat state, keeps staged Review Suggestions, profiles, and keys. |

## Whole Map Requests

The current shipped input summary is branch-local:

```text
Input selected branch + style notes + recent chat
```

The preferred future whole-map UX is intent-driven, not another hotkey:

- user types "summarize the whole map", "suggest gaps across the whole map", or
  similar;
- composer preview changes to `Message target Whole map`;
- input summary changes to `whole map + style notes + recent chat`;
- the resulting turn is labeled `You | Whole map`;
- Review Suggestions still require explicit user intent and clear placement.

This avoids a hidden persistent scope mode while making the model input obvious.

## Automation Hooks

Automation hooks are planned and currently disabled in the AI Panel. The intended
UX is to reuse the same Review Suggestions queue:

- user chooses event type, path scope, and instruction prompt;
- background output stages suggestions only;
- user reviews and applies through the existing Review Suggestions surface.

Examples:

- on added node under root, check spelling;
- on added node under Characters, suggest relationships with other characters.

## Delight And Fit

The AI feature should feel quiet and capable:

- thin status signals instead of large persistent panels on the main map;
- right-aligned counters and state badges;
- concise prompt provenance;
- target-first labels;
- streaming feedback with cancellation;
- natural-language intent detection;
- no surprise writes.

The delightful moment is not that AI talks a lot. It is that the TUI always says
what AI is doing, what it is looking at, and exactly what will happen if the user
accepts a suggestion.
