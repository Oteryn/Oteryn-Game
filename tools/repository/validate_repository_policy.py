#!/usr/bin/env python3
"""Fail-closed wrapper for the Game repository-policy validator."""
from __future__ import annotations

import importlib.util
import re
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]
CORE_PATH = Path(__file__).with_name("validate_repository_policy_core.py")
PR_GATE_CONTRACT_PATH = Path(__file__).with_name("validate_pr_gate_pg_sim.py")
MERGE_AUTHORITY_AUDIT = ROOT / ".github/workflows/merge-authority-audit.yml"
PR_METADATA_WORKFLOWS = (
    ROOT / ".github/workflows/agent-governance.yml",
    ROOT / ".github/workflows/merge-gate.yml",
)


def load_module(path: Path, name: str):
    spec = importlib.util.spec_from_file_location(name, path)
    if spec is None or spec.loader is None:
        raise RuntimeError(f"cannot load repository policy module: {path}")
    module = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(module)
    return module


def load_core():
    return load_module(CORE_PATH, "validate_repository_policy_core")


def validate_protected_base_audit() -> list[str]:
    if not MERGE_AUTHORITY_AUDIT.is_file():
        return ["missing protected-base merge-authority audit"]

    text = MERGE_AUTHORITY_AUDIT.read_text(encoding="utf-8")
    errors: list[str] = []
    for fragment in (
        "  pull_request_target:\n",
        "      - opened\n",
        "      - reopened\n",
        "      - synchronize\n",
        "      - edited\n",
        "      contents: read\n",
        "      pull-requests: read\n",
        "candidate merge-group gate does not match the protected-base approved blob",
        "candidate modifies the protected-base audit itself",
    ):
        if fragment not in text:
            errors.append(f"protected-base merge-authority audit missing contract: {fragment.strip()}")

    pin = re.search(r'^\s*EXPECTED_MERGE_GROUP_GATE_BLOB:\s*"([0-9a-f]{40})"\s*$', text, re.MULTILINE)
    if pin is None:
        errors.append("protected-base merge-authority audit must own an exact gate blob pin")
    if "actions/checkout@" in text:
        errors.append("protected-base merge-authority audit must not checkout candidate code")
    if re.search(r'^\s*continue-on-error\s*:', text, re.MULTILINE):
        errors.append("protected-base merge-authority audit must not permit continue-on-error")

    for fragment in (
        "merge_group_head_expression = '$' + '{{ github.event.merge_group.head_sha }}'",
        "durability_result_expression = '$' + '{{ needs.durability_postgres.result }}'",
        "f'          EXPECTED_SHA: {merge_group_head_expression}\\n'",
        "f'          DURABILITY_POSTGRES: {durability_result_expression}\\n'",
    ):
        if fragment not in text:
            errors.append(
                "protected-base merge-authority audit must construct future-context "
                f"workflow expressions at Python runtime: {fragment}"
            )

    for unsafe in (
        "          EXPECTED_SHA: ${{ github.event.merge_group.head_sha }}\\n",
        "          DURABILITY_POSTGRES: ${{ needs.durability_postgres.result }}\\n",
    ):
        if unsafe in text:
            errors.append(
                "protected-base merge-authority audit must not embed a future-context "
                f"GitHub expression directly in its pull_request_target run script: {unsafe.strip()}"
            )
    return errors


def validate_pr_metadata_advisory_contract() -> list[str]:
    errors: list[str] = []
    forbidden = (
        "errors.append('PR title must be at most 72 characters')",
        'errors.append("PR title must be at most 72 characters")',
        "errors.append('PR title must follow type(scope): imperative summary')",
        'errors.append("PR title must follow type(scope): imperative summary")',
        "errors.append(f'PR body is missing {heading}')",
        'errors.append(f"PR body is missing {heading}")',
    )
    required = (
        "warnings: list[str] = []",
        "warnings.append('PR title should be at most 72 characters')",
        "warnings.append('PR title should follow type(scope): imperative summary')",
        "any('validation' in heading for heading in headings)",
        "::warning title=PR metadata guidance::",
    )
    for path in PR_METADATA_WORKFLOWS:
        if not path.is_file():
            errors.append(f"missing PR metadata workflow: {path.relative_to(ROOT)}")
            continue
        text = path.read_text(encoding="utf-8")
        for fragment in forbidden:
            if fragment in text:
                errors.append(
                    f"{path.relative_to(ROOT)} must not hard-fail presentation-only PR metadata: {fragment}"
                )
        for fragment in required:
            if fragment not in text:
                errors.append(
                    f"{path.relative_to(ROOT)} missing advisory PR metadata contract: {fragment}"
                )
    return errors


def validate_pr_gate_contract() -> list[str]:
    module = load_module(PR_GATE_CONTRACT_PATH, "validate_pr_gate_pg_sim")
    return module.validate()


def main() -> int:
    errors = validate_protected_base_audit()
    errors.extend(validate_pr_metadata_advisory_contract())
    errors.extend(validate_pr_gate_contract())
    if errors:
        print("Repository policy validation failed:")
        for error in errors:
            print(f"- {error}")
        return 1
    return load_core().main()


if __name__ == "__main__":
    raise SystemExit(main())
