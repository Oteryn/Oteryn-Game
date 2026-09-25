#!/usr/bin/env python3
"""Fail-closed wrapper for the Game repository-policy validator."""
from __future__ import annotations

import contextlib
import importlib.util
import io
import json
import os
import re
import tempfile
import textwrap
from pathlib import Path
from unittest.mock import patch

ROOT = Path(__file__).resolve().parents[2]
CORE_PATH = Path(__file__).with_name("validate_repository_policy_core.py")
PR_GATE_CONTRACT_PATH = Path(__file__).with_name("validate_pr_gate_pg_sim.py")
MERGE_AUTHORITY_AUDIT = ROOT / ".github/workflows/merge-authority-audit.yml"
PR_METADATA_WORKFLOWS = (
    (ROOT / ".github/workflows/agent-governance.yml", "Verify pull request target and metadata"),
    (ROOT / ".github/workflows/merge-gate.yml", "Verify pull request metadata"),
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


def _extract_step_python(text: str, step_name: str) -> str:
    step_marker = f"      - name: {step_name}\n"
    step_start = text.find(step_marker)
    if step_start < 0:
        raise ValueError(f"missing workflow step: {step_name}")
    heredoc = "          python - <<'PY'\n"
    source_start = text.find(heredoc, step_start)
    if source_start < 0:
        raise ValueError(f"missing Python heredoc in workflow step: {step_name}")
    source_start += len(heredoc)
    source_end = text.find("\n          PY\n", source_start)
    if source_end < 0:
        raise ValueError(f"unterminated Python heredoc in workflow step: {step_name}")
    return textwrap.dedent(text[source_start:source_end])


def _run_metadata_source(
    source: str,
    label: str,
    environment: dict[str, str],
    pull: dict[str, object],
) -> tuple[int, str, str]:
    stdout = io.StringIO()
    stderr = io.StringIO()
    payload = json.dumps(pull).encode("utf-8")

    def urlopen(request, timeout=30):
        if timeout != 30:
            raise AssertionError(f"{label} changed GitHub metadata timeout: {timeout}")
        return io.BytesIO(payload)

    with tempfile.TemporaryDirectory() as directory:
        env = dict(environment)
        env["GH_TOKEN"] = "fixture-token"
        env["GITHUB_ENV"] = str(Path(directory) / "github-env")
        exit_code = 0
        with (
            patch.dict(os.environ, env, clear=True),
            patch("urllib.request.urlopen", side_effect=urlopen),
            contextlib.redirect_stdout(stdout),
            contextlib.redirect_stderr(stderr),
        ):
            try:
                exec(compile(source, f"{label}:metadata", "exec"), {})
            except SystemExit as exc:
                if exc.code is None:
                    exit_code = 0
                elif isinstance(exc.code, int) and not isinstance(exc.code, bool):
                    exit_code = exc.code
                else:
                    exit_code = 1
    return exit_code, stdout.getvalue(), stderr.getvalue()


def _metadata_environment(step_name: str, expected_head: str) -> dict[str, str]:
    common = {
        "REPOSITORY": "Oteryn/Oteryn-Game",
    }
    if step_name == "Verify pull request target and metadata":
        return common | {
            "EVENT_NAME": "pull_request",
            "EVENT_PR_NUMBER": "914",
            "EVENT_PR_HEAD_SHA": expected_head,
        }
    if step_name == "Verify pull request metadata":
        return common | {
            "PULL_NUMBER": "914",
            "EXPECTED_HEAD": expected_head,
        }
    raise ValueError(f"unsupported PR metadata step: {step_name}")


def _valid_pull(expected_head: str) -> dict[str, object]:
    return {
        "state": "open",
        "title": "fix(ci): preserve advisory metadata",
        "body": "## Summary\nok\n## Scope\nok\n## Exact-head validation\nok\n",
        "head": {
            "sha": expected_head,
            "repo": {"full_name": "Oteryn/Oteryn-Game"},
        },
        "base": {"ref": "main"},
    }


def validate_pr_metadata_workflow_text(text: str, label: str, step_name: str) -> list[str]:
    errors: list[str] = []
    try:
        source = _extract_step_python(text, step_name)
        compile(source, f"{label}:metadata", "exec")
    except (SyntaxError, ValueError) as exc:
        return [f"{label} metadata validator is not executable: {exc}"]

    expected_head = "a" * 40
    environment = _metadata_environment(step_name, expected_head)

    def execute(name: str, pull: dict[str, object]) -> tuple[int, str, str] | None:
        try:
            return _run_metadata_source(source, f"{label}:{name}", environment, pull)
        except Exception as exc:
            errors.append(f"{label} metadata fixture {name} raised unexpectedly: {exc}")
            return None

    baseline = execute("baseline", _valid_pull(expected_head))
    if baseline is not None:
        code, stdout, stderr = baseline
        if code != 0:
            errors.append(
                f"{label} valid PR metadata must succeed, got exit {code}: {stderr.strip()}"
            )
        if "::warning title=PR metadata guidance::" in stdout:
            errors.append(f"{label} valid PR metadata must not emit presentation warnings")

    presentation = _valid_pull(expected_head)
    presentation["title"] = "not conventional " + ("x" * 80)
    presentation["body"] = ""
    result = execute("presentation-advisory", presentation)
    required_warnings = (
        "PR title should be at most 72 characters",
        "PR title should follow type(scope): imperative summary",
        "PR body should include a Summary section",
        "PR body should include a Scope section",
        "PR body should include a validation section",
    )
    if result is not None:
        code, stdout, stderr = result
        if code != 0:
            errors.append(
                f"{label} presentation-only metadata must remain advisory, got exit {code}: "
                f"{stderr.strip()}"
            )
        for warning in required_warnings:
            if warning not in stdout:
                errors.append(f"{label} missing advisory metadata warning: {warning}")

    prevalidation = _valid_pull(expected_head)
    prevalidation["body"] = "## Summary\nok\n## Scope\nok\n## Prevalidation notes\nok\n"
    result = execute("prevalidation-heading", prevalidation)
    if result is not None:
        code, stdout, stderr = result
        if code != 0:
            errors.append(
                f"{label} prevalidation heading fixture must remain advisory, got exit {code}: "
                f"{stderr.strip()}"
            )
        if "PR body should include a validation section" not in stdout:
            errors.append(
                f"{label} must not treat Prevalidation as a validation heading"
            )

    invalidation = _valid_pull(expected_head)
    invalidation["body"] = "## Summary\nok\n## Scope\nok\n## Invalidation risks\nok\n"
    result = execute("invalidation-heading", invalidation)
    if result is not None:
        code, stdout, stderr = result
        if code != 0:
            errors.append(
                f"{label} invalidation heading fixture must remain advisory, got exit {code}: "
                f"{stderr.strip()}"
            )
        if "PR body should include a validation section" not in stdout:
            errors.append(
                f"{label} must not treat Invalidation as a validation heading"
            )

    identity_cases: tuple[tuple[str, dict[str, object]], ...] = (
        ("closed", {"state": "closed"}),
        ("head-mismatch", {"head": {"sha": "b" * 40, "repo": {"full_name": "Oteryn/Oteryn-Game"}}}),
        ("repository-mismatch", {"head": {"sha": expected_head, "repo": {"full_name": "Other/Repo"}}}),
        ("base-mismatch", {"base": {"ref": "release"}}),
    )
    for name, mutation in identity_cases:
        pull = _valid_pull(expected_head)
        pull.update(mutation)
        result = execute(name, pull)
        if result is None:
            continue
        code, _stdout, stderr = result
        if code != 1:
            errors.append(
                f"{label} identity fixture {name} must fail with SystemExit(1), "
                f"got {code}: {stderr.strip()}"
            )

    return errors


def validate_pr_metadata_advisory_contract() -> list[str]:
    errors: list[str] = []
    for path, step_name in PR_METADATA_WORKFLOWS:
        if not path.is_file():
            errors.append(f"missing PR metadata workflow: {path.relative_to(ROOT)}")
            continue
        text = path.read_text(encoding="utf-8")
        errors.extend(
            validate_pr_metadata_workflow_text(text, str(path.relative_to(ROOT)), step_name)
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
