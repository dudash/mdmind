#!/usr/bin/env bash
set -euo pipefail

script_dir="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
repo_root="$(cd "${script_dir}/.." && pwd)"
plugin_skills="${repo_root}/plugins/mdmind/skills"

skills=(
  "mdmind-map-authoring"
  "mdm-cli-inspection"
)

mkdir -p "${plugin_skills}"

for skill in "${skills[@]}"; do
  source_dir="${repo_root}/skills/${skill}"
  target_dir="${plugin_skills}/${skill}"

  if [[ ! -d "${source_dir}" ]]; then
    echo "Missing canonical skill: ${source_dir}" >&2
    exit 1
  fi

  rm -rf "${target_dir}"
  cp -R "${source_dir}" "${target_dir}"
done

echo "Synced ${#skills[@]} mdmind skills into plugins/mdmind/skills"
