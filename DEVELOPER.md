# Developer Guide

This file is the developer-facing companion to `README.md`.

Use `README.md` for product overview and end-user usage.
Use this file for setup, repo structure, testing, CI, and release workflow.

## Local Setup

Requirements:

- Rust `1.85` or newer

Common commands:

```bash
cargo run --bin mdm -- --help
cargo run --bin mdmind -- examples/demo.md
cargo fmt
cargo test
cargo clippy --all-targets -- -D warnings
```

## Repo Structure

- `src/`
  - core parser, model, render, query, editor, session, saved-view, and TUI code
- `src/bin/`
  - binary entrypoints for `mdm` and `mdmind`
- `templates/`
  - starter maps for `mdm init`
- `tests/`
  - parser, CLI, editor, query, and saved-view coverage
- `docs/`
  - product direction and future-feature specs
- `docs/_archive/`
  - older planning docs kept for reference only

## Developer Workflow

Typical loop:

1. Make the change.
2. Run `cargo fmt`.
3. Run `cargo test`.
4. Run `cargo clippy --all-targets -- -D warnings`.
5. Smoke test the relevant binary manually if the UX changed.

Useful examples:

```bash
cargo run --bin mdm -- view examples/demo.md
cargo run --bin mdm -- find examples/demo.md "#todo"
cargo run --bin mdmind -- examples/demo.md
```

## Documentation Conventions

- Keep `README.md` user-facing.
- Put implementation and contributor workflow in `DEVELOPER.md`.
- Put feature briefs and UX specs in `docs/`.
- Avoid editing `docs/_archive/` unless you are intentionally preserving history.

## CI

GitHub Actions live in `.github/workflows/`.

`ci.yml` runs:

- `cargo fmt --check`
- `cargo clippy --locked --all-targets -- -D warnings`
- `cargo test --locked`
- a build matrix on Ubuntu, macOS, and Windows

`release.yml` runs on tags matching `v*.*.*` and:

- verifies the tag matches `Cargo.toml`
- builds `mdm` and `mdmind`
- uploads release archives to GitHub Releases

`prepare-release.yml` runs manually and:

- accepts a version input like `0.2.0`
- updates `Cargo.toml` and `Cargo.lock`
- verifies `mdm version`
- runs format, clippy, and tests
- commits the release prep
- creates tag `vX.Y.Z`
- pushes the commit and tag to `main`
- creates the GitHub Release entry
- builds platform archives
- uploads binary assets to that release

## Versioning

This repo uses semantic versioning in `Cargo.toml`.

- `MAJOR`: breaking CLI, file-format, or behavior changes
- `MINOR`: new features and non-breaking UX improvements
- `PATCH`: bug fixes and release-only fixes

Commands that surface the current version:

```bash
mdm version
mdmind --version
```

The TUI also shows the version in its header and help overlay.

## Release Process

Recommended path:

1. Land any docs or product changes before the release.
2. Run the `Prepare Release` GitHub Action on `main` with a version like `0.2.0`.
3. The workflow updates the crate version, verifies the repo, commits, tags, and pushes.
4. The same workflow creates the GitHub Release and uploads binary archives for each supported platform.
5. GitHub will also show its default source `.zip` and `.tar.gz` archives for the tag.
6. The install-metadata job generates checksums and the Homebrew formula, then publishes the formula to `dudash/homebrew-tap` when the tap settings are configured.

Useful local validation:

```bash
scripts/validate-release-installers.sh --tag v0.2.0 --tap-repo ../homebrew-tap
scripts/validate-release-installers.sh --tag v0.2.0 --tap-repo ../homebrew-tap --check-brew
```

Manual fallback:

1. Update `Cargo.toml` to the new version.
2. Update `Cargo.lock` to match.
3. Run:

```bash
cargo fmt
cargo test
cargo clippy --all-targets -- -D warnings
```

4. Commit the release prep.
5. Create and push the tag:

```bash
git tag v0.1.0
git push origin main
git push origin v0.1.0
```

## Distribution

Current and planned install surfaces:

- local dev via `cargo run`
- local install via `cargo install --path .`
- GitHub Releases for prebuilt binaries
- Homebrew via a custom tap

Recommended Homebrew shape:

- app repo: `dudash/mdmind`
- tap repo: `dudash/homebrew-tap`
- install command:

```bash
brew tap dudash/tap
brew install mdmind
```

Current release targets:

- `x86_64-unknown-linux-gnu`
- `x86_64-apple-darwin`
- `aarch64-apple-darwin`
- `x86_64-pc-windows-msvc`

GitHub’s source archives are always present too, but those are separate from the built binary assets.

## Publishing Notes

Before publishing broadly, consider:

- whether to publish to crates.io
- whether to add a changelog
- whether to replace the hand-written release workflow with `cargo-dist`
