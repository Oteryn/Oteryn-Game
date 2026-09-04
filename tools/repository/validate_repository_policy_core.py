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
EXPECTED_REQUIRED_STATUS = "game-gate"
EXPECTED_CONTROL_PLANE_PATHS = [
    ".github/workflows/*",
    ".github/workflows/**/*",
    ".github/repository-policy.json",
    "tools/repository/*",
    "tools/repository/**/*",
]
EXPECTED_MERGE_GATE_TOP_LEVEL_KEYS = [
    "name",
    "run-name",
    "on",
    "permissions",
    "concurrency",
    "jobs",
]
EXPECTED_MERGE_GATE_TRIGGER_BLOCK = """on:
  pull_request:
    branches:
      - main
    types:
      - opened
      - reopened
      - synchronize
      - edited
"""
EXPECTED_MERGE_GATE_SCOPE_JOB_SHA256 = (
    "ae1234b97375c5a86acc1554b19e8f92c9e4b0979d16539924a73c11fe97312e"
)
EXPECTED_MERGE_GATE_VALIDATE_JOB_SHA256 = (
    "c10c941048014cfc8712b0d02eee438a3dabaf6578c212e4c861d36a02d4f11a"
)
EXPECTED_MERGE_GROUP_GATE_TOP_LEVEL_KEYS = [
    "name",
    "on",
    "permissions",
    "concurrency",
    "jobs",
]
EXPECTED_MERGE_GROUP_GATE_TRIGGER_BLOCK = """on:
  merge_group:
    types: [checks_requested]
"""
EXPECTED_MERGE_GROUP_JOB_KEYS = [
    "candidate",
    "dependency_review",
    "codeql",
    "rust_linux",
    "rust_windows",
    "rust_supply_chain",
    "game_gate",
]

REQUIRED_FILES = [
    ".github/CODEOWNERS",
    ".github/pull_request_template.md",
    ".github/ISSUE_TEMPLATE/bug_report.yml",
    ".github/ISSUE_TEMPLATE/feature_request.yml",
    ".github/ISSUE_TEMPLATE/config.yml",
    ".github/dependabot.yml",
    ".github/repository-policy.json",
    ".github/workflows/merge-gate.yml",
    ".github/workflows/merge-group-gate.yml",
    ".github/workflows/agent-governance.yml",
    ".github/workflows/codeql.yml",
    ".github/workflows/repository-configuration.yml",
    ".github/workflows/rust.yml",
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


def required_status_contexts(ruleset: dict) -> list[str]:
    for rule in ruleset.get("rules", []):
        if isinstance(rule, dict) and rule.get("type") == "required_status_checks":
            checks = rule.get("parameters", {}).get("required_status_checks", [])
            if not isinstance(checks, list):
                return []
            return [
                check.get("context")
                for check in checks
                if isinstance(check, dict) and isinstance(check.get("context"), str)
            ]
    return []


def restricted_file_paths(ruleset: dict) -> list[str]:
    for rule in ruleset.get("rules", []):
        if isinstance(rule, dict) and rule.get("type") == "file_path_restriction":
            paths = rule.get("parameters", {}).get("restricted_file_paths", [])
            if isinstance(paths, list) and all(isinstance(path, str) for path in paths):
                return paths
            return []
    return []


def canonical_top_level_yaml_keys(text: str) -> list[str] | None:
    keys: list[str] = []
    key_line = re.compile(r"^([a-z][a-z0-9-]*):(?:[ \t].*)?$")
    for line in text.replace("\r\n", "\n").splitlines():
        if not line.strip() or line.lstrip().startswith("#"):
            continue
        if line[0].isspace():
            continue
        match = key_line.fullmatch(line)
        if match is None:
            return None
        keys.append(match.group(1))
    return keys


def yaml_mapping_keys_at_indent(text: str, indent: int) -> list[str] | None:
    keys: list[str] = []
    prefix = " " * indent
    key_line = re.compile(rf"^{re.escape(prefix)}([a-z][a-z0-9_-]*):(?:[ \t].*)?$")
    for line in text.replace("\r\n", "\n").splitlines():
        if not line.strip() or line.lstrip().startswith("#"):
            continue
        leading_spaces = len(line) - len(line.lstrip(" "))
        if leading_spaces != indent:
            continue
        match = key_line.fullmatch(line)
        if match is None:
            return None
        keys.append(match.group(1))
    return keys


def indented_yaml_mapping_block(text: str, key: str, indent: int) -> str | None:
    lines = text.splitlines(keepends=True)
    prefix = " " * indent
    key_line = re.compile(rf"^{re.escape(prefix + key)}:\s*(?:#.*)?(?:\r?\n)?$")
    starts = [index for index, line in enumerate(lines) if key_line.fullmatch(line)]
    if len(starts) != 1:
        return None
    start = starts[0]
    end = len(lines)
    for index in range(start + 1, len(lines)):
        line = lines[index]
        if not line.strip() or line.lstrip().startswith("#"):
            continue
        leading_spaces = len(line) - len(line.lstrip(" "))
        if leading_spaces <= indent:
            end = index
            break
    return "".join(lines[start:end]).replace("\r\n", "\n").rstrip("\n") + "\n"


def top_level_yaml_mapping_block(text: str, key: str) -> str | None:
    return indented_yaml_mapping_block(text, key, 0)


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
    if policy.get("required_status_check") != EXPECTED_REQUIRED_STATUS:
        errors.append(f"required_status_check must be {EXPECTED_REQUIRED_STATUS}")

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
        for fragment in (
            "Mozilla Public License Version 2.0",
            "2. License Grants and Conditions",
            "3. Responsibilities",
            "10. Versions of the License",
            "Exhibit A - Source Code Form License Notice",
            'Exhibit B - "Incompatible With Secondary Licenses" Notice',
        ):
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
    if ruleset.get("name") != "Protect main" or ruleset.get("target") != "branch":
        errors.append("main ruleset must be the branch ruleset named Protect main")
    if ruleset.get("enforcement") != "active":
        errors.append("main ruleset must be active")
    if ruleset.get("bypass_actors") != []:
        errors.append("main ruleset must not define routine bypass actors")
    rule_types = {rule.get("type") for rule in ruleset.get("rules", []) if isinstance(rule, dict)}
    required_rule_types = {
        "deletion",
        "required_linear_history",
        "pull_request",
        "merge_queue",
        "required_status_checks",
        "non_fast_forward",
    }
    missing_rules = sorted(required_rule_types - rule_types)
    if missing_rules:
        errors.append(f"main ruleset missing rules: {', '.join(missing_rules)}")
    if "file_path_restriction" in rule_types:
        errors.append("file_path_restriction must not be placed in the branch ruleset")
    if "required_signatures" in rule_types:
        errors.append("strict signed commits are incompatible with third-party squash PRs")

    contexts = required_status_contexts(ruleset)
    if contexts != [EXPECTED_REQUIRED_STATUS]:
        errors.append(
            "ruleset required status checks must contain only the stable aggregate "
            f"{EXPECTED_REQUIRED_STATUS!r}; got {contexts!r}"
        )

    push_ruleset = policy.get("push_ruleset", {})
    if push_ruleset.get("name") != "Protect repository control plane" or push_ruleset.get("target") != "push":
        errors.append("control-plane ruleset must be a dedicated push ruleset")
    if push_ruleset.get("enforcement") != "active":
        errors.append("control-plane push ruleset must be active")
    if push_ruleset.get("bypass_actors") != []:
        errors.append("control-plane push ruleset must not define bypass actors")
    push_rule_types = {
        rule.get("type") for rule in push_ruleset.get("rules", []) if isinstance(rule, dict)
    }
    if push_rule_types != {"file_path_restriction"}:
        errors.append("control-plane push ruleset must contain only file_path_restriction")
    if restricted_file_paths(push_ruleset) != EXPECTED_CONTROL_PLANE_PATHS:
        errors.append("control-plane push ruleset restricted paths do not match canonical policy")

    workflow_dir = ROOT / ".github/workflows"
    if workflow_dir.is_dir():
        for workflow in sorted(workflow_dir.glob("*.y*ml")):
            text = workflow.read_text(encoding="utf-8")
            for action, ref in USES_LINE.findall(text):
                if re.fullmatch(r"[0-9a-f]{40}", ref) is None:
                    errors.append(f"{workflow.relative_to(ROOT)} uses unpinned action {action}@{ref}")
            if "pull_request_target:" in text and "actions/checkout" in text:
                errors.append(f"{workflow.relative_to(ROOT)} combines pull_request_target with checkout")

    merge_gate = ROOT / ".github/workflows/merge-gate.yml"
    if merge_gate.is_file():
        text = merge_gate.read_text(encoding="utf-8")
        top_level_keys = canonical_top_level_yaml_keys(text)
        if top_level_keys != EXPECTED_MERGE_GATE_TOP_LEVEL_KEYS:
            errors.append(
                "merge gate must use only the canonical top-level workflow keys in the "
                f"expected order; got {top_level_keys!r}"
            )
        trigger_block = top_level_yaml_mapping_block(text, "on")
        if trigger_block != EXPECTED_MERGE_GATE_TRIGGER_BLOCK:
            errors.append(
                "merge gate trigger block must exactly match the canonical always-on "
                "pull_request contract"
            )
        if "workflow_dispatch:" in text:
            errors.append("merge gate must not execute pull-request code through workflow_dispatch")
        scope_block = indented_yaml_mapping_block(text, "scope", 2)
        scope_digest = hashlib.sha256(scope_block.encode("utf-8")).hexdigest() if scope_block else None
        if scope_digest != EXPECTED_MERGE_GATE_SCOPE_JOB_SHA256:
            errors.append(
                "merge gate scope job must exactly match the canonical exact-head, "
                "changed-path classification and output implementation"
            )
        validate_block = indented_yaml_mapping_block(text, "validate", 2)
        validate_digest = hashlib.sha256(validate_block.encode("utf-8")).hexdigest() if validate_block else None
        if validate_digest != EXPECTED_MERGE_GATE_VALIDATE_JOB_SHA256:
            errors.append(
                "merge gate aggregate validate job must exactly match the canonical "
                "needs/result wiring and fail-closed implementation"
            )
        game_gate_block = indented_yaml_mapping_block(text, "game_gate", 2)
        if game_gate_block is None:
            errors.append("merge gate must publish the stable game-gate aggregate")
        else:
            for fragment in (
                "    name: game-gate\n",
                "    if: always()\n",
                "    needs: validate\n",
                "          LEGACY_VALIDATE: ${{ needs.validate.result }}\n",
                "        run: test \"$LEGACY_VALIDATE\" = \"success\"\n",
            ):
                if fragment not in game_gate_block:
                    errors.append(f"game-gate aggregate missing canonical fragment: {fragment.strip()}")
        for required_fragment in (
            "pull request head moved after event head was resolved",
            "changed_files = pull.get('changed_files')",
            "changed_files > 3000",
            "len(files) != changed_files",
            "previous_filename = item.get('previous_filename')",
            "classification_paths.append(previous_filename)",
            "prefixes = ('.cargo/', 'apps/', 'crates/', 'tests/', 'tools/', 'docs/migration/')",
            "base-ref: ${{ needs.scope.outputs.base_sha }}",
            "head-ref: ${{ needs.scope.outputs.target_sha }}",
            "Merge gate / governance",
            "Merge gate / dependency review",
            "Merge gate / CodeQL",
            "Merge gate / Rust policy and metadata",
            "Merge gate / Rust Linux workspace",
            "Merge gate / Rust Windows client",
            "Merge gate / Rust supply chain",
        ):
            if required_fragment not in text:
                errors.append(f"merge gate missing required recovery/sub-gate contract: {required_fragment}")

    merge_group_gate = ROOT / ".github/workflows/merge-group-gate.yml"
    if merge_group_gate.is_file():
        text = merge_group_gate.read_text(encoding="utf-8")
        top_level_keys = canonical_top_level_yaml_keys(text)
        if top_level_keys != EXPECTED_MERGE_GROUP_GATE_TOP_LEVEL_KEYS:
            errors.append(
                "merge-group gate must use only the canonical top-level workflow keys in the "
                f"expected order; got {top_level_keys!r}"
            )
        trigger_block = top_level_yaml_mapping_block(text, "on")
        if trigger_block != EXPECTED_MERGE_GROUP_GATE_TRIGGER_BLOCK:
            errors.append("merge-group gate trigger must be exactly merge_group/checks_requested")
        jobs_block = top_level_yaml_mapping_block(text, "jobs")
        job_keys = yaml_mapping_keys_at_indent(jobs_block or "", 2)
        if job_keys != EXPECTED_MERGE_GROUP_JOB_KEYS:
            errors.append(
                "merge-group gate jobs must exactly match the canonical qualification fan-in; "
                f"got {job_keys!r}"
            )

        required_by_job = {
            "candidate": (
                "    name: Merge Queue / candidate and governance\n",
                "[[ \"$BASE_REF\" == \"refs/heads/main\" ]]",
                "[[ \"$HEAD_SHA\" == \"$GITHUB_SHA_VALUE\" ]]",
                "git diff --check \"$BASE_SHA\" \"$HEAD_SHA\"",
                "python tools/agents/validate_governance.py",
                "python tools/repository/validate_repository_policy.py",
            ),
            "dependency_review": (
                "    name: Merge Queue / dependency review\n",
                "actions/dependency-review-action@a1d282b36b6f3519aa1f3fc636f609c47dddb294",
                "base-ref: ${{ github.event.merge_group.base_sha }}",
                "head-ref: ${{ github.event.merge_group.head_sha }}",
            ),
            "codeql": (
                "    name: Merge Queue / CodeQL (${{ matrix.language }})\n",
                "language: [python, actions]",
                "github/codeql-action/init@ff2f1c621b7f889edc0d3c761ac2e6a3f8cdb0dd",
                "github/codeql-action/analyze@ff2f1c621b7f889edc0d3c761ac2e6a3f8cdb0dd",
            ),
            "rust_linux": (
                "    name: Merge Queue / Rust Linux workspace\n",
                "cargo +1.94.0 build --locked --workspace --all-targets",
                "cargo +1.94.0 clippy --locked --workspace --all-targets -- -D warnings",
                "cargo +1.94.0 test --locked --workspace",
                "cargo +1.94.0 run --locked -p oteryn-synthetic-client-harness",
                "cargo +1.94.0 run --locked -p oteryn-game-server -- --smoke",
            ),
            "rust_windows": (
                "    name: Merge Queue / Rust Windows client\n",
                "--target x86_64-pc-windows-msvc",
                "cargo +1.94.0 run --locked -p oteryn-client --target x86_64-pc-windows-msvc -- --smoke",
                "cargo +1.94.0 run --locked -p oteryn-synthetic-client-harness",
            ),
            "rust_supply_chain": (
                "    name: Merge Queue / Rust supply chain\n",
                "EmbarkStudios/cargo-deny-action@3c6349835b2b7b196a839186cb8b78e02f7b5f25",
                "      command: check\n",
                "      arguments: --all-features\n",
            ),
            "game_gate": (
                "    name: game-gate\n",
                "    if: always()\n",
                "    needs: [candidate, dependency_review, codeql, rust_linux, rust_windows, rust_supply_chain]\n",
                "          CANDIDATE: ${{ needs.candidate.result }}\n",
                "          DEPENDENCY_REVIEW: ${{ needs.dependency_review.result }}\n",
                "          CODEQL: ${{ needs.codeql.result }}\n",
                "          RUST_LINUX: ${{ needs.rust_linux.result }}\n",
                "          RUST_WINDOWS: ${{ needs.rust_windows.result }}\n",
                "          RUST_SUPPLY_CHAIN: ${{ needs.rust_supply_chain.result }}\n",
                "            test \"$result\" = success\n",
            ),
        }
        for job, fragments in required_by_job.items():
            job_block = indented_yaml_mapping_block(text, job, 2)
            if job_block is None:
                errors.append(f"merge-group gate missing canonical job: {job}")
                continue
            for fragment in fragments:
                if fragment not in job_block:
                    errors.append(
                        f"merge-group gate job {job} missing canonical fragment: {fragment.strip()}"
                    )

    rust_workflow = ROOT / ".github/workflows/rust.yml"
    if rust_workflow.is_file():
        text = rust_workflow.read_text(encoding="utf-8")
        if "      - '.cargo/**'" not in text:
            errors.append("Rust post-merge workflow must treat .cargo/** as Rust-sensitive")

    dependabot = ROOT / ".github/dependabot.yml"
    if dependabot.is_file():
        text = dependabot.read_text(encoding="utf-8")
        for ecosystem in ("github-actions", "cargo"):
            if f"package-ecosystem: {ecosystem}" not in text:
                errors.append(f"Dependabot missing ecosystem: {ecosystem}")

    github_governance = ROOT / "docs/repository/GITHUB_GOVERNANCE.md"
    if github_governance.is_file():
        text = github_governance.read_text(encoding="utf-8")
        for fragment in (
            f"`{EXPECTED_REQUIRED_STATUS}` is the single stable required status check",
            "PR metadata edits automatically re-run the exact-head aggregate gate",
            "dedicated push ruleset",
            "Dependabot maintains both GitHub Actions and Cargo dependencies",
        ):
            if fragment not in text:
                errors.append(f"GitHub governance document missing current merge policy: {fragment}")

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
