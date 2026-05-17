#!/usr/bin/env python3
"""Initialize and grade mdmind skill eval workspaces.

The harness is intentionally file-based. Agent runners can be local humans,
Codex, Claude Code, Promptfoo, Skill Bench, or another tool as long as they write
the requested artifacts into each case's outputs directory.
"""

from __future__ import annotations

import argparse
import json
import os
import re
import shutil
import subprocess
import sys
import time
from pathlib import Path
from typing import Any


REPO_ROOT = Path(__file__).resolve().parents[1]
DEFAULT_CASES = REPO_ROOT / "evals" / "skill-workflows" / "cases.json"


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--cases", default=str(DEFAULT_CASES), help="Path to eval case JSON.")
    parser.add_argument(
        "--workspace",
        default=str(REPO_ROOT / "evals" / "skill-workflows" / "workspace" / "iteration-1"),
        help="Iteration workspace directory to initialize or grade.",
    )
    parser.add_argument(
        "--configs",
        default="with_skill,without_skill",
        help="Comma-separated run configurations to initialize or grade.",
    )
    parser.add_argument("--mdm", default=os.environ.get("MDM_BIN"), help="Path to mdm binary.")
    parser.add_argument("--list", action="store_true", help="List eval cases and exit.")
    parser.add_argument("--init", action="store_true", help="Create an empty iteration workspace.")
    parser.add_argument("--grade", action="store_true", help="Grade an existing iteration workspace.")
    parser.add_argument(
        "--allow-failures",
        action="store_true",
        help="Exit 0 even when one or more checks fail.",
    )
    args = parser.parse_args()

    cases_path = Path(args.cases)
    workspace = Path(args.workspace)
    configs = [config.strip() for config in args.configs.split(",") if config.strip()]
    suite = load_suite(cases_path)

    if args.list:
        list_cases(suite)
        return 0

    if not args.init and not args.grade:
        parser.error("choose --list, --init, or --grade")

    if args.init:
        init_workspace(suite, workspace, configs, cases_path)

    if args.grade:
        report = grade_workspace(suite, workspace, configs, args.mdm)
        print_report(report)
        if report["summary"]["failed"] and not args.allow_failures:
            return 1

    return 0


def load_suite(path: Path) -> dict[str, Any]:
    try:
        suite = json.loads(path.read_text())
    except FileNotFoundError:
        raise SystemExit(f"missing eval cases file: {path}") from None
    except json.JSONDecodeError as error:
        raise SystemExit(f"invalid eval cases JSON in {path}: {error}") from None

    if suite.get("version") != "mdmind.skill_evals.v1":
        raise SystemExit(f"unsupported eval suite version in {path}: {suite.get('version')}")
    cases = suite.get("cases")
    if not isinstance(cases, list) or not cases:
        raise SystemExit(f"eval suite must contain a non-empty cases list: {path}")
    for case in cases:
        require(case, "id", str)
        require(case, "title", str)
        require(case, "skill", str)
        require(case, "prompt", str)
        require(case, "checks", list)
    return suite


def require(mapping: dict[str, Any], key: str, expected_type: type) -> None:
    if not isinstance(mapping.get(key), expected_type):
        raise SystemExit(f"case {mapping.get('id', '<unknown>')} needs {key}: {expected_type.__name__}")


def list_cases(suite: dict[str, Any]) -> None:
    for case in suite["cases"]:
        print(f"{case['id']}\t{case['skill']}\t{case['title']}")


def init_workspace(
    suite: dict[str, Any],
    workspace: Path,
    configs: list[str],
    cases_path: Path,
) -> None:
    workspace.mkdir(parents=True, exist_ok=True)
    manifest = {
        "version": "mdmind.skill_eval_workspace.v1",
        "created_at": timestamp(),
        "cases": str(cases_path),
        "configs": configs,
    }
    write_json(workspace / "manifest.json", manifest)

    for case in suite["cases"]:
        case_dir = workspace / case["id"]
        for config in configs:
            run_dir = case_dir / config
            inputs_dir = run_dir / "inputs"
            outputs_dir = run_dir / "outputs"
            inputs_dir.mkdir(parents=True, exist_ok=True)
            outputs_dir.mkdir(parents=True, exist_ok=True)
            write_prompt(run_dir / "prompt.md", case, config)
            write_inputs(inputs_dir, case)

    print(f"Initialized skill eval workspace: {workspace}")
    print("Fill each outputs/ directory, then run:")
    print(f"  {Path(sys.argv[0]).name} --cases {cases_path} --workspace {workspace} --grade")


def write_prompt(path: Path, case: dict[str, Any], config: str) -> None:
    expected = case.get("expected_artifacts", [])
    lines = [
        f"# Eval: {case['title']}",
        "",
        f"- Case: `{case['id']}`",
        f"- Skill under test: `{case['skill']}`",
        f"- Configuration: `{config}`",
        "",
        "## Task",
        "",
        case["prompt"].strip(),
        "",
        "## Output Contract",
        "",
        "Write only the requested artifacts into `outputs/`.",
    ]
    if expected:
        lines.append("")
        for artifact in expected:
            lines.append(f"- `outputs/{artifact}`")
    lines.extend(
        [
            "",
            "Do not edit files outside this eval case directory.",
            "Use the narrowest useful mdm command sequence when CLI inspection is part of the task.",
            "",
        ]
    )
    path.write_text("\n".join(lines))


def write_inputs(inputs_dir: Path, case: dict[str, Any]) -> None:
    for item in case.get("inputs", []):
        target = inputs_dir / item["path"]
        target.parent.mkdir(parents=True, exist_ok=True)
        if "content" in item:
            target.write_text(item["content"])
        elif "source" in item:
            source = (REPO_ROOT / item["source"]).resolve()
            if not source.is_file():
                raise SystemExit(f"input source does not exist: {source}")
            shutil.copyfile(source, target)
        else:
            raise SystemExit(f"input for case {case['id']} needs content or source")


def grade_workspace(
    suite: dict[str, Any],
    workspace: Path,
    configs: list[str],
    mdm_override: str | None,
) -> dict[str, Any]:
    mdm = MdmRunner(mdm_override)
    reports = []
    summary: dict[str, dict[str, int]] = {
        config: {"passed": 0, "failed": 0, "skipped": 0, "checks": 0}
        for config in configs
    }

    for case in suite["cases"]:
        for config in configs:
            run_dir = workspace / case["id"] / config
            outputs_dir = run_dir / "outputs"
            if not outputs_dir.exists():
                result = {
                    "case_id": case["id"],
                    "config": config,
                    "status": "skipped",
                    "checks": [],
                    "message": f"missing outputs directory: {outputs_dir}",
                }
                reports.append(result)
                summary[config]["skipped"] += 1
                continue

            check_results = [run_check(check, run_dir, mdm) for check in case["checks"]]
            failed = [check for check in check_results if not check["pass"]]
            passed = len(check_results) - len(failed)
            result = {
                "case_id": case["id"],
                "title": case["title"],
                "skill": case["skill"],
                "config": config,
                "status": "failed" if failed else "passed",
                "passed": passed,
                "failed": len(failed),
                "checks": check_results,
            }
            write_json(run_dir / "grading.json", result)
            reports.append(result)
            summary[config]["passed"] += passed
            summary[config]["failed"] += len(failed)
            summary[config]["checks"] += len(check_results)

    total_failed = sum(config["failed"] for config in summary.values())
    total_passed = sum(config["passed"] for config in summary.values())
    report = {
        "version": "mdmind.skill_eval_report.v1",
        "generated_at": timestamp(),
        "workspace": str(workspace),
        "summary": {
            "passed": total_passed,
            "failed": total_failed,
            "checks": total_passed + total_failed,
            "configs": summarize_configs(summary),
        },
        "cases": reports,
    }
    write_json(workspace / "benchmark.json", report)
    return report


def summarize_configs(summary: dict[str, dict[str, int]]) -> dict[str, dict[str, Any]]:
    summarized: dict[str, dict[str, Any]] = {}
    for config, counts in summary.items():
        checks = counts["checks"]
        pass_rate = None if checks == 0 else counts["passed"] / checks
        summarized[config] = {**counts, "pass_rate": pass_rate}
    return summarized


def run_check(check: dict[str, Any], run_dir: Path, mdm: "MdmRunner") -> dict[str, Any]:
    check_type = check.get("type")
    try:
        if check_type == "file_exists":
            return check_file_exists(check, run_dir)
        if check_type == "contains":
            return check_contains(check, run_dir)
        if check_type == "json_valid":
            return check_json_valid(check, run_dir)
        if check_type == "mdm_validate":
            return check_mdm_validate(check, run_dir, mdm)
        if check_type == "mdm_find":
            return check_mdm_find(check, run_dir, mdm)
        if check_type == "mdm_links":
            return check_mdm_links(check, run_dir, mdm)
        if check_type == "max_label_chars":
            return check_max_label_chars(check, run_dir)
    except Exception as error:  # noqa: BLE001 - eval reports should capture any check failure.
        return fail(check_type, f"{error}")

    return fail(check_type, f"unknown check type: {check_type}")


def check_file_exists(check: dict[str, Any], run_dir: Path) -> dict[str, Any]:
    path = checked_path(run_dir, check["path"])
    return result(check, path.exists(), f"{path} exists", f"missing {path}")


def check_contains(check: dict[str, Any], run_dir: Path) -> dict[str, Any]:
    path = checked_path(run_dir, check["path"])
    text = path.read_text()
    missing = [needle for needle in check.get("all", []) if needle not in text]
    any_values = check.get("any", [])
    absent_hits = [needle for needle in check.get("none", []) if needle in text]
    any_failed = bool(any_values) and not any(needle in text for needle in any_values)
    passed = not missing and not absent_hits and not any_failed
    details = {
        "missing": missing,
        "forbidden": absent_hits,
        "any": any_values,
    }
    message = "text expectations satisfied" if passed else "text expectations failed"
    return with_details(check, passed, message, details)


def check_json_valid(check: dict[str, Any], run_dir: Path) -> dict[str, Any]:
    path = checked_path(run_dir, check["path"])
    value = json.loads(path.read_text())
    required_top_level = check.get("required_top_level", [])
    missing = [key for key in required_top_level if key not in value]
    return with_details(
        check,
        not missing,
        "valid JSON" if not missing else "valid JSON missing required keys",
        {"missing": missing},
    )


def check_mdm_validate(check: dict[str, Any], run_dir: Path, mdm: "MdmRunner") -> dict[str, Any]:
    path = checked_path(run_dir, check["path"])
    completed = mdm.run(["validate", str(path), "--json"])
    parsed = parse_json_stdout(completed)
    passed = completed.returncode == 0 and parsed.get("ok") is True
    details = command_details(completed, parsed)
    return with_details(check, passed, "map validates" if passed else "map validation failed", details)


def check_mdm_find(check: dict[str, Any], run_dir: Path, mdm: "MdmRunner") -> dict[str, Any]:
    path = checked_path(run_dir, check["path"])
    query = check["query"]
    minimum = int(check.get("min_matches", 1))
    completed = mdm.run(["find", str(path), query, "--json"])
    parsed = parse_json_stdout(completed)
    count = int(parsed.get("summary", {}).get("count", len(parsed.get("data", []))))
    passed = completed.returncode == 0 and parsed.get("ok") is True and count >= minimum
    details = {**command_details(completed, parsed), "count": count, "min_matches": minimum}
    return with_details(check, passed, f"{count} matches for {query!r}", details)


def check_mdm_links(check: dict[str, Any], run_dir: Path, mdm: "MdmRunner") -> dict[str, Any]:
    path = checked_path(run_dir, check["path"])
    minimum = int(check.get("min_links", 1))
    required_ids = check.get("required_ids", [])
    completed = mdm.run(["links", str(path), "--json"])
    parsed = parse_json_stdout(completed)
    rows = parsed.get("data", [])
    ids = {row.get("id") for row in rows if isinstance(row, dict)}
    missing_ids = [item for item in required_ids if item not in ids]
    passed = (
        completed.returncode == 0
        and parsed.get("ok") is True
        and len(rows) >= minimum
        and not missing_ids
    )
    details = {
        **command_details(completed, parsed),
        "count": len(rows),
        "min_links": minimum,
        "missing_ids": missing_ids,
    }
    return with_details(check, passed, "link expectations satisfied", details)


def check_max_label_chars(check: dict[str, Any], run_dir: Path) -> dict[str, Any]:
    path = checked_path(run_dir, check["path"])
    maximum = int(check["max"])
    too_long = [
        {"line": line_number, "label": label, "chars": len(label)}
        for line_number, label in node_labels(path)
        if len(label) > maximum
    ]
    return with_details(
        check,
        not too_long,
        "node labels fit length budget" if not too_long else "node labels exceed length budget",
        {"max": maximum, "too_long": too_long},
    )


def checked_path(run_dir: Path, relative_path: str) -> Path:
    path = (run_dir / relative_path).resolve()
    if run_dir.resolve() not in path.parents and path != run_dir.resolve():
        raise ValueError(f"path escapes run directory: {relative_path}")
    return path


def node_labels(path: Path) -> list[tuple[int, str]]:
    labels = []
    for line_number, line in enumerate(path.read_text().splitlines(), start=1):
        stripped = line.strip()
        if not stripped.startswith("- "):
            continue
        label = stripped[2:].strip()
        label = re.sub(r"^\[[ xX]\]\s+", "", label)
        label = re.sub(r"\[\[[^\]]+\]\]", "", label)
        label = re.sub(r"\[[^\]]+\]", "", label)
        label = re.sub(r"#[^\s]+", "", label)
        label = re.sub(r"@[^\s]+", "", label)
        label = " ".join(label.split())
        labels.append((line_number, label))
    return labels


class MdmRunner:
    def __init__(self, override: str | None) -> None:
        if override:
            self.command = [override]
            self.cwd = None
        elif (REPO_ROOT / "target" / "debug" / "mdm").exists():
            self.command = [str(REPO_ROOT / "target" / "debug" / "mdm")]
            self.cwd = None
        elif shutil.which("mdm"):
            self.command = [shutil.which("mdm") or "mdm"]
            self.cwd = None
        else:
            self.command = ["cargo", "run", "--quiet", "--bin", "mdm", "--"]
            self.cwd = REPO_ROOT

    def run(self, args: list[str]) -> subprocess.CompletedProcess[str]:
        return subprocess.run(
            [*self.command, *args],
            cwd=self.cwd,
            text=True,
            stdout=subprocess.PIPE,
            stderr=subprocess.PIPE,
            check=False,
        )


def parse_json_stdout(completed: subprocess.CompletedProcess[str]) -> dict[str, Any]:
    try:
        return json.loads(completed.stdout)
    except json.JSONDecodeError:
        return {}


def command_details(completed: subprocess.CompletedProcess[str], parsed: dict[str, Any]) -> dict[str, Any]:
    details: dict[str, Any] = {
        "returncode": completed.returncode,
        "stderr": completed.stderr.strip(),
    }
    if parsed:
        details["ok"] = parsed.get("ok")
        details["format"] = parsed.get("format")
        details["summary"] = parsed.get("summary")
        details["error"] = parsed.get("error")
    else:
        details["stdout"] = completed.stdout[:1000]
    return details


def result(check: dict[str, Any], passed: bool, pass_message: str, fail_message: str) -> dict[str, Any]:
    return with_details(check, passed, pass_message if passed else fail_message, {})


def with_details(
    check: dict[str, Any],
    passed: bool,
    message: str,
    details: dict[str, Any],
) -> dict[str, Any]:
    return {
        "type": check.get("type"),
        "pass": passed,
        "message": message,
        "details": details,
    }


def fail(check_type: str | None, message: str) -> dict[str, Any]:
    return {
        "type": check_type,
        "pass": False,
        "message": message,
        "details": {},
    }


def print_report(report: dict[str, Any]) -> None:
    summary = report["summary"]
    print(f"Skill eval checks: {summary['passed']} passed, {summary['failed']} failed")
    for config, counts in summary["configs"].items():
        rate = counts["pass_rate"]
        rate_text = "n/a" if rate is None else f"{rate:.0%}"
        print(
            f"- {config}: {counts['passed']} passed, {counts['failed']} failed, "
            f"{counts['skipped']} skipped, pass rate {rate_text}"
        )
    for case in report["cases"]:
        if case["status"] != "failed":
            continue
        print(f"\nFAILED {case['case_id']} [{case['config']}]:")
        for check in case["checks"]:
            if not check["pass"]:
                print(f"  - {check['type']}: {check['message']}")


def write_json(path: Path, value: Any) -> None:
    path.parent.mkdir(parents=True, exist_ok=True)
    path.write_text(json.dumps(value, indent=2, sort_keys=True) + "\n")


def timestamp() -> str:
    return time.strftime("%Y-%m-%dT%H:%M:%SZ", time.gmtime())


if __name__ == "__main__":
    raise SystemExit(main())
