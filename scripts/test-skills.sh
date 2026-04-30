#!/usr/bin/env bash

set -euo pipefail

usage() {
  cat <<'EOF'
Usage:
  test-skills.sh [--skill mdmind-map-authoring|mdm-cli-inspection] [--dest /tmp/codex-home] [--codex-cmd codex]

Creates an isolated Codex skills home containing the selected mdmind skill package
and prints the exact commands to test it manually.

Examples:
  test-skills.sh --skill mdmind-map-authoring
  test-skills.sh --skill mdm-cli-inspection
  test-skills.sh --dest /tmp/mdmind-skill-test
EOF
}

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REPO_ROOT="$(cd "${SCRIPT_DIR}/.." && pwd)"
SKILLS_ROOT="${REPO_ROOT}/skills"

DEST=""
CODEX_CMD="codex"
SELECTED_SKILL=""

available_skills=(
  "mdmind-map-authoring"
  "mdm-cli-inspection"
)

while [[ $# -gt 0 ]]; do
  case "$1" in
    --skill)
      SELECTED_SKILL="$2"
      shift 2
      ;;
    --dest)
      DEST="$2"
      shift 2
      ;;
    --codex-cmd)
      CODEX_CMD="$2"
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

is_known_skill() {
  local candidate="$1"
  local skill
  for skill in "${available_skills[@]}"; do
    if [[ "${skill}" == "${candidate}" ]]; then
      return 0
    fi
  done
  return 1
}

if [[ -n "${SELECTED_SKILL}" ]] && ! is_known_skill "${SELECTED_SKILL}"; then
  echo "Unknown skill: ${SELECTED_SKILL}" >&2
  usage >&2
  exit 1
fi

if [[ -z "${DEST}" ]]; then
  suffix="all"
  if [[ -n "${SELECTED_SKILL}" ]]; then
    suffix="${SELECTED_SKILL}"
  fi
  DEST="$(mktemp -d "${TMPDIR:-/tmp}/mdmind-skill-test-${suffix}.XXXXXX")"
else
  mkdir -p "${DEST}"
  if [[ -n "$(find "${DEST}" -mindepth 1 -maxdepth 1 -print -quit 2>/dev/null)" ]]; then
    echo "Destination must be empty for an isolated test: ${DEST}" >&2
    exit 1
  fi
fi

mkdir -p "${DEST}/skills"

copy_skill() {
  local skill_name="$1"
  local source_dir="${SKILLS_ROOT}/${skill_name}"
  local target_dir="${DEST}/skills/${skill_name}"

  if [[ ! -f "${source_dir}/SKILL.md" ]]; then
    echo "Missing SKILL.md for ${skill_name}: ${source_dir}/SKILL.md" >&2
    exit 1
  fi

  cp -R "${source_dir}" "${target_dir}"
}

selected_skills=()
if [[ -n "${SELECTED_SKILL}" ]]; then
  selected_skills+=("${SELECTED_SKILL}")
else
  selected_skills=("${available_skills[@]}")
fi

for skill in "${selected_skills[@]}"; do
  copy_skill "${skill}"
done

validator_cmd="mdm validate /path/to/output.md"
if ! command -v mdm >/dev/null 2>&1; then
  validator_cmd="cargo run --bin mdm -- validate /path/to/output.md"
fi

echo "Created isolated Codex skill home:"
echo "  ${DEST}"
echo
echo "Installed skills:"
for skill in "${selected_skills[@]}"; do
  echo "  - ${skill}"
done
echo
echo "Start a clean Codex session with:"
echo "  CODEX_HOME=\"${DEST}\" ${CODEX_CMD}"
echo
echo "Suggested test flow:"
for skill in "${selected_skills[@]}"; do
  prompt_file="${DEST}/skills/${skill}/examples/prompts.md"
  echo
  echo "Skill: ${skill}"
  echo "  Prompt file: ${prompt_file}"
  if [[ "${skill}" == "mdmind-map-authoring" ]]; then
    echo "  After generating a map, validate it with:"
    echo "    ${validator_cmd}"
  else
    echo "  Use one of the prompts against an existing map file and verify the agent chooses the narrowest correct mdm command."
  fi
done

echo
echo "For trigger isolation, test one skill at a time with:"
echo "  ${SCRIPT_DIR}/test-skills.sh --skill mdmind-map-authoring"
echo "  ${SCRIPT_DIR}/test-skills.sh --skill mdm-cli-inspection"
