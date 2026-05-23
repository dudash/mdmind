# UX Design Review

## Why This Review Exists

`mdmind` has grown a lot.

That is good because the product is now genuinely useful. It is also risky because every new capability can make the core mental model harder to hold in your head.

The main design question is no longer just “what should we add?”

It is:

- what belongs in the product
- where it belongs
- how to keep the product feeling intuitive as it grows

## Current Core Mental Model

Right now the product is strongest when it stays grounded in a few simple ideas:

- one line is one node
- the tree is the primary structure
- tags, metadata, ids, relations, and details add lightweight structure around that tree
- view modes change projection, not the document
- palette and help reduce memory load
- local files stay the source of truth

That model is still good. New features should fit it instead of diluting it.

## Complexity Risks

### Too Many Surfaces With Similar Jobs

The main danger is adding multiple ways to solve the same problem:

- palette vs search vs browse vs section list
- minimal mode vs focused views vs reading mode
- ids vs relations vs saved views vs recent places

If each surface does not have a clear job, the product starts to feel clever instead of clear.

### Turning Plain Text Into A Feature Dump

The native format works because it is readable.

That means:

- avoid adding too many new raw syntax forms
- prefer TUI rendering and workflows before adding new file syntax
- use existing metadata and detail lines before inventing parallel systems

### Letting Power Features Leak Into The Beginner Path

`mdmind` is much stronger now, but a new user should still be able to ignore:

- ids
- relations
- checkpoints
- mindmap mode
- advanced palette flows

and still get useful work done.

That means discoverability should exist without making the default workflow feel crowded.

## Design Rules Going Forward

### 1. One Surface, One Job

Every major surface should have a short plain-English role.

- view modes: change what part of the tree is visible
- palette: jump or act when you know your intent
- search and browse: narrow or inspect the working set
- minimal mode: reduce chrome
- reading mode: emphasize long-form content on the current node

If a new feature cannot be described that simply, it probably is not ready.

### 2. Prefer Rendering And Workflow Before Syntax

For example:

- checklist behavior should probably start as metadata plus TUI rendering, not new raw syntax
- reading mode should be a surface change, not a file-format change
- stats should be derived, not authored

### 3. Keep The Tree Primary

Even when the app adds:

- relations
- details
- reading mode
- branch index
- spatial canvas

the product should still feel tree-first.

### 4. Advanced Features Should Compose, Not Compete

The most important pairing rule is:

- view mode = scope
- minimal mode = shell density
- reading mode = content emphasis

Those should work together instead of behaving like overlapping master switches.

### 5. Teach By Context, Not By Permanent Chrome

The footer, help, palette, and contextual hints should carry a lot of the teaching burden so the default interface can stay calmer.

## Recommended Next Features

These ideas are worth tracking, but individually:

- branch index
- detail reading mode
- checklist semantics
- document and branch stats
- outliner-oriented guides and help

They should not be treated as one “outliner parity” batch.

## Product Test

A new feature is a good fit when:

- a user can understand where it lives quickly
- it does not require learning a second mental model for the same job
- it keeps the raw file readable
- it makes one of the core workflows noticeably better
