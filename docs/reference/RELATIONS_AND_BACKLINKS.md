# Relations And Backlinks

## Goal

Let maps express meaningful connections that are not strict parent/child structure.

The tree remains the core model, but some ideas naturally cross branches.

## Problem

Once maps cover requirements, tasks, prompts, and implementation notes, important relationships become lateral:

- a TODO references a requirement
- a prompt references a constraint
- a design note supports several features

Without relations, these links either stay hidden or get duplicated in the tree.

## UX

Support optional inline references and derived backlinks.

Potential syntax direction:

- explicit relation: `[[rel:blocks->product/mvp]]`
- lightweight reference: `[[product/mvp]]`

TUI behavior:

- focus panel shows outgoing references
- backlinks panel shows incoming references
- a command jumps across relations
- the future mindmap view can render relation edges

## Model

Keep relations additive:

- the tree remains authoritative for structure
- relations are references, not extra parents
- backlinks are derived, not stored separately

## Validation

- unresolved targets should be warnings or errors depending on command mode
- cycles in relations are allowed
- cycles in tree structure remain impossible

## Delivery

Phase 1:

- parse references
- validate targets
- show backlinks in CLI and TUI

Phase 2:

- relation-aware navigation
- render relation edges in visual views

## Risks

- relation syntax becoming noisy in plain text
- users overusing relations and weakening tree clarity

## Product Test

The feature succeeds when users can connect ideas across branches without flattening the tree into a graph-shaped mess.
