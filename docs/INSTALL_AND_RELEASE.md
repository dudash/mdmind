# Install And Release

This is the current install and release story for `mdmind`.

## What Ships Today

Release builds already produce platform archives for:

- macOS Apple Silicon
- macOS Intel
- Linux x86_64
- Windows x86_64

Those archives are attached to each GitHub release.

## Installer Strategy

`mdmind` is still a plain binary-first tool. The release flow should stay simple:

- GitHub Releases are the source of truth
- install metadata is generated from release artifacts
- package-manager automation sits on top of those artifacts

That keeps the process predictable and avoids a second packaging build pipeline.

## Homebrew

For macOS, the preferred installer path is a Homebrew tap.

The release workflow now generates:

- `mdmind.rb`: a Homebrew formula built from the current release tarballs
- `mdmind-vX.Y.Z-checksums.txt`: SHA256 checksums for the published archives

If a tap repo is configured, the release workflow can also push the generated formula to that tap automatically.

Expected tap shape:

- repository: `dudash/homebrew-tap`
- formula path: `Formula/mdmind.rb`
- install command:

```bash
brew tap dudash/tap
brew install mdmind
```

That command is the intended public install path for macOS once the formula is present in the tap.

## Other Operating Systems

Current recommendation:

- Linux: install from the release tarball
- Windows: install from the release zip

Those are simple and already supported by the release pipeline. If later adoption justifies it, Linux packaging can grow into Homebrew-on-Linux, distro packages, or similar. Windows can grow into Scoop or Winget later.

## Release Flow

Current release flow:

1. Prepare or tag a release.
2. Build platform archives.
3. Attach archives to the GitHub release.
4. Generate checksums and a Homebrew formula from the release assets.
5. Optionally publish the formula into a tap repo.

For a local dry run against an existing release tag:

```bash
scripts/validate-release-installers.sh --tag v0.2.0 --tap-repo ../homebrew-tap
```

If Homebrew is installed locally, you can also ask the script to run `brew audit` on the generated formula:

```bash
scripts/validate-release-installers.sh --tag v0.2.0 --tap-repo ../homebrew-tap --check-brew
```

## Tap Automation

Optional GitHub configuration:

- repository variable: `HOMEBREW_TAP_REPO`
- secret: `HOMEBREW_TAP_TOKEN`

Recommended values:

- `HOMEBREW_TAP_REPO=dudash/homebrew-tap`
- `HOMEBREW_TAP_TOKEN=<PAT with repo write access to dudash/homebrew-tap>`

If both are present, the release workflow will:

- clone the tap repo
- write `Formula/mdmind.rb`
- commit the updated formula
- push it back to the tap repo

If they are not present, the workflow still uploads the generated formula and checksum manifest to the GitHub release so the tap can be updated manually.

## Why This Shape

This keeps the packaging story aligned with the product:

- local-first binaries
- one release source of truth
- simple operator workflow
- room to add more installers later without changing how builds are produced
