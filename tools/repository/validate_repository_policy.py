#!/usr/bin/env python3
"""Validate repository-level GitHub governance without external dependencies."""

from __future__ import annotations

import hashlib
import json
import re
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]
POLICY_PATH = ROOT / ".github/repository-policy.json"
USES_LINE = re.compile(r"^\s*uses:\s*([^@\s]+)@([^\s#]+)", re.MULTILINE)
CANONICAL_MPL_2_0_GIT_BLOB_SHA = "d0a1fa1482eea82e19510e7920cbe3a03e41f691"

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
    "LICENSE",
    "LICENSE-ASSETS.md",
    "TRADEMARKS.md",
    ".editorconfig",
    ".gitignore",
    "docs/repository/GITHUB_GOVERNANCE.md",
    "docs/repository/LICENSING.md",
]


def git_blob_sha(data: bytes) -> str:
    header = f"blob {len(data)}\0".encode("ascii")
    return hashlib.sha1(header + data).hexdigest()


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

    licensing = policy.get("licensing", {})
    expected_licensing = {
        "spdx_id": "MPL-2.0",
        "license_file": "LICENSE",
        "scope_policy": "docs/repository/LICENSING.md",
        "reserved_assets_notice": "LICENSE-ASSETS.md",
        "trademark_notice": "TRADEMARKS.md",
        "incompatible_with_secondary_licenses": False,
    }
    if licensing != expected_licensing:
        errors.append("repository licensing policy does not match the canonical MPL-2.0 boundary")

    license_path = ROOT / "LICENSE"
    if license_path.is_file():
        license_bytes = license_path.read_bytes()
        license_text = license_bytes.decode("utf-8")
        if git_blob_sha(license_bytes) != CANONICAL_MPL_2_0_GIT_BLOB_SHA:
            errors.append("LICENSE does not match the pinned canonical MPL-2.0 text")
        required_license_fragments = (
            "Mozilla Public License Version 2.0",
            "2. License Grants and Conditions",
            "3. Responsibilities",
            "10. Versions of the License",
            "Exhibit A - Source Code Form License Notice",
            'Exhibit B - "Incompatible With Secondary Licenses" Notice',
        )
        for fragment in required_license_fragments:
            if fragment not in license_text:
                errors.append(f"LICENSE is missing canonical MPL-2.0 fragment: {fragment}")
        if len(license_text.splitlines()) < 330:
            errors.append("LICENSE is unexpectedly short for the canonical MPL-2.0 text")

    licensing_policy_path = ROOT / "docs/repository/LICENSING.md"
    if licensing_policy_path.is_file():
        licensing_text = licensing_policy_path.read_text(encoding="utf-8")
        for fragment in (
            "MPL-2.0",
            "LICENSE-ASSETS.md",
            "TRADEMARKS.md",
            "does not attach or apply the separate Exhibit B incompatibility notice",
            "does not currently require copyright assignment or a Contributor License Agreement",
        ):
            if fragment not in licensing_text:
                errors.append(f"licensing policy missing required boundary: {fragment}")

    assets_path = ROOT / "LICENSE-ASSETS.md"
    if assets_path.is_file() and "applies repository-wide" not in assets_path.read_text(encoding="utf-8"):
        errors.append("creative asset reservation must explicitly apply repository-wide")

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

    for path, fragments in {
        "README.md": ("Mozilla Public License 2.0", "LICENSE-ASSETS.md", "TRADEMARKS.md"),
        "CONTRIBUTING.md": ("MPL-2.0", "LICENSE-ASSETS.md", "TRADEMARKS.md"),
    }.items():
        file_path = ROOT / path
        if file_path.is_file():
            text = file_path.read_text(encoding="utf-8")
            for fragment in fragments:
                if fragment not in text:
                    errors.append(f"{path} missing licensing reference: {fragment}")

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
