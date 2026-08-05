#!/usr/bin/env python3
"""Validate repository-level GitHub governance without external dependencies."""

from __future__ import annotations

import json
import re
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]
POLICY_PATH = ROOT / ".github/repository-policy.json"
USES_LINE = re.compile(r"^\s*uses:\s*([^@\s]+)@([^\s#]+)", re.MULTILINE)

REQUIRED_FILES = [
    ".github/CODEOWNERS",
    ".github/pull_request_template.md",
    ".github/ISSUE_TEMPLATE/bug_report.yml",
    ".github/ISSUE_TEMPLATE/feature_request.yml",
    ".github/ISSUE_TEMPLATE/config.yml",
    ".github/dependabot.yml",
    ".github/repository-policy.json",
    ".github/workflows/agent-governance.yml",
    ".github/workflows/codeql.yml",
    ".github/workflows/dependency-review.yml",
    ".github/workflows/repository-configuration.yml",
    "CONTRIBUTING.md",
    "SECURITY.md",
    ".editorconfig",
    ".gitignore",
    "docs/repository/GITHUB_GOVERNANCE.md",
]


def main() -> int:
    errors: list[str] = []

    for relative in REQUIRED_FILES:
        if not (ROOT / relative).is_file():
            errors.append(f"missing required repository-governance file: {relative}")

    try:
        policy = json.loads(POLICY_PATH.read_text(encoding="utf-8"))
    except (FileNotFoundError, json.JSONDecodeError) as exc:
        errors.append(f"invalid repository policy: {exc}")
        policy = {}

    if policy.get("schema_version") != "1.0":
        errors.append("repository policy schema_version must be 1.0")
    if policy.get("required_status_check") != "Agent governance / validate":
        errors.append("unexpected required status check")

    repo = policy.get("repository", {})
    expected_repo = {
        "allow_auto_merge": True,
        "allow_squash_merge": True,
        "allow_merge_commit": False,
        "allow_rebase_merge": False,
        "delete_branch_on_merge": True,
        "use_squash_pr_title_as_default": True,
        "squash_merge_commit_title": "PR_TITLE",
        "squash_merge_commit_message": "PR_BODY",
        "has_wiki": False,
    }
    for key, expected in expected_repo.items():
        if repo.get(key) != expected:
            errors.append(f"repository policy {key} must be {expected!r}")

    topics = policy.get("topics", [])
    if not isinstance(topics, list) or not topics or len(topics) != len(set(topics)):
        errors.append("repository topics must be a non-empty unique list")
    labels = policy.get("labels", [])
    label_names = [label.get("name") for label in labels if isinstance(label, dict)]
    if len(label_names) != len(labels) or len(label_names) != len(set(label_names)):
        errors.append("repository labels must have unique names")
    for required_label in ("dependencies", "ci", "security", "architecture"):
        if required_label not in label_names:
            errors.append(f"repository policy missing label: {required_label}")

    ruleset = policy.get("ruleset", {})
    if ruleset.get("enforcement") != "active":
        errors.append("main ruleset must be active")
    if ruleset.get("bypass_actors") != []:
        errors.append("main ruleset must not define routine bypass actors")
    rule_types = {
        rule.get("type")
        for rule in ruleset.get("rules", [])
        if isinstance(rule, dict)
    }
    required_rule_types = {
        "deletion",
        "required_linear_history",
        "pull_request",
        "required_status_checks",
        "non_fast_forward",
    }
    missing_rules = sorted(required_rule_types - rule_types)
    if missing_rules:
        errors.append(f"main ruleset missing rules: {', '.join(missing_rules)}")
    if "required_signatures" in rule_types:
        errors.append("strict signed commits are incompatible with third-party squash PRs")

    workflow_dir = ROOT / ".github/workflows"
    if workflow_dir.is_dir():
        for workflow in sorted(workflow_dir.glob("*.y*ml")):
            text = workflow.read_text(encoding="utf-8")
            for action, ref in USES_LINE.findall(text):
                if re.fullmatch(r"[0-9a-f]{40}", ref) is None:
                    errors.append(
                        f"{workflow.relative_to(ROOT)} uses unpinned action {action}@{ref}"
                    )
            if "pull_request_target:" in text and "actions/checkout" in text:
                errors.append(
                    f"{workflow.relative_to(ROOT)} combines pull_request_target with checkout"
                )

    template = ROOT / ".github/pull_request_template.md"
    if template.is_file():
        text = template.read_text(encoding="utf-8")
        for heading in ("## Summary", "## Scope", "## Validation"):
            if heading not in text:
                errors.append(f"pull request template missing {heading}")

    if errors:
        print("Repository policy validation failed:", file=sys.stderr)
        for error in errors:
            print(f"- {error}", file=sys.stderr)
        return 1

    workflows = len(list((ROOT / ".github/workflows").glob("*.y*ml")))
    print(f"Repository policy validation passed ({len(REQUIRED_FILES)} files, {workflows} workflows).")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
