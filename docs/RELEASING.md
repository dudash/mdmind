# Releasing mdmind

This repo should ship like a modern Rust CLI/TUI:

- CI on every push and pull request
- semver in `Cargo.toml`
- git tags like `v0.1.0`
- GitHub Releases with prebuilt binaries
- a Homebrew tap for easy install on macOS and Linux

## What is in the repo now

- `.github/workflows/ci.yml`
  - runs `cargo fmt --check`
  - runs `cargo clippy --locked --all-targets -- -D warnings`
  - runs `cargo test --locked`
  - smoke-builds on Ubuntu, macOS, and Windows
- `.github/workflows/release.yml`
  - triggers on tags matching `v*.*.*`
  - verifies the tag matches `Cargo.toml`
  - builds `mdm` and `mdmind`
  - uploads platform archives to the GitHub Release

## Release versioning policy

Use semantic versioning:

- `MAJOR`: breaking CLI, file-format, or behavior changes
- `MINOR`: new features and non-breaking UX improvements
- `PATCH`: bug fixes, small polish, and release-only fixes

Examples:

- `0.2.0` for a new big-map workflow or visual overlay milestone
- `0.2.1` for search or save/revert fixes

## Supported release targets

The release workflow currently builds:

- `x86_64-unknown-linux-gnu`
- `x86_64-apple-darwin`
- `aarch64-apple-darwin`
- `x86_64-pc-windows-msvc`

If Linux ARM becomes important, add:

- `aarch64-unknown-linux-gnu`

## Release process

1. Update `Cargo.toml` to the new version.
2. Update `README.md` and any release notes if the UX changed materially.
3. Run:

```bash
cargo fmt
cargo test
```

4. Commit the release prep.
5. Create and push the tag:

```bash
git tag v0.1.0
git push origin v0.1.0
```

6. GitHub Actions builds binaries and uploads the release artifacts for that tag.

## Install surfaces

These are the recommended install paths:

- local dev: `cargo run --bin mdm -- ...` and `cargo run --bin mdmind -- ...`
- local install: `cargo install --path .`
- public binary install: GitHub Releases
- package-manager install: Homebrew tap

## Homebrew support

The right early-stage path is a custom tap, not `homebrew/core`.

Recommended shape:

- app repo: `dudash/mdmind`
- tap repo: `dudash/homebrew-tap`
- install command:

```bash
brew install dudash/tap/mdmind
```

Each release should publish:

- macOS `x86_64` and `aarch64` archives
- SHA256 checksums for each archive

The tap formula should download the matching GitHub Release asset for the version.

## Before publishing widely

Do these before promoting installs broadly:

- decide whether to publish to crates.io
- add a changelog if release cadence becomes regular
- consider replacing the hand-written release workflow with `cargo-dist` if packaging needs become more advanced
