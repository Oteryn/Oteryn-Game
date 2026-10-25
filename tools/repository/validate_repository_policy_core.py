#!/usr/bin/env python3
"""Validate repository-level GitHub governance without external dependencies."""

from __future__ import annotations

import hashlib
import json
import re
import subprocess
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
EXPECTED_MERGE_GATE_SCOPE_JOB_SHA256 = "ff75ac35780038e15ba648e99b8540ef474d2b34b093fe684c045dc302265bd4"
EXPECTED_MERGE_GATE_VALIDATE_JOB_SHA256 = "331213b6a65f8852cd43eaeba80d7cb4bc9b5dc928399cf679475c85bf0a589e"
EXPECTED_MERGE_GATE_FINAL_JOB_SHA256 = "9c5223ecee21f441d85fec7995135e8361f0c711876810b5a076de28e010e032"
EXPECTED_MERGE_GATE_LANES_JOB_SHA256 = "4102f456f4fed6e8cbcb5b9b53356279f7fae42017b1157b4dae1eac7b17bde4"
EXPECTED_MERGE_GATE_ROUTING_CONTRACT_JOB_SHA256 = "24b5197d8c22835f08a27423ae909f2bdf6b240e7c64026ce76174753c820e4c"
EXPECTED_ROUTING_CONTRACT_VALIDATOR_BLOB = "ce2fc840f22fd75c0ccb067d9807698a87650f77"
EXPECTED_MERGE_GATE_WORLD_BUNDLE_JOB_SHA256 = "0aa50bfeaab0042f2a8fec7e7e302a7f056e77170db28bea7e0a6ed32dea1dd9"
EXPECTED_MERGE_GROUP_GATE_BLOB = "9e377ee40894b16320ba5dd800af4458e04a04db"
EXPECTED_POST_MERGE_RUST_SHA256 = "39d0b76e70a55d733b697eefe46264f753e3ac8ffdf955bf03beaeb30e60732c"
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
  push:
    branches: [main]
"""
EXPECTED_MERGE_GROUP_JOB_KEYS = [
    "candidate",
    "dependency_review",
    "codeql",
    "atlas_fullworld",
    "world_bundle",
    "rust_linux",
    "durability_postgres_build",
    "durability_postgres",
    "rust_windows",
    "rust_supply_chain",
    "node_boot",
    "server_seam",
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
    "tools/repository/validate_pr_routing_contract.py",
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


def validate_post_merge_rust(text: str) -> list[str]:
    # Reuse the repository's canonical workflow/job pin pattern. This binds
    # selection, failure fallbacks and evidence commands, not just substrings.
    if hashlib.sha256(text.encode("utf-8")).hexdigest() != EXPECTED_POST_MERGE_RUST_SHA256:
        return ["Rust post-merge workflow must match the reviewed fail-closed contract"]
    return []


def main() -> int:
    errors: list[str] = []

    for relative in REQUIRED_FILES:
        if not (ROOT / relative).is_file():
            errors.append(f"missing required repository-governance file: {relative}")

    routing_validator = ROOT / "tools/repository/validate_pr_routing_contract.py"
    if routing_validator.is_file():
        validator_text = routing_validator.read_text(encoding="utf-8")
        if git_blob_sha(validator_text.encode("utf-8")) != EXPECTED_ROUTING_CONTRACT_VALIDATOR_BLOB:
            errors.append("routing contract validator must equal the reviewed fail-closed implementation")

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
        for forbidden_fragment, label in (
            ("errors.append('PR title", "blocking PR title presentation lint"),
            ('errors.append("PR title', "blocking PR title presentation lint"),
            ("PR body is missing {heading}", "blocking exact PR heading presentation lint"),
        ):
            if forbidden_fragment in text:
                errors.append(f"merge gate must not contain {label}")
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
        if "  group: ${{ format('merge-gate-{0}', github.event.pull_request.number) }}\n" not in text:
            errors.append("merge gate runs of one pull request must share one concurrency group")
        if "  cancel-in-progress: ${{ !(github.event.action == 'edited' && github.event.changes.base == null) }}\n" not in text:
            errors.append("merge gate metadata edits must not cancel product qualifications")
        scope_block = indented_yaml_mapping_block(text, "scope", 2)
        scope_digest = hashlib.sha256(scope_block.encode("utf-8")).hexdigest() if scope_block else None
        if scope_digest != EXPECTED_MERGE_GATE_SCOPE_JOB_SHA256:
            errors.append(
                "merge gate scope job must exactly match the canonical exact-head, "
                "changed-path classification and output implementation"
            )
        lanes_block = indented_yaml_mapping_block(text, "lanes", 2)
        lanes_digest = hashlib.sha256(lanes_block.encode("utf-8")).hexdigest() if lanes_block else None
        if lanes_digest != EXPECTED_MERGE_GATE_LANES_JOB_SHA256:
            errors.append("merge gate risk lanes must exactly match trusted-base classification and fail-closed outputs")
        routing_contract_block = indented_yaml_mapping_block(text, "routing_contract", 2)
        routing_contract_digest = (
            hashlib.sha256(routing_contract_block.encode("utf-8")).hexdigest()
            if routing_contract_block else None
        )
        if routing_contract_digest != EXPECTED_MERGE_GATE_ROUTING_CONTRACT_JOB_SHA256:
            errors.append("merge gate routing contract job must exactly match the reviewed exact-head consumer-routing contract")
        world_block = indented_yaml_mapping_block(text, "world_bundle", 2)
        world_digest = hashlib.sha256(world_block.encode("utf-8")).hexdigest() if world_block else None
        if world_digest != EXPECTED_MERGE_GATE_WORLD_BUNDLE_JOB_SHA256:
            errors.append("merge gate World bundle must retain its reviewed fail-closed qualification")
        jobs = yaml_mapping_keys_at_indent(top_level_yaml_mapping_block(text, "jobs") or "", 2)
        if jobs != ["scope", "lanes", "governance", "dependency_review", "routing_contract", "world_bundle", "rust_policy", "rust_fast", "rust_supply_chain", "validate", "game_gate"]:
            errors.append("merge gate must contain only the canonical fast PR jobs")
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
                "    if: always() && (github.event.pull_request.base.ref == 'main' || !github.event.pull_request.base.ref)\n",
                "    needs: [scope, validate]\n",
                "          LEGACY_VALIDATE: ${{ needs.validate.result }}\n",
                "        run: test \"$LEGACY_VALIDATE\" = \"success\"\n",
            ):
                if fragment not in game_gate_block:
                    errors.append(f"game-gate aggregate missing canonical fragment: {fragment.strip()}")
        final_digest = hashlib.sha256(game_gate_block.encode("utf-8")).hexdigest() if game_gate_block else None
        if final_digest != EXPECTED_MERGE_GATE_FINAL_JOB_SHA256:
            errors.append("game-gate must retain canonical aggregation and the isolated edit live-target fence")
        for required_fragment in (
            "pull request head moved after event head was resolved",
            "changed_files = pull.get('changed_files')",
            "len(files) != changed_files",
            "{key: item[key] for key in ('filename', 'status', 'previous_filename') if key in item}",
            "Merge gate / trusted-base risk lanes",
            "base-ref: ${{ needs.scope.outputs.base_sha }}",
            "head-ref: ${{ needs.scope.outputs.target_sha }}",
            "Merge gate / governance",
            "Merge gate / dependency review",
            "Merge gate / routing contract",
            "Verify protected-base routing contract health",
            "git diff --check \"$EXPECTED_BASE\" \"$EXPECTED_HEAD\"",
            "Merge gate / Rust policy and metadata",
            "Merge gate / Rust changed-crate Clippy and unit tests",
            "Merge gate / Rust supply chain",
        ):
            if required_fragment not in text:
                errors.append(f"merge gate missing required recovery/sub-gate contract: {required_fragment}")

    merge_group_gate = ROOT / ".github/workflows/merge-group-gate.yml"
    if merge_group_gate.is_file():
        text = merge_group_gate.read_text(encoding="utf-8")
        if git_blob_sha(text.encode("utf-8")) != EXPECTED_MERGE_GROUP_GATE_BLOB:
            errors.append("merge-group gate must equal the protected-base preapproved PG/SIM blob")
        top_level_keys = canonical_top_level_yaml_keys(text)
        if top_level_keys != EXPECTED_MERGE_GROUP_GATE_TOP_LEVEL_KEYS:
            errors.append(
                "merge-group gate must use only the canonical top-level workflow keys in the "
                f"expected order; got {top_level_keys!r}"
            )
        trigger_block = top_level_yaml_mapping_block(text, "on")
        if trigger_block != EXPECTED_MERGE_GROUP_GATE_TRIGGER_BLOCK:
            errors.append("merge-group gate trigger must be exactly merge_group/checks_requested plus push to main")
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
                "merge_group) [[ \"$EVENT_ACTION\" == \"checks_requested\" ]] ;;",
                "BASE_REF: ${{ github.event.merge_group.base_ref || github.ref }}",
                "[[ \"$HEAD_SHA\" == \"$GITHUB_SHA_VALUE\" ]]",
                "git diff --check \"$BASE_SHA\" \"$HEAD_SHA\"",
                "python tools/agents/validate_governance.py",
                "python tools/repository/validate_repository_policy.py",
                "python tools/agents/tests/test_governance_lifecycle_discovery.py",
                "python tools/repository/test_validate_merge_group_pg_sim.py",
            ),
            "dependency_review": (
                "    name: Merge Queue / dependency review\n",
                "actions/dependency-review-action@a1d282b36b6f3519aa1f3fc636f609c47dddb294",
                "base-ref: ${{ github.event.merge_group.base_sha || github.event.before }}",
                "head-ref: ${{ github.event.merge_group.head_sha || github.sha }}",
            ),
            "codeql": (
                "    name: Merge Queue / CodeQL (${{ matrix.language }})\n",
                "language: [python, actions]",
                "github/codeql-action/init@2892aa5e19bbd11bc0cff5427e3b750a04d9e3c2",
                "github/codeql-action/analyze@2892aa5e19bbd11bc0cff5427e3b750a04d9e3c2",
            ),
            "atlas_fullworld": (
                "    name: Merge Queue / Atlas fullworld source\n",
                "    needs: candidate\n",
                "    if: needs.candidate.outputs.atlas_fullworld != 'false'\n",
                "ref: ${{ github.event.merge_group.head_sha || github.sha }}",
                "EXPECTED_SHA: ${{ github.event.merge_group.head_sha || github.sha }}",
                'run: test "$(git rev-parse HEAD)" = "$EXPECTED_SHA"',
                "python -S -m py_compile",
                "tools/game-atlas-fullworld-source/producer.py",
                "tools/game-atlas-fullworld-source/self_test.py",
                "tools/reference-world-corridor-census/content_source_batch.py",
                "tools/reference-world-corridor-census/content_source_batch_self_test.py",
                "run: python -S tools/game-atlas-fullworld-source/self_test.py",
                "run: python -S tools/reference-world-corridor-census/content_source_batch_self_test.py",
            ),
            "world_bundle": (
                "    name: Merge Queue / World bundle\n",
                "    needs: candidate\n",
                "    if: needs.candidate.outputs.world_bundle != 'false'\n",
                "ref: ${{ github.event.merge_group.head_sha || github.sha }}",
                "EXPECTED_SHA: ${{ github.event.merge_group.head_sha || github.sha }}",
                'run: test "$(git rev-parse HEAD)" = "$EXPECTED_SHA"',
                "cargo +1.99.1 run --locked --release -p oteryn-world-bundle-compiler -- pin-check . \"$RUNNER_TEMP/world-bundle\"",
                "if-no-files-found: error",
            ),
            "rust_linux": (
                "    name: Merge Queue / Rust Linux workspace\n",
                "      fail-fast: true\n",
                "        shard: [lint, lib-content, lib-rest, integration-heavy, integration-rest, build-smoke]\n",
                "cargo +1.99.1 clippy --locked --workspace --all-targets -- -D warnings",
                "python tools/repository/test_check_duplicate_test_sources.py",
                'python tools/repository/check_duplicate_test_sources.py "$RUNNER_TEMP/clippy-messages.jsonl"',
                "NEXTEST_SHA256: 682c21b777c333e96fd532e114d3a5a894e0729ab88d94c0a9f20f8419695428",
                "echo \"${NEXTEST_SHA256}  ${archive}\" | sha256sum -c -",
                "cargo +1.99.1 nextest run --locked --workspace --lib --no-fail-fast -E 'test(content::)'",
                "cargo +1.99.1 nextest run --locked --workspace --lib --no-fail-fast -E 'not test(content::)'",
                'python tools/repository/plan_rust_test_shards.py "$RUNNER_TEMP/metadata.json" "$SHARD"',
                'cargo +1.99.1 nextest run --locked --workspace "${shard_args[@]}" --no-fail-fast',
                "cargo +1.99.1 build --locked --workspace --bins --examples --benches",
                "cargo +1.99.1 test --locked --workspace --bins",
                "cargo +1.99.1 test --locked --workspace --doc",
                "cargo +1.99.1 run --locked -p oteryn-synthetic-client-harness",
                "cargo +1.99.1 run --locked -p oteryn-game-server -- --smoke",
            ),
            "durability_postgres_build": (
                "    name: Merge Queue / Durability PostgreSQL harness build\n",
                "    if: needs.candidate.outputs.rust == 'true'\n",
                "ref: ${{ github.event.merge_group.head_sha || github.sha }}",
                "BASE_SHA: ${{ github.event.merge_group.base_sha || github.event.before }}",
                "HEAD_SHA: ${{ github.event.merge_group.head_sha || github.sha }}",
                'git cat-file -e "$BASE_SHA:$path"',
                'git cat-file -e "$HEAD_SHA:$path"',
                'if [[ "$base_present" == true && "$head_present" == false ]]; then',
                "verify_registered_target_binding() {",
                'cargo +1.99.1 metadata --locked --no-deps --format-version 1 > "$metadata"',
                "owners = [package for package in packages if package.get('name') == 'oteryn-game-server']",
                "expected_manifest = (pathlib.Path.cwd() / 'apps/game-server/Cargo.toml').resolve(strict=True)",
                "observed_manifest = pathlib.Path(owners[0]['manifest_path']).resolve(strict=True)",
                "if observed_manifest != expected_manifest:",
                "matches = [target for target in targets if target.get('name') == name]",
                "if len(matches) != 1 or matches[0].get('kind') != ['test']:",
                "expected = (pathlib.Path.cwd() / registered_path).resolve(strict=True)",
                "observed = pathlib.Path(matches[0]['src_path']).resolve(strict=True)",
                'verify_registered_target_binding "$name" "$path"',
                'archived_targets+=(--test "$name")',
                "(( ${#archived_targets[@]} > 0 )) || {",
                'cargo +1.99.1 nextest archive --locked --workspace "${archived_targets[@]}" --archive-file "$RUNNER_TEMP/pg-archives/pg-e2e.tar.zst"',
                "run_registered_target durability_postgres apps/game-server/tests/durability_postgres.rs",
                "run_registered_target character_authority_postgres apps/game-server/tests/character_authority_postgres.rs",
                "run_registered_target runtime_scope_assignment_postgres apps/game-server/tests/runtime_scope_assignment_postgres.rs",
                "run_registered_target native_admission_source_postgres apps/game-server/tests/native_admission_source_postgres.rs",
                'cargo +1.99.1 nextest archive --locked -p oteryn-gameplay-durability -p oteryn-gameplay-dispatch -p oteryn-game-server --lib --archive-file "$RUNNER_TEMP/pg-archives/pg-lib.tar.zst"',
                "NEXTEST_SHA256: 682c21b777c333e96fd532e114d3a5a894e0729ab88d94c0a9f20f8419695428",
                'echo "${NEXTEST_SHA256}  ${archive}" | sha256sum -c -',
                "sha256sum head-sha pg-e2e.tar.zst pg-lib.tar.zst > SHA256SUMS",
                "actions/upload-artifact@043fb46d1a93c77aae656e7c1c64a875d1fc6a0a",
                "          if-no-files-found: error\n",
            ),
            "durability_postgres": (
                "    name: Merge Queue / Durability PostgreSQL harness\n",
                "    needs: [candidate, durability_postgres_build]\n",
                "    if: needs.candidate.outputs.rust == 'true'\n",
                "image: postgres:18.6-bookworm@sha256:bbf83594798e144bff365a9c191c5d779f181fb3135902064ab8ff11918020fc",
                "ref: ${{ github.event.merge_group.head_sha || github.sha }}",
                "NEXTEST_SHA256: 682c21b777c333e96fd532e114d3a5a894e0729ab88d94c0a9f20f8419695428",
                'echo "${NEXTEST_SHA256}  ${archive}" | sha256sum -c -',
                "actions/download-artifact@9000827ccba6bdab643e8b6fd33ac0654aef8333",
                "ARCHIVES_SHA256: ${{ needs.durability_postgres_build.outputs.archives_sha256 }}",
                'echo "${ARCHIVES_SHA256}  SHA256SUMS" | sha256sum -c -',
                "sha256sum -c SHA256SUMS",
                'test "$(cat head-sha)" = "$EXPECTED_SHA"',
                'test "$LABEL" = "$((JOB_INDEX + 1))"',
                'echo "spec=count:$((JOB_INDEX + 1))/${JOB_TOTAL}" >> "$GITHUB_OUTPUT"',
                'cargo-nextest nextest run --archive-file "$RUNNER_TEMP/pg-archives/pg-e2e.tar.zst" --workspace-remap "$GITHUB_WORKSPACE" --partition "$PARTITION_SPEC" --no-fail-fast',
                "if: ${{ !cancelled() && steps.partition.outcome == 'success' }}",
                'cargo-nextest nextest run --archive-file "$RUNNER_TEMP/pg-archives/pg-lib.tar.zst" --workspace-remap "$GITHUB_WORKSPACE" --partition "$PARTITION_SPEC" --no-fail-fast',
            ),
            "rust_windows": (
                "    name: Merge Queue / Rust Windows client\n",
                "EXPECTED_SHA: ${{ github.event.merge_group.head_sha || github.sha }}",
                "$ErrorActionPreference = 'Stop'",
                "$PSNativeCommandUseErrorActionPreference = $true",
                "TARGET_SHA: ${{ github.event.merge_group.head_sha || github.sha }}",
                '"OTERYN_RELEASE_ID=$version+dev.ci.g$($env:TARGET_SHA.Substring(0, 12))" >> $env:GITHUB_ENV',
                "INNO_SETUP_SHA256: 9c73c3bae7ed48d44112a0f48e66742c00090bdb5bef71d9d3c056c66e97b732",
                "Inno Setup download SHA-256 mismatch",
                "Inno Setup install failed with exit code",
                r".\apps\client\installer\ci-installer.ps1",
                "-GameCommit $env:TARGET_SHA",
                "          if-no-files-found: error",
                "cargo +1.99.1 test --locked -p oteryn-input-platform --target x86_64-pc-windows-msvc",
                "cargo +1.99.1 test --locked -p oteryn-simulation-determinism --target x86_64-pc-windows-msvc",
                "--target x86_64-pc-windows-msvc",
                '$client = ".\\target\\x86_64-pc-windows-msvc\\release\\oteryn-client.exe"',
                "Test-Path -LiteralPath $client -PathType Leaf",
                "& $client --smoke",
                "cargo +1.99.1 run --locked -p oteryn-synthetic-client-harness --target x86_64-pc-windows-msvc",
            ),
            "rust_supply_chain": (
                "    name: Merge Queue / Rust supply chain\n",
                "EmbarkStudios/cargo-deny-action@3c6349835b2b7b196a839186cb8b78e02f7b5f25",
                "      command: check\n",
                "      arguments: --all-features\n",
            ),
            "node_boot": (
                "    name: Merge Queue / Node boot against the real Platform\n",
                "    if: needs.candidate.outputs.server_qualification != 'false'\n",
                "EXPECTED_SHA: ${{ github.event.merge_group.head_sha || github.sha }}",
                "ref: 5d4883acf7079e26fd51e03f460166730de1ada0",
                "bash tools/qualification/node_boot/run.sh | tee \"$evidence\"",
                "grep -Fxq 'NODE_BOOT_RESULT=NODE_BOOT_PASS' \"$evidence\"",
            ),
            "server_seam": (
                "    name: Merge Queue / Server Seam over TCP+TLS\n",
                "    if: needs.candidate.outputs.server_qualification != 'false'\n",
                "EXPECTED_SHA: ${{ github.event.merge_group.head_sha || github.sha }}",
                "ref: 5d4883acf7079e26fd51e03f460166730de1ada0",
                "WP5_QUALIFICATION=seam bash tools/qualification/wp5_s3b/run.sh | tee \"$evidence\"",
                "grep -Fxq 'S3B_RESULT=SEAM_PASS' \"$evidence\"",
            ),
            "game_gate": (
                "    name: game-gate\n",
                "    if: always()\n",
                "    needs: [candidate, dependency_review, codeql, atlas_fullworld, world_bundle, rust_linux, durability_postgres, rust_windows, rust_supply_chain, node_boot, server_seam]\n",
                "          CANDIDATE: ${{ needs.candidate.result }}\n",
                "          DEPENDENCY_REVIEW: ${{ needs.dependency_review.result }}\n",
                "          CODEQL: ${{ needs.codeql.result }}\n",
                "          ATLAS_FULLWORLD_REQUIRED: ${{ needs.candidate.outputs.atlas_fullworld }}\n",
                "          ATLAS_FULLWORLD: ${{ needs.atlas_fullworld.result }}\n",
                "          WORLD_BUNDLE_REQUIRED: ${{ needs.candidate.outputs.world_bundle }}\n",
                "          WORLD_BUNDLE: ${{ needs.world_bundle.result }}\n",
                "          RUST_LINUX: ${{ needs.rust_linux.result }}\n",
                "          DURABILITY_POSTGRES: ${{ needs.durability_postgres.result }}\n",
                "          RUST_WINDOWS: ${{ needs.rust_windows.result }}\n",
                "          RUST_SUPPLY_CHAIN: ${{ needs.rust_supply_chain.result }}\n",
                "          SERVER_QUALIFICATION_REQUIRED: ${{ needs.candidate.outputs.server_qualification }}\n",
                "          NODE_BOOT: ${{ needs.node_boot.result }}\n",
                "          SERVER_SEAM: ${{ needs.server_seam.result }}\n",
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
        # The PostgreSQL partitions only run the archives built once by durability_postgres_build.
        partitions = indented_yaml_mapping_block(text, "durability_postgres", 2)
        for forbidden in ("rustup ", "cargo +", "cargo build", "cargo test", "actions/cache/"):
            if partitions is not None and forbidden in partitions:
                errors.append(
                    f"merge-group gate job durability_postgres must not compile or restore a cache: {forbidden.strip()}"
                )

    agent_governance = ROOT / ".github/workflows/agent-governance.yml"
    if agent_governance.is_file():
        text = agent_governance.read_text(encoding="utf-8")
        for forbidden_fragment, label in (
            ("errors.append('PR title", "blocking PR title presentation lint"),
            ('errors.append("PR title', "blocking PR title presentation lint"),
            ("PR body is missing {heading}", "blocking exact PR heading presentation lint"),
        ):
            if forbidden_fragment in text:
                errors.append(f"agent governance must not contain {label}")

    rust_workflow = ROOT / ".github/workflows/rust.yml"
    if rust_workflow.is_file():
        text = rust_workflow.read_text(encoding="utf-8")
        errors.extend(validate_post_merge_rust(text))
        if not errors:
            regression = subprocess.run([sys.executable, str(ROOT / "tools/repository/test_classify_post_merge_lanes.py")], cwd=ROOT, check=False)
            if regression.returncode != 0:
                errors.append("Rust post-merge behavioral regressions failed")

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
