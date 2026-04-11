# Release Installers And Homebrew

## Status

In work.

## What Is Real Now

`mdmind` already ships tagged release archives for:

- macOS Intel
- macOS Apple Silicon
- Linux x86_64
- Windows x86_64

The release flow now also generates:

- a Homebrew formula asset
- a checksum manifest for the release archives
- optional tap publishing when repo settings are configured

## What Is Still Incomplete

- no public tap repo is assumed yet
- Linux and Windows still use direct archive installs by default
- CLI completions and man pages are still separate work

## Product Goal

The installer story should feel clean and boring:

- macOS: Homebrew
- Linux: direct archive first, maybe more later
- Windows: direct archive first, maybe more later

## Why This Matters

The project now has enough surface area that `cargo install --path .` is no longer a sufficient public install story. Release packaging needs to feel intentional and repeatable.
