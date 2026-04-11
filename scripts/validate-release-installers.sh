#!/usr/bin/env bash

set -euo pipefail

usage() {
  cat <<'EOF'
Usage:
  validate-release-installers.sh --tag v0.2.0 [--repo dudash/mdmind] [--tap-repo ../homebrew-tap] [--check-brew]

Downloads the release archives for a tag, rebuilds the checksum manifest and Homebrew
formula exactly like the release workflow, and verifies the generated formula against
the tap repo when one is provided.
EOF
}

TAG_NAME=""
REPO="dudash/mdmind"
TAP_REPO=""
CHECK_BREW="0"

while [[ $# -gt 0 ]]; do
  case "$1" in
    --tag)
      TAG_NAME="$2"
      shift 2
      ;;
    --repo)
      REPO="$2"
      shift 2
      ;;
    --tap-repo)
      TAP_REPO="$2"
      shift 2
      ;;
    --check-brew)
      CHECK_BREW="1"
      shift
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

if [[ -z "${TAG_NAME}" ]]; then
  usage >&2
  exit 1
fi

VERSION="${TAG_NAME#v}"
repo_root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
if [[ -n "${TAP_REPO}" ]]; then
  TAP_REPO="$(cd "${TAP_REPO}" && pwd)"
fi
WORKDIR="$(mktemp -d "${TMPDIR:-/tmp}/mdmind-release-validate.XXXXXX")"
trap 'rm -rf "${WORKDIR}"' EXIT

cd "${WORKDIR}"

for target in x86_64-apple-darwin aarch64-apple-darwin x86_64-unknown-linux-gnu x86_64-pc-windows-msvc; do
  extension="tar.gz"
  if [[ "${target}" == "x86_64-pc-windows-msvc" ]]; then
    extension="zip"
  fi
  artifact="mdmind-${TAG_NAME}-${target}.${extension}"
  curl -fsSL -o "${artifact}" "https://github.com/${REPO}/releases/download/${TAG_NAME}/${artifact}"
done

shasum -a 256 \
  "mdmind-${TAG_NAME}-x86_64-unknown-linux-gnu.tar.gz" \
  "mdmind-${TAG_NAME}-x86_64-apple-darwin.tar.gz" \
  "mdmind-${TAG_NAME}-aarch64-apple-darwin.tar.gz" \
  "mdmind-${TAG_NAME}-x86_64-pc-windows-msvc.zip" \
  > "mdmind-${TAG_NAME}-checksums.txt"

darwin_amd64_sha="$(awk '/x86_64-apple-darwin/ {print $1}' "mdmind-${TAG_NAME}-checksums.txt")"
darwin_arm64_sha="$(awk '/aarch64-apple-darwin/ {print $1}' "mdmind-${TAG_NAME}-checksums.txt")"
linux_amd64_sha="$(awk '/x86_64-unknown-linux-gnu/ {print $1}' "mdmind-${TAG_NAME}-checksums.txt")"

bash "${repo_root}/scripts/render-homebrew-formula.sh" \
  --version "${VERSION}" \
  --repo "${REPO}" \
  --darwin-amd64-sha "${darwin_amd64_sha}" \
  --darwin-arm64-sha "${darwin_arm64_sha}" \
  --linux-amd64-sha "${linux_amd64_sha}" \
  > mdmind.rb

echo "Validated release assets for ${TAG_NAME}"
echo "Checksum manifest: ${WORKDIR}/mdmind-${TAG_NAME}-checksums.txt"
echo "Generated formula: ${WORKDIR}/mdmind.rb"

if [[ -n "${TAP_REPO}" ]]; then
  tap_formula="${TAP_REPO}/Formula/mdmind.rb"
  if [[ ! -f "${tap_formula}" ]]; then
    echo "Tap formula not found at ${tap_formula}" >&2
    exit 1
  fi
  diff -u "${tap_formula}" mdmind.rb
  echo "Tap formula matches generated formula"
fi

if [[ "${CHECK_BREW}" == "1" ]]; then
  if ! command -v brew >/dev/null 2>&1; then
    echo "brew not found in PATH" >&2
    exit 1
  fi
  if [[ -z "${TAP_REPO}" ]]; then
    echo "--check-brew requires --tap-repo so brew can audit the tapped formula" >&2
    exit 1
  fi
  tap_name="local/mdmind-validate"
  if brew tap | grep -qx "${tap_name}"; then
    brew untap "${tap_name}"
  fi
  brew tap "${tap_name}" "${TAP_REPO}"
  brew audit --strict "${tap_name}/mdmind"
  brew untap "${tap_name}"
  echo "brew audit passed"
fi
