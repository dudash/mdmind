# Release Installers And Homebrew

The public install story is now real.

What ships today:

- tagged release archives for macOS Intel, macOS Apple Silicon, Linux x86_64, and Windows x86_64
- generated checksum manifest alongside release artifacts
- generated Homebrew formula
- Homebrew tap support for macOS installs
- bundled `mdm`, `mdmind`, and example maps in release packaging

Why it matters:

- public installs no longer depend on cloning the repo or using `cargo install --path .`
- macOS has a normal installer path
- release artifacts and the tap formula can be validated repeatably

Related docs:

- [INSTALL_AND_RELEASE.md](/Users/jason/Documents/Programming/mdmind/docs/INSTALL_AND_RELEASE.md)
- [DEVELOPER.md](/Users/jason/Documents/Programming/mdmind/DEVELOPER.md)
