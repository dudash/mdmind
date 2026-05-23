# Install And Release

## Install

For macOS, use Homebrew:

```bash
brew tap dudash/tap
brew install mdmind
```

For other platforms, install from the GitHub release archive:

- Linux: download the release tarball for your architecture
- Windows: download the release zip

For local development from this repo:

```bash
cargo install --path .
```

That installs both:

- `mdm`
- `mdmind`

## Release Artifacts

Release builds already produce platform archives for:

- macOS Apple Silicon
- macOS Intel
- Linux x86_64
- Windows x86_64

Those archives are attached to each GitHub release.

## Homebrew

The release workflow now generates:

- `mdmind.rb`: a Homebrew formula built from the current release tarballs
- `mdmind-vX.Y.Z-checksums.txt`: SHA256 checksums for the published archives

Expected tap shape:

- repository: `dudash/homebrew-tap`
- formula path: `Formula/mdmind.rb`

## Release Flow

Current release flow:

1. Curate the release entry in [CHANGELOG.md](../../CHANGELOG.md).
2. Prepare or tag a release.
3. Validate that `mdm changelog --version X.Y.Z` can render the target entry.
4. Build platform archives.
5. Attach archives to the GitHub release.
6. Generate checksums and a Homebrew formula from the release assets.
7. Optionally publish the formula into a tap repo.

## Changelog

`CHANGELOG.md` is the durable source for user-facing release notes. Keep it
curated and organized around user-visible change, not every commit.

Recommended sections:

- `Features`
- `New Commands`
- `Changed`
- `Fixed`
- `Closed Issues`
- `Docs`
- `Upgrade Notes`
- `Additional Notes`

Use `Closed Issues` when a release resolves a public GitHub issue that helps
users understand the change. Prefer a short link like
`[#1](https://github.com/dudash/mdmind/issues/1)` plus the user-facing outcome,
not a raw issue dump.

The CLI and TUI read the same bundled file:

```bash
mdm changelog
mdm changelog --version 0.8.0
mdm changelog --all
mdm changelog --json
```

When you only want to know whether a newer build exists, use the manual update
check:

```bash
mdm version --check
```

That command checks GitHub Releases and needs network access. The TUI exposes
the same check through the command palette as `Check For Updates`.

By default, `mdm changelog` shows the section matching the bundled app version.
Before running the release workflow, move relevant draft notes into the target
`## [X.Y.Z] - YYYY-MM-DD` section. The workflow fails if that target version
cannot be rendered by `mdm changelog --version X.Y.Z`.

## Validate Installer Metadata

For a local dry run against an existing release tag:

```bash
scripts/release/validate-release-installers.sh --tag v0.2.0 --tap-repo ../homebrew-tap
```

If Homebrew is installed locally, you can also ask the script to run `brew audit` on the generated formula:

```bash
scripts/release/validate-release-installers.sh --tag v0.2.0 --tap-repo ../homebrew-tap --check-brew
```
