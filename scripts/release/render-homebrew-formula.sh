#!/usr/bin/env bash

set -euo pipefail

usage() {
  cat <<'EOF'
Usage:
  render-homebrew-formula.sh \
    --version 0.2.0 \
    --repo dudash/mdmind \
    --darwin-amd64-sha <sha256> \
    --darwin-arm64-sha <sha256> \
    [--linux-amd64-sha <sha256>]

Renders a Homebrew formula for mdmind to stdout.
EOF
}

VERSION=""
REPO=""
DARWIN_AMD64_SHA=""
DARWIN_ARM64_SHA=""
LINUX_AMD64_SHA=""

while [[ $# -gt 0 ]]; do
  case "$1" in
    --version)
      VERSION="$2"
      shift 2
      ;;
    --repo)
      REPO="$2"
      shift 2
      ;;
    --darwin-amd64-sha)
      DARWIN_AMD64_SHA="$2"
      shift 2
      ;;
    --darwin-arm64-sha)
      DARWIN_ARM64_SHA="$2"
      shift 2
      ;;
    --linux-amd64-sha)
      LINUX_AMD64_SHA="$2"
      shift 2
      ;;
    -h|--help)
      usage
      exit 0
      ;;
    *)
      echo "Unknown argument: $1" >&2
      usage >&2
      exit 1
      ;;
  esac
done

if [[ -z "${VERSION}" || -z "${REPO}" || -z "${DARWIN_AMD64_SHA}" || -z "${DARWIN_ARM64_SHA}" ]]; then
  usage >&2
  exit 1
fi

TAG_NAME="v${VERSION}"
BASE_URL="https://github.com/${REPO}/releases/download/${TAG_NAME}"

cat <<EOF
class Mdmind < Formula
  desc "Local-first plain-text mind mapping for structured thinking"
  homepage "https://github.com/${REPO}"
  license "Apache-2.0"

  if OS.mac?
    if Hardware::CPU.arm?
      url "${BASE_URL}/mdmind-${TAG_NAME}-aarch64-apple-darwin.tar.gz"
      sha256 "${DARWIN_ARM64_SHA}"
    else
      url "${BASE_URL}/mdmind-${TAG_NAME}-x86_64-apple-darwin.tar.gz"
      sha256 "${DARWIN_AMD64_SHA}"
    end
EOF

if [[ -n "${LINUX_AMD64_SHA}" ]]; then
  cat <<EOF
  elsif OS.linux?
    url "${BASE_URL}/mdmind-${TAG_NAME}-x86_64-unknown-linux-gnu.tar.gz"
    sha256 "${LINUX_AMD64_SHA}"
EOF
fi

cat <<EOF
  end

  def install
    root = Dir["mdmind-*"].find { |path| File.directory?(path) } || "."

    bin.install File.join(root, "mdm")
    bin.install File.join(root, "mdmind")

    readme = File.join(root, "README.md")
    doc.install readme if File.exist?(readme)

    examples_dir = File.join(root, "examples")
    pkgshare.install examples_dir if Dir.exist?(examples_dir)
  end

  test do
    assert_match "mdm #{version}", shell_output("#{bin}/mdm version").strip
  end
end
EOF
