# PRD: Markdown Viewing and Pretty Rendering

**Status:** Draft for review  
**Linear:** MDM-59  
**Last reviewed:** 2026-05-23  
**Product surface:** `mdm`, `mdmind`, shared Markdown rendering layer  
**Primary promise:** mdmind can read the Markdown around your maps without pretending every Markdown file is a native map.

---

## 1. Summary

`mdmind` should let users comfortably read ordinary Markdown files in the CLI and TUI while preserving strict native mdmind map parsing.

The user-facing behavior should feel simple:

```bash
mdmind README.md
mdmind CHANGELOG.md
mdm render README.md
mdm changelog
```

When the file is a valid mdmind map, open the normal map experience.

When the file is ordinary Markdown, open a beautiful read-only Markdown view.

When the file looks like a broken mdmind map, protect the user, explain the issue, and offer the next safe action.

The core design principle:

> Opening a file should do the kind thing by default, without weakening the map format.

This feature is intentionally separate from mindspace. Mindspace is about folder-level workspaces, multiple maps, mixed Markdown folders, agents, context bundles, and workspace maintenance. This feature is narrower:

> Make single Markdown files pleasant to read inside today’s mdmind surfaces.

---

## 2. Product Thesis

Markdown viewing should be a delight layer, not a format compromise.

Users should never need to understand internal classification unless something goes wrong. The product should feel like this:

```text
I opened a README. mdmind showed me a nice README.
I opened a map. mdmind showed me my map.
I opened a broken map. mdmind protected me and told me how to fix it.
```

Internally, mdmind needs classification, rendering models, parser choices, and CLI/TUI reuse. Externally, the user should feel:

- It knew what I meant.
- It did not destroy anything.
- It gave me the right next move.
- It looks better than raw Markdown.
- It still feels like mdmind.

The feature should not make users feel like mdmind grew a file-type control panel. It should make mdmind feel more welcoming.

---

## 3. Problem

mdmind maps are Markdown-like, but not every `.md` file is a mdmind map.

Real folders often look like this:

```text
project/
  roadmap.md          # native mdmind map
  decisions.md        # native mdmind map
  README.md           # ordinary Markdown
  CHANGELOG.md        # ordinary Markdown
  agent-report.md     # ordinary Markdown
  notes.md            # ordinary Markdown
```

Today, the product is strongest when the file is a native map. It is weaker when the file is ordinary Markdown:

- `mdm changelog` can emit useful release notes, but raw Markdown is not delightful.
- `mdmind README.md` does not have a first-class read-only document path.
- `mdm view normal.md` is map-oriented, so it is the wrong mental model for ordinary Markdown.
- `mdm import normal.md --from markdown` exists for conversion, but reading is not importing.
- `.md` references can be previewed from a map, but that preview is not yet a shared Markdown rendering layer.

The user need is simple:

> I’m already in mdmind. Let me read the Markdown next to my maps without making me leave or convert it.

---

## 4. Goals

### User Goals

Users should be able to run:

```bash
mdmind README.md
mdmind CHANGELOG.md
mdmind docs/agent-report.md
mdm render README.md
mdm changelog --pretty
```

and get a pleasant read-only Markdown experience.

### Product Goals

This feature should:

- make ordinary Markdown readable in mdmind;
- preserve strict parsing and validation for native mdmind maps;
- improve `mdm changelog` for humans;
- keep scripts and JSON output stable;
- reuse one rendering model across CLI and TUI;
- avoid exposing users to unnecessary mode complexity;
- create a clean future path for import preview and repair preview.

---

## 5. Non-Goals

This feature does not add:

- Markdown editing;
- WYSIWYG editing;
- a general notes app;
- an Obsidian replacement;
- folder-level workspace behavior;
- multi-map switching;
- Markdown sync;
- Markdown backlinks;
- automatic Markdown-to-mdmind conversion;
- automatic repair;
- LLM summarization;
- full GitHub-Flavored Markdown parity.

The first slice is:

> Read beautifully. Do not mutate.

---

## 6. Delight Principles

This feature should be judged as much by feel as by capability.

### 6.1 The Happy Path Should Require Zero Choices

A user should not see a prompt like this on normal open:

```text
Choose file type:
1. mdmind map
2. Markdown document
3. Importable outline
```

That is technically honest but emotionally clunky.

Instead:

- valid map opens as a map;
- normal Markdown opens as a document;
- suspicious broken map opens a calm recovery screen.

Users should only see the machinery when the product genuinely needs their judgment.

### 6.2 Read-Only Should Feel Safe, Not Limited

Do not make the Markdown view feel like a disabled editor. Make it feel like a polished reader.

Good header:

```text
Markdown · README.md · read-only
```

Avoid copy that sounds like failure:

```text
Editing disabled
```

When a user presses an editing key, teach instead of scolding:

```text
This is a read-only Markdown view. Press i to import and open README-mind.md.
```

### 6.3 The TUI Should Reward Opening Common Files

`README.md` and `CHANGELOG.md` are emotional entry points. They should look good.

This means headings, spacing, bullets, code blocks, and links need to feel intentionally designed, not merely parsed.

### 6.4 The Product Should Explain Itself in One Line

Header:

```text
Markdown · README.md · read-only
```

Footer:

```text
↑↓ scroll · / search · i import map · q quit
```

No mystery. No docs required.

If a footer action is not implemented yet, do not show it. The footer should only advertise working affordances.

### 6.5 Recovery Should Be Kind

When a file looks like a broken mdmind map, mdmind should not dump parser internals first.

Good recovery copy:

```text
This looks like a mdmind map, but it has 3 structure issues.

Recommended:
  mdm validate broken.md

Other options:
  mdmind --as markdown broken.md
  mdm import broken.md --from markdown --preview
```

Then show the first error.

The tone should be calm and specific. The product should feel protective, not punitive.

### 6.6 Beauty Comes From Restraint

Do not over-style the terminal.

Use:

- whitespace;
- indentation;
- wrapping;
- quiet emphasis;
- a small number of semantic styles.

Avoid:

- heavy borders around every block;
- too many colors;
- theme settings in the MVP;
- noisy symbols that make Markdown feel busier than the source.

### 6.7 Power Should Be Discoverable, Not Thrown at the User

The first view should be simple. Advanced actions can exist behind clear keys or CLI flags.

Good:

```text
↑↓ scroll · / search · r raw · q quit
```

Later, once implemented:

```text
↑↓ scroll · / search · o open link · i import preview · r raw · q quit
```

Do not show users five half-working pathways just because the architecture can support them.

---

## 7. Target Users

### Primary: mdmind User in a Mixed Markdown Folder

This user uses mdmind maps for decisions, plans, todos, and structured thinking, but the same folder has READMEs, changelogs, docs, and reports.

They want mdmind to be hospitable to those files without turning them into maps.

### Secondary: Terminal-First Developer

This user wants a nice terminal rendering of release notes, project docs, or agent reports.

They care that:

```bash
mdm changelog
```

is readable, and:

```bash
mdm changelog --json
```

does not break.

### Tertiary: Agent-Heavy User

This user receives or generates Markdown reports from agents. Some reports may later become maps, but first they just want to read them.

They want a safe separation between:

- reading a report;
- importing an outline;
- repairing a near-miss map;
- editing a native map.

---

## 8. User Stories

### Story 1: Read the Changelog Beautifully

As a user, I want:

```bash
mdm changelog
```

to show readable release notes in my terminal.

Acceptance:

- headings are distinct;
- bullets wrap cleanly;
- code fences are readable;
- links are visible;
- raw/script output remains available through `--plain` and `--json`;
- `mdm changelog --json` remains unchanged.

### Story 2: Render Any Markdown File From CLI

As a user, I want:

```bash
mdm render README.md
```

to pretty-print Markdown in the terminal.

Acceptance:

- supports headings, paragraphs, lists, task lists, code fences, blockquotes, links, horizontal rules, and simple tables;
- exits clearly for missing or unreadable files;
- does not require the file to be a mdmind map;
- never rewrites the file.

### Story 3: Open README in TUI

As a user, I want:

```bash
mdmind README.md
```

to open a read-only Markdown view.

Acceptance:

- opens without import;
- clearly shows `Markdown · read-only`;
- scroll works;
- search works if existing search can be reused;
- editing commands do not mutate the file;
- no map checkpoints are created for ordinary Markdown.

### Story 4: Preserve Strict Map Behavior

As a user, I want broken mdmind maps to remain strict.

Acceptance:

- valid maps open as maps;
- near-miss maps do not silently open as ordinary Markdown;
- the user gets validate, import-preview, and force-Markdown options;
- `mdm validate` remains the source of truth.

### Story 5: Recover From Ambiguity Gracefully

As a user, I want mdmind to make a smart default decision but give me an escape hatch.

Acceptance:

- `mdmind --as markdown file.md` forces Markdown reader mode;
- `mdmind --as map file.md` forces strict map parsing;
- error messages show the suggested command for the other path.

---

## 9. Product Behavior

### 9.1 File Open Behavior

When the user runs:

```bash
mdmind file.md
```

mdmind should route the file as follows:

| File type | User-facing behavior |
| --- | --- |
| Valid mdmind map | Open normal map TUI |
| Ordinary Markdown | Open read-only Markdown view |
| Near-miss mdmind map | Show friendly structure issue screen |
| Non-Markdown file | Existing behavior or unsupported-file message |

The routing should feel automatic, not like a setup step.

### 9.2 Explicit Overrides

Support:

```bash
mdmind --as map file.md
mdmind --as markdown file.md
```

These are escape hatches, not the normal UX.

`--as map` forces strict map parsing.

`--as markdown` forces read-only Markdown view.

### 9.3 CLI Commands

#### `mdm changelog`

Recommended behavior:

```bash
mdm changelog              # pretty Markdown rendering
mdm changelog --pretty     # explicit pretty Markdown rendering
mdm changelog --plain      # raw Markdown
mdm changelog --json       # existing JSON behavior
```

Implementation recommendation:

1. Ship `--pretty` and `--plain` first.
2. Default changelog to pretty for humans.
3. Keep `--plain` and `--json` as the stable script paths.
4. Respect `NO_COLOR` and avoid ANSI when stdout is not a TTY.

#### `mdm render`

Add:

```bash
mdm render <file.md>
mdm render <file.md> --pretty
mdm render <file.md> --plain
mdm render <file.md> --width 100
mdm render <file.md> --no-color
```

Do not overload `mdm view`.

`mdm view` should remain map-oriented. If a user runs:

```bash
mdm view README.md
```

on ordinary Markdown, return:

```text
README.md is ordinary Markdown, not a native mdmind map.

To read it:
  mdm render README.md

To convert an outline:
  mdm import README.md --from markdown --preview
```

This is a delightful error because it teaches the product without scolding the user.

---

## 10. TUI Markdown View

### 10.1 View Concept

Add a read-only TUI mode:

```text
MarkdownDocumentView
```

This is not a map view. It is a document reader.

Suggested header:

```text
Markdown · README.md · read-only
```

Suggested MVP footer:

```text
↑↓ scroll · / search · i import map · r raw · q quit
```

Suggested later footer:

```text
↑↓ scroll · / search · o open link · i import map · r raw · q quit
```

Keep the first version modest. Do not show link focus until it actually works.

### 10.2 Visual Design

Rendered Markdown should feel calm:

- H1 gets strong spacing and emphasis.
- H2 and H3 get lighter emphasis.
- Paragraphs wrap to terminal width.
- Bullets indent consistently.
- Nested bullets stay readable.
- Code fences render as visually separate blocks.
- Blockquotes are quiet, not noisy.
- Links show both label and target when helpful.
- Tables degrade gracefully.

Example:

```text
Markdown · README.md · read-only

mdmind
======

A local-first thinking tool for structured maps in plain text.

Install
-------

  brew tap dudash/tap
  brew install mdmind

Why mdmind
----------

• product and feature planning
• research and writing maps
• project breakdowns
• keyboard-first personal planning
```

No heavy boxes around every element. Let whitespace do most of the work.

### 10.3 Navigation

Minimum controls:

| Key | Behavior |
| --- | --- |
| `j` / `↓` | Scroll down |
| `k` / `↑` | Scroll up |
| `PageDown` / `Space` | Page down |
| `PageUp` | Page up |
| `g` | Top |
| `G` | Bottom |
| `/` | Search |
| `n` | Next search result |
| `N` | Previous search result |
| `r` | Toggle raw/rendered view, optional |
| `q` / `Esc` | Quit or back |

### 10.4 Edit-Key Behavior

Do not just beep.

If a user presses an editing key:

```text
README.md is open in Markdown view.
Press i to import and open README-mind.md, or q to return.
```

### 10.5 First-Open Delight

When opening ordinary Markdown in the TUI for the first time, show a small, dismissible hint in the footer area:

```text
Opened as Markdown · read-only · maps still open normally
```

This reassures the user that mdmind made an intentional decision.

### 10.6 Raw Toggle

A raw/render toggle is useful because users sometimes want to inspect exact Markdown.

Recommended key:

```text
r
```

Behavior:

- rendered view is default;
- raw view preserves source text and line wrapping;
- header changes to:

```text
Markdown source · README.md · read-only
```

This is not required for the first implementation if it complicates the TUI state model. It is a high-delight, low-concept feature once the renderer is in place.

---

## 11. File Classification

Classification is internal routing, not a user-visible workflow.

### 11.1 Algorithm

Step 1: Try strict mdmind parse.

```text
If valid mdmind map:
  open map TUI
```

Step 2: If parse fails, score mdmind intent.

Strong mdmind signals:

- `[id:...]`
- `[[rel:kind->target]]`
- mdmind-style branch links
- mdmind detail lines using `|`
- dense `@key:value` metadata
- dense `#tag` usage on outline nodes
- sidecar evidence for this file
- nested bullet structure with mdmind tokens

Weak ordinary Markdown signals:

- prose-first heading and paragraphs;
- frontmatter;
- code fences;
- tables;
- normal Markdown links;
- changelog/date headings;
- README-style sections.

Step 3: Route.

```text
valid parse              => NativeMap
invalid + strong intent  => NearMissMap
invalid + weak intent    => OrdinaryMarkdown
```

### 11.2 Suggested Scoring

```text
+5  contains [id:...]
+5  contains [[rel:...->...]]
+4  known mdmind sidecar nearby
+3  contains mdmind-style [[target/id]]
+3  contains detail lines under bullets: /^\s*\|\s+/
+2  contains repeated @key:value metadata on bullet lines
+2  contains task markers in nested outline structure
+1  contains multiple #tags on bullet lines

-3  starts with README/prose-style heading and paragraph
-2  has fenced code blocks before outline structure
-2  has Markdown tables before outline structure
-2  has frontmatter plus prose-first structure
```

Initial threshold:

```text
score >= 5 => near-miss mdmind map
score < 5  => ordinary Markdown
```

Err toward ordinary Markdown unless there are strong mdmind signals. A README with checkboxes should not be punished.

### 11.3 Classification Examples

| Input | Classification | Behavior |
| --- | --- | --- |
| Valid mdmind map with ids | Native map | Open map TUI |
| Broken map with `[id:...]` | Near-miss | Show structure issue screen |
| README with headings/prose | Ordinary Markdown | Open Markdown view |
| CHANGELOG with headings/bullets | Ordinary Markdown | Pretty render |
| Agent report with headings/code fences | Ordinary Markdown | Pretty render |
| Markdown checklist only | Ordinary Markdown | Pretty render |
| Outline with relations but parse errors | Near-miss | Validate/import guidance |

---

## 12. Rendering Architecture

The repo already has separate `mdm` and `mdmind` binaries. This makes a shared library renderer the right design.

Recommended module shape:

```text
src/
  markdown/
    mod.rs
    classify.rs
    parse.rs
    model.rs
    render_ansi.rs
    render_tui.rs
    wrap.rs
```

### 12.1 Internal Model

Avoid making the CLI and TUI each parse Markdown independently.

Use one intermediate representation:

```rust
pub struct MarkdownDocument {
    pub blocks: Vec<MarkdownBlock>,
    pub links: Vec<MarkdownLink>,
    pub diagnostics: Vec<MarkdownDiagnostic>,
}

pub enum MarkdownBlock {
    Heading { level: u8, text: Vec<Inline> },
    Paragraph { text: Vec<Inline> },
    BulletList { items: Vec<ListItem> },
    OrderedList { items: Vec<ListItem> },
    TaskList { items: Vec<TaskItem> },
    CodeBlock { language: Option<String>, text: String },
    BlockQuote { blocks: Vec<MarkdownBlock> },
    Table { rows: Vec<Vec<Vec<Inline>>> },
    HorizontalRule,
}

pub enum Inline {
    Text(String),
    Emphasis(String),
    Strong(String),
    Code(String),
    Link { label: String, target: String },
}
```

Then render that model into:

- ANSI lines for CLI;
- `ratatui` lines/spans for TUI;
- raw Markdown passthrough for `--plain`.

### 12.2 Parser Recommendation

Use a CommonMark parser rather than hand-rolling Markdown.

Recommended default: `pulldown-cmark`.

Why:

- mature Rust crate;
- event-based parser;
- good enough for CommonMark;
- supports useful extensions behind options;
- easy to transform into an internal IR.

Avoid trying to implement all GitHub-Flavored Markdown in the MVP. Tables and task lists are worth supporting if cheap; footnotes, math, HTML blocks, and advanced extensions can degrade gracefully.

### 12.3 Style Tokens

Use semantic styling, not theme complexity.

Initial style tokens:

```rust
pub enum MdStyle {
    Normal,
    Heading1,
    Heading2,
    Heading3,
    Bullet,
    TaskOpen,
    TaskDone,
    Code,
    CodeFence,
    Link,
    BlockQuote,
    Dim,
    Error,
}
```

Do not expose themes in MVP. Respect `--no-color`, `NO_COLOR`, and non-TTY output.

### 12.4 Rendering Pipeline

Recommended pipeline:

```text
read file
  -> classify
  -> parse Markdown if needed
  -> MarkdownDocument IR
  -> layout lines for target width
  -> render to ANSI or TUI spans
```

Keep wrapping and layout separate from ANSI/TUI styling. This lets the CLI and TUI share line-breaking behavior.

---

## 13. CLI Output Rules

### 13.1 TTY vs Pipe

Default behavior should be safe:

| Situation | Default |
| --- | --- |
| TTY | Pretty output with ANSI styling when color is allowed |
| Piped stdout | Pretty output without ANSI styling |
| `--pretty` | Pretty output |
| `--plain` | Raw Markdown |
| `--json` | JSON only |
| `NO_COLOR` set | No ANSI color |
| Dumb terminal | Plain or minimal formatting |

### 13.2 Width

Default width:

- detect terminal width;
- fallback to 88 or 100;
- allow override:

```bash
mdm render README.md --width 120
```

### 13.3 Exit Codes

Suggested:

| Code | Meaning |
| ---: | --- |
| `0` | Success |
| `1` | General render/read error |
| `2` | Invalid arguments |
| `3` | File not found |
| `4` | Unsupported input type |
| `5` | Markdown parse/render warning promoted to error, future |

---

## 14. Import and Repair Path

This feature should make conversion explicit and one-way by default.

From Markdown view:

```text
i import map
```

Writes a new sibling map:

```text
README-mind.md
```

The original Markdown file remains unchanged. The imported map opens immediately after a successful import.

The imported map ends with a review section:

```text
Lossy Import Summary #lossy-summary @source:markdown
```

That section names the source file, states that the original was left unchanged, and reminds the user that Markdown formatting may have been simplified into outline nodes and detail lines.
It should also state whether non-empty text lines were dropped; when the importer preserved every non-empty source line, say so explicitly.

If the sibling output already exists, the TUI prompts instead of silently failing:

```text
Type a new name or press Enter to overwrite: README-mind.md · Esc cancels
```

Later, the TUI can show a split preview before writing:

```text
Left: original Markdown
Right: proposed mdmind map
```

Important principle:

> Import is explicit, previewable, and lossy by design.

No silent conversion.

No fixing files on open.

No rewriting ordinary Markdown unless the user explicitly chooses an import or repair path.

---

## 15. Related TUI Delight Improvements

These are related enough to consider while touching the TUI, but they should not all be required for MDM-59.

### 15.1 Friendlier Empty and Error States

Instead of raw parser errors first, show a calm state card:

```text
This file looks like a mdmind map, but it has structure issues.

3 issues found.
First issue: line 14 has an indented detail without a parent node.

Recommended:
  mdm validate notes.md
```

This would improve the whole product, not just Markdown view.

### 15.2 Consistent “What Can I Do Here?” Footers

Every major view should have a concise footer with only valid actions.

Map view footer:

```text
↑↓ move · Enter edit · / search · v view · ? help
```

Markdown view footer:

```text
↑↓ scroll · / search · r raw · q quit
```

Table view footer:

```text
↑↓ row · c columns · Enter focus · q back
```

A consistent footer makes the TUI feel designed rather than assembled.

### 15.3 Gentle First-Run Hints

Add small, dismissible hints when users enter a mode for the first time:

```text
Tip: Press v to focus this branch without rewriting the map.
```

The best hint is short, contextual, and easy to ignore.

### 15.4 Better Relation and Backlink Pickers

If there are multiple outgoing relations or backlinks, mdmind should help the user choose without guessing.

Make pickers more delightful with relation kind, target title, and breadcrumb:

```text
Follow relation

→ blocks     API contract decision       product/api
→ informs    Launch checklist            release/readiness
→ supports   Migration plan              infra/migration
```

### 15.5 A Reusable Reader Mode Design Language

Markdown view could establish a reusable reader mode visual language for:

- help screens;
- changelog;
- reference previews;
- import previews;
- validation explanations;
- future review reports.

That gives the TUI more polish without adding many concepts.

### 15.6 Beautiful Command Confirmations

When an action succeeds, use compact confirmations:

```text
Copied branch link: onboarding/decision
```

```text
Opened README.md as Markdown
```

```text
Focused branch: Launch readiness
```

These small confirmations add delight because they reinforce that the system understood the user’s intent.

### 15.7 Respect the User’s Focus

Avoid interruptive prompts. Prefer:

- status-line confirmations;
- footer hints;
- command palette entries;
- explicit recovery screens only when needed.

The TUI should feel calm under the user’s hands.

---

## 16. Milestones

### Milestone 1: Shared Renderer Foundation

Ship:

- Markdown parser module;
- internal Markdown document model;
- ANSI renderer;
- wrapping utilities;
- basic snapshot tests.

Supports:

- headings;
- paragraphs;
- bullets;
- ordered lists;
- task lists;
- code fences;
- inline code;
- links;
- blockquotes;
- horizontal rules.

### Milestone 2: CLI Pretty Output

Ship:

```bash
mdm render <file.md>
mdm changelog --pretty
mdm changelog --plain
```

Keep:

```bash
mdm changelog --json
```

unchanged.

Add TTY detection only when tests are ready.

### Milestone 3: TUI Markdown View

Ship:

```bash
mdmind README.md
mdmind --as markdown README.md
mdmind --as map roadmap.md
```

Include:

- read-only header;
- scrolling;
- raw/render toggle if cheap;
- `i` imports to `<stem>-mind.md`, adds a `#lossy-summary`, and opens the imported map;
- if `<stem>-mind.md` exists, the user can press Enter to overwrite the default, type a new name, or cancel;
- friendly edit-key message;
- safe quit/back behavior.

### Milestone 4: Classification and Near-Miss Handling

Ship:

- strict parse first;
- mdmind intent scoring;
- ordinary Markdown fallback;
- near-miss recovery screen;
- fixture suite for common cases.

### Milestone 5: Reference Preview Reuse

Use the shared Markdown renderer for `.md` previews where practical.

This should be the last milestone, not the first. Get direct file open right before threading it through previews.

---

## 17. Acceptance Criteria

### Functional

- `mdmind valid-map.md` opens normal map TUI.
- `mdmind README.md` opens read-only Markdown view.
- `mdmind CHANGELOG.md` opens read-only Markdown view.
- `mdmind broken-map.md` shows near-miss guidance when strong mdmind signals exist.
- `mdmind --as markdown broken-map.md` opens Markdown view.
- `mdmind --as map README.md` attempts strict map parse and fails clearly.
- pressing `i` in Markdown view imports to `README-mind.md`, adds a `#lossy-summary`, opens the imported map, and leaves `README.md` unchanged.
- `mdm render README.md` pretty-prints Markdown.
- `mdm render README.md --plain` prints raw Markdown.
- `mdm changelog --json` remains unchanged.
- `mdm changelog --pretty` renders terminal-friendly Markdown.

### Safety

- Ordinary Markdown view does not rewrite files.
- Ordinary Markdown import writes only a sibling `<stem>-mind.md` file unless the user explicitly chooses another name or overwrite.
- Ordinary Markdown view does not create map checkpoints.
- Ordinary Markdown view does not make the file pass `mdm validate`.
- Near-miss mdmind files do not silently open as ordinary Markdown unless forced.
- Raw/script output remains available through explicit `--plain` and `--json` modes.

### Delight

- README and CHANGELOG look intentionally designed.
- First-time users can understand the mode from the header and footer.
- Error messages recommend the next action.
- Pressing an edit key in Markdown view teaches instead of scolding.
- Pressing `i` in Markdown view gives the user a useful converted map and moves them into it without making them leave the TUI.
- Common files open with no extra decision prompts.
- Raw/rendered toggle, if shipped, feels instant and reversible.

### Test Fixtures

Add fixtures for:

```text
valid-map.md
broken-map-with-id.md
readme.md
changelog.md
agent-report.md
markdown-checklist.md
markdown-with-frontmatter.md
markdown-with-code-fences.md
markdown-table.md
outline-without-mdmind-signals.md
outline-with-relations-broken.md
```

---

## 18. Risks and Mitigations

### Risk: The Classifier Annoys Users

If a README with checkboxes is treated as a broken mdmind map, the feature will feel hostile.

Mitigation:

- require strong mdmind signals for near-miss classification;
- add fixtures for common README, changelog, and checklist patterns;
- provide `--as markdown` escape hatch.

### Risk: Renderer Scope Creeps

Full Markdown compatibility can become endless.

Mitigation:

- support the common reading subset first;
- degrade gracefully;
- do not block MVP on advanced GitHub-Flavored Markdown features.

### Risk: `mdm view` Meaning Gets Muddy

If `mdm view` starts rendering ordinary Markdown, map users and scripts may get confused.

Mitigation:

- add `mdm render`;
- keep `mdm view` map-native;
- show a helpful redirect when `mdm view` receives ordinary Markdown.

### Risk: TUI Gets Too Many Modes

Markdown view should be simple and obviously read-only.

Mitigation:

- one header;
- one footer;
- one purpose;
- no editing commands;
- no workspace behavior in this feature.

### Risk: Pretty Output Breaks Scripts

Scripts may rely on exact changelog output.

Mitigation:

- keep `--json` unchanged;
- add `--plain`;
- document `--plain` as the raw Markdown path;
- avoid ANSI color when stdout is not a TTY;
- keep default pretty output structurally stable enough for humans, not exact-match scripts.

---

## 19. Implementation Notes

### 19.1 Suggested Public Types

```rust
pub enum OpenClassification {
    NativeMap,
    NearMissMap { score: i32, signals: Vec<ClassificationSignal> },
    OrdinaryMarkdown,
    Unsupported,
}

pub struct RenderOptions {
    pub width: usize,
    pub color: ColorMode,
    pub target: RenderTarget,
}

pub enum RenderTarget {
    CliAnsi,
    Tui,
    Plain,
}
```

### 19.2 Suggested Tests

- classification unit tests;
- renderer snapshot tests;
- CLI golden output tests;
- TTY/non-TTY behavior tests where practical;
- TUI state tests for read-only mode;
- regression tests for `mdm changelog --json`.

### 19.3 Performance

Markdown files are usually small enough for full-file parse and render on open.

Still, use practical safeguards:

- avoid reparsing on every scroll;
- cache rendered lines for current width;
- re-render only on terminal resize;
- support large file warning later if needed;
- avoid syntax highlighting in MVP.

### 19.4 Accessibility and Terminal Compatibility

- Respect `NO_COLOR`.
- Avoid relying on color alone for meaning.
- Use text labels in headers and footers.
- Keep Unicode tasteful but not required; provide ASCII fallback if the project already supports it.

---

## 20. Recommendation

Build this feature, but keep it elegant and small.

The final user-facing story should be:

> mdmind now opens Markdown nicely. Maps still behave like maps.

Implementation priority:

1. shared renderer;
2. `mdm render`;
3. `mdm changelog --pretty`;
4. `mdmind README.md` read-only view;
5. near-miss protection;
6. reference preview reuse.

The most important product call:

> Do not expose complexity unless the user needs recovery.

Automatic routing should feel smart. Every fallback should be safe, kind, and reversible.

---

## 21. Reference Context

This PRD was developed in the context of mdmind’s existing CLI/TUI split, native map strictness, import behavior, reference previews, and future mindspace planning.

Useful related docs:

- `README.md`
- `docs/product/shipped/cli-inspection-and-export.md`
- `docs/product/shipped/focused-views.md`
- `docs/product/shipped/metadata-table-view.md`
- `docs/reference/CROSS_LINKS_AND_BACKLINKS.md`
- `docs/design/LLM_WIKI_MINDSPACE_STRATEGY.md`
