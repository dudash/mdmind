# Command Palette And Fuzzy Search

## Goal

Make large maps feel fast by giving `mdmind` one keyboard-first entry point for actions, jumps, filters, and node search.

## Problem

As the feature set grows, discoverability and navigation speed will suffer if every capability is just another hotkey.

Large maps also need faster branch lookup than linear arrow-key navigation.

## UX

Open a palette with a key such as `:` or `Ctrl+P`.

The palette supports several modes from one input:

- action search: `save`, `revert`, `add child`, `export png`
- node search: fuzzy match on labels, ids, tags, and metadata
- command search: `filter #todo`, `group by status`
- recent items: recently opened ids, filters, and actions

## Results

Results should be grouped and ranked:

- actions first when the query looks command-like
- node hits first when the query looks like content
- exact id matches strongly boosted
- tag and metadata hits visible but secondary

## Interaction

- `Up` / `Down`: move selection
- `Enter`: run action or jump
- `Tab`: cycle result groups if needed
- `Esc`: close palette

## Scope

Early palette behavior should focus on:

- jump to node by fuzzy label or id
- run common actions
- reopen saved views
- open built-in help topics

Do not try to replace every dedicated key on day one.

## Architecture

- provider model for actions, node search, filters, and recents
- shared fuzzy-ranking utility
- structured result type with label, kind, preview, and execute callback target

## Delivery

Phase 1:

- node jump
- action search
- saved views
- help topics
- implemented first slice

Phase 2:

- richer previews
- recent items
- scoped search within subtree or filter

## Risks

- too many low-quality results
- overloading one input with too many mental models

## Product Test

A user with a large map should be able to jump to the intended branch or action in a few keystrokes without remembering every hotkey.
