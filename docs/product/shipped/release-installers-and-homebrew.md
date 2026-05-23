# Release Installers And Homebrew

The public install story is now real.

What ships today:

- tagged release archives for macOS Intel, macOS Apple Silicon, Linux x86_64, and Windows x86_64
- generated checksum manifest alongside release artifacts
- generated Homebrew formula
- curated changelog body for GitHub releases
- Homebrew tap support for macOS installs
- bundled `mdm`, `mdmind`, and example maps in release packaging

Why it matters:

- public installs no longer depend on cloning the repo or using `cargo install --path .`
- macOS has a normal installer path
- release artifacts and the tap formula can be validated repeatably
- release notes can be checked locally with `mdm changelog --version X.Y.Z`

Related docs:

- [CHANGELOG.md](../../../CHANGELOG.md)
- [INSTALL_AND_RELEASE.md](../../manual/INSTALL_AND_RELEASE.md)
- [DEVELOPER.md](../../../DEVELOPER.md)
