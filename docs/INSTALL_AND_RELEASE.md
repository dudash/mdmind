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

1. Prepare or tag a release.
2. Build platform archives.
3. Attach archives to the GitHub release.
4. Generate checksums and a Homebrew formula from the release assets.
5. Optionally publish the formula into a tap repo.

## Validate Installer Metadata

For a local dry run against an existing release tag:

```bash
scripts/validate-release-installers.sh --tag v0.2.0 --tap-repo ../homebrew-tap
```

If Homebrew is installed locally, you can also ask the script to run `brew audit` on the generated formula:

```bash
scripts/validate-release-installers.sh --tag v0.2.0 --tap-repo ../homebrew-tap --check-brew
```
