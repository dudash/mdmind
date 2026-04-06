# CLI Help, Completions, And Manual Pages

## Goal

Make `mdm` easier to learn, package, and use in a terminal-heavy workflow.

## Problem

Strong CLI ergonomics are one of the product promises, but discoverability is incomplete without:

- shell completions
- man pages
- packaged help artifacts

## UX

Support:

- `mdm completions bash`
- `mdm completions zsh`
- `mdm completions fish`
- generated `mdm.1` and possibly `mdmind.1`

This improves:

- first-run adoption
- packaging for Homebrew or distro formulas
- offline help

## Architecture

- generate completions from the Clap command definition
- generate man pages from the same source
- keep help text authoritative in one place

## Delivery

Phase 1:

- shell completion subcommand
- man-page generation in a build or release task

Phase 2:

- install docs for packaging
- release artifact automation

## Risks

- help text drift if generation is not wired into release flow

## Product Test

The CLI should feel like a polished terminal tool, not a side project with hidden commands.
