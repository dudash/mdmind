# mdmind Spec

This folder defines the durable file contract for `mdmind`.

The short version:

- `mdmind` mind maps and outlines are plain `.md` files.
- The format is a Markdown-compatible profile, not a new document language.
- The tree is the source of truth.
- Tags, metadata, ids, details, and relations add structure without making the file stop feeling like text.

## Current Version

- [mdmind format 1](mdmind-1.md)

## What Counts As Normative

For format behavior, read these in order:

1. [mdmind-1.md](mdmind-1.md)
2. fixtures in [fixtures/valid](fixtures/valid) and [fixtures/invalid](fixtures/invalid)
3. parser and validation tests in [../tests](../tests)

Archived docs under `docs/_archive/` are useful background, but they are not the spec.

## Why This Lives Here

The spec lives in this repo while the format is still close to the reference implementation. If other tools start producing or consuming mdmind mind maps, outlines, or exports, this folder can become the seed for a dedicated format repo later.
