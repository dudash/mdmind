# Deep-Link Path Fallback

## Goal

Let a user open a node with a fragment like `file.md#parent/child/grandchild` even when the file does not contain explicit `[id:...]` annotations for that path.

This makes casual maps easier to navigate without forcing ids onto every branch.

## Problem

Today deep links are strongest when the map author assigns explicit ids.

That is correct for stable references, but it is too strict for quick exploration because:

- early maps often start without ids
- users still expect bookmarkable navigation
- plain-label paths are natural to type and share

## UX

Accept two deep-link styles:

- explicit id: `roadmap.md#product/mvp`
- label path fallback: `roadmap.md#Product Idea/Tasks/Ship tests`

Resolution rules:

1. Try exact id lookup first.
2. If no id matches, try label-path lookup.
3. Match path segments case-insensitively after normalization.
4. If multiple branches match the same label path, return an ambiguity error with candidate breadcrumbs.
5. If no branch matches, return the current “not found” style error.

## Normalization

Path fallback should normalize labels conservatively:

- trim whitespace
- collapse internal whitespace runs
- compare case-insensitively
- optionally treat `-` and `_` as spaces

Do not silently strip meaningful punctuation beyond that.

## Architecture

- Keep explicit ids as the canonical stable reference system.
- Implement label-path fallback as an optional second resolver.
- Do not store fallback paths in the document model.

Suggested module additions:

- path normalization helper
- label-path resolver that walks the tree by segment
- ambiguity reporter for CLI and TUI feedback

## Delivery

Phase 1:

- CLI and TUI open commands use label fallback if id lookup fails
- ambiguity errors are readable

Phase 2:

- surface “copy label path” and “copy stable id” actions in the TUI

## Risks

- repeated labels can create ambiguity
- aggressive normalization can make paths feel surprising

## Product Test

The feature succeeds when quick, unannotated maps still feel linkable, while explicit ids remain the preferred option for durable references.
