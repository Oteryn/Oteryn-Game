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
    (
        ROOT / ".github/workflows/agent-governance.yml",
        "validate",
        "Verify pull request target and metadata",
    ),
    (
        ROOT / ".github/workflows/merge-gate.yml",
        "governance",
        "Verify pull request metadata",
    ),
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


def _extract_step_text(text: str, step_name: str) -> str:
    step_marker = f"      - name: {step_name}\n"
    step_start = text.find(step_marker)
    if step_start < 0:
        raise ValueError(f"missing workflow step: {step_name}")
    next_step = text.find("\n      - name: ", step_start + len(step_marker))
    if next_step < 0:
        raise ValueError(f"workflow step has no bounded successor: {step_name}")
    return text[step_start:next_step]


def _extract_job_text(text: str, job_name: str) -> str:
    job_marker = f"  {job_name}:\n"
    job_start = text.find(job_marker)
    if job_start < 0:
        raise ValueError(f"missing workflow job: {job_name}")
    remainder = text[job_start + len(job_marker) :]
    next_job = re.search(r"(?m)^  [A-Za-z0-9_-]+:\s*$", remainder)
    job_end = (
        job_start + len(job_marker) + next_job.start()
        if next_job is not None
        else len(text)
    )
    return text[job_start:job_end]


def _normalize_mapping_key(raw_key: str) -> str:
    if raw_key.startswith('"'):
        key = json.loads(raw_key)
        if not isinstance(key, str):
            raise ValueError(f"non-string YAML mapping key: {raw_key}")
        return key
    if raw_key.startswith("'"):
        return raw_key[1:-1].replace("''", "'")
    return raw_key


def _mapping_entries_at_indent(
    block: str,
    indent: int,
    label: str,
) -> dict[str, list[str]]:
    entries: dict[str, list[str]] = {}
    prefix = " " * indent
    deeper_prefix = prefix + " "
    key_pattern = re.compile(
        r"""(?P<key>"(?:[^"\\]|\\.)*"|'(?:[^']|'')*'|[A-Za-z0-9_-]+)\s*:\s*(?P<value>.*?)\s*$"""
    )
    for line_number, line in enumerate(block.splitlines(), start=1):
        if not line.startswith(prefix) or line.startswith(deeper_prefix):
            continue
        content = line[indent:]
        if not content.strip() or content.lstrip().startswith("#"):
            continue
        match = key_pattern.fullmatch(content)
        if match is None:
            raise ValueError(
                f"{label} contains unsupported YAML mapping syntax at line "
                f"{line_number}: {content!r}"
            )
        key = _normalize_mapping_key(match.group("key"))
        entries.setdefault(key, []).append(match.group("value"))
    return entries


def _step_mapping_entries(step: str) -> dict[str, list[str]]:
    return _mapping_entries_at_indent(step, 8, "workflow metadata step")


def _extract_step_python(text: str, step_name: str) -> str:
    step = _extract_step_text(text, step_name)
    heredoc = "          python - <<'PY'\n"
    source_start = step.find(heredoc)
    if source_start < 0:
        raise ValueError(f"missing Python heredoc in workflow step: {step_name}")
    source_start += len(heredoc)
    source_end = step.find("\n          PY\n", source_start)
    if source_end < 0:
        raise ValueError(f"unterminated Python heredoc in workflow step: {step_name}")
    return textwrap.dedent(step[source_start:source_end])


def _run_metadata_source(
    source: str,
    label: str,
    environment: dict[str, str],
    pull: dict[str, object],
) -> tuple[int, str, str, str]:
    stdout = io.StringIO()
    stderr = io.StringIO()
    payload = json.dumps(pull).encode("utf-8")

    def urlopen(request, timeout=30):
        if timeout != 30:
            raise AssertionError(f"{label} changed GitHub metadata timeout: {timeout}")
        if environment.get("EVENT_NAME") == "workflow_dispatch":
            fixture_pr = environment.get("DISPATCH_PR_NUMBER", "")
        elif "PULL_NUMBER" in environment:
            fixture_pr = environment.get("PULL_NUMBER", "")
        else:
            fixture_pr = environment.get("EVENT_PR_NUMBER", "")
        expected_url = (
            f"https://api.github.com/repos/{environment['REPOSITORY']}/pulls/{fixture_pr}"
        )
        actual_url = getattr(request, "full_url", None)
        if actual_url != expected_url:
            raise AssertionError(
                f"{label} changed GitHub metadata request target: "
                f"expected {expected_url}, got {actual_url!r}"
            )
        if request.get_method() != "GET":
            raise AssertionError(
                f"{label} changed GitHub metadata request method: {request.get_method()}"
            )
        return io.BytesIO(payload)

    with tempfile.TemporaryDirectory() as directory:
        env = dict(environment)
        env["GH_TOKEN"] = "fixture-token"
        github_env_path = Path(directory) / "github-env"
        env["GITHUB_ENV"] = str(github_env_path)
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
        github_env = (
            github_env_path.read_text(encoding="utf-8")
            if github_env_path.is_file()
            else ""
        )
    return exit_code, stdout.getvalue(), stderr.getvalue(), github_env


def _metadata_environment(
    step_name: str,
    expected_head: str,
    event_name: str = "pull_request",
) -> dict[str, str]:
    common = {
        "REPOSITORY": "Oteryn/Oteryn-Game",
    }
    if step_name == "Verify pull request target and metadata":
        if event_name == "pull_request":
            return common | {
                "EVENT_NAME": "pull_request",
                "EVENT_PR_NUMBER": "914",
                "EVENT_PR_HEAD_SHA": expected_head,
            }
        if event_name == "workflow_dispatch":
            return common | {
                "EVENT_NAME": "workflow_dispatch",
                "DISPATCH_PR_NUMBER": "914",
                "DISPATCH_EXPECTED_HEAD_SHA": expected_head,
                "EVENT_SHA": expected_head,
            }
        raise ValueError(f"unsupported agent-governance event: {event_name}")
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


def validate_pr_metadata_workflow_text(
    text: str,
    label: str,
    job_name: str,
    step_name: str,
) -> list[str]:
    errors: list[str] = []
    try:
        job = _extract_job_text(text, job_name)
        step = _extract_step_text(text, step_name)
        job_entries = _mapping_entries_at_indent(
            job,
            4,
            f"{label} job {job_name}",
        )
        step_entries = _step_mapping_entries(step)
        source = _extract_step_python(text, step_name)
        compile(source, f"{label}:metadata", "exec")
    except (SyntaxError, ValueError) as exc:
        return [f"{label} metadata validator is not executable: {exc}"]

    job_continue_on_error = job_entries.get("continue-on-error", [])
    if job_continue_on_error:
        errors.append(
            f"{label} job {job_name} must not use continue-on-error, "
            f"got {job_continue_on_error!r}"
        )
    job_conditions = job_entries.get("if", [])
    if job_conditions:
        errors.append(
            f"{label} job {job_name} must not be conditionally skipped, "
            f"got {job_conditions!r}"
        )

    continue_on_error = step_entries.get("continue-on-error", [])
    if continue_on_error:
        errors.append(
            f"{label} metadata step must not use continue-on-error: "
            f"the tested script result must govern the job, got {continue_on_error!r}"
        )

    step_conditions = step_entries.get("if", [])
    if step_name == "Verify pull request target and metadata":
        expected_condition = (
            "github.event_name == 'pull_request' || "
            "github.event_name == 'workflow_dispatch'"
        )
        if step_conditions != [expected_condition]:
            errors.append(
                f"{label} metadata step must use exactly the supported-event condition "
                f"{expected_condition!r}, got {step_conditions!r}"
            )
    elif step_conditions:
        errors.append(
            f"{label} metadata step must not be conditionally skipped, got {step_conditions!r}"
        )

    expected_head = "a" * 40
    environment = _metadata_environment(step_name, expected_head)

    def execute(
        name: str,
        pull: dict[str, object],
        fixture_environment: dict[str, str] | None = None,
    ) -> tuple[int, str, str, str] | None:
        try:
            return _run_metadata_source(
                source,
                f"{label}:{name}",
                fixture_environment or environment,
                pull,
            )
        except Exception as exc:
            errors.append(f"{label} metadata fixture {name} raised unexpectedly: {exc}")
            return None

    def require_target_sha(
        name: str,
        result: tuple[int, str, str, str] | None,
    ) -> None:
        if step_name != "Verify pull request target and metadata" or result is None:
            return
        _code, _stdout, _stderr, github_env = result
        expected = f"TARGET_SHA={expected_head}\n"
        if github_env != expected:
            errors.append(
                f"{label} metadata fixture {name} must write exactly {expected.strip()} "
                f"to GITHUB_ENV, got {github_env!r}"
            )

    baseline = execute("baseline", _valid_pull(expected_head))
    if baseline is not None:
        code, stdout, stderr, _github_env = baseline
        if code != 0:
            errors.append(
                f"{label} valid PR metadata must succeed, got exit {code}: {stderr.strip()}"
            )
        if "::warning title=PR metadata guidance::" in stdout:
            errors.append(f"{label} valid PR metadata must not emit presentation warnings")
    require_target_sha("baseline", baseline)

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
        code, stdout, stderr, _github_env = result
        if code != 0:
            errors.append(
                f"{label} presentation-only metadata must remain advisory, got exit {code}: "
                f"{stderr.strip()}"
            )
        for warning in required_warnings:
            if warning not in stdout:
                errors.append(f"{label} missing advisory metadata warning: {warning}")
    require_target_sha("presentation-advisory", result)

    prevalidation = _valid_pull(expected_head)
    prevalidation["body"] = "## Summary\nok\n## Scope\nok\n## Prevalidation notes\nok\n"
    result = execute("prevalidation-heading", prevalidation)
    if result is not None:
        code, stdout, stderr, _github_env = result
        if code != 0:
            errors.append(
                f"{label} prevalidation heading fixture must remain advisory, got exit {code}: "
                f"{stderr.strip()}"
            )
        if "PR body should include a validation section" not in stdout:
            errors.append(
                f"{label} must not treat Prevalidation as a validation heading"
            )
    require_target_sha("prevalidation-heading", result)

    invalidation = _valid_pull(expected_head)
    invalidation["body"] = "## Summary\nok\n## Scope\nok\n## Invalidation risks\nok\n"
    result = execute("invalidation-heading", invalidation)
    if result is not None:
        code, stdout, stderr, _github_env = result
        if code != 0:
            errors.append(
                f"{label} invalidation heading fixture must remain advisory, got exit {code}: "
                f"{stderr.strip()}"
            )
        if "PR body should include a validation section" not in stdout:
            errors.append(
                f"{label} must not treat Invalidation as a validation heading"
            )
    require_target_sha("invalidation-heading", result)

    identity_cases: tuple[tuple[str, dict[str, object]], ...] = (
        ("closed", {"state": "closed"}),
        ("head-mismatch", {"head": {"sha": "b" * 40, "repo": {"full_name": "Oteryn/Oteryn-Game"}}}),
        ("repository-mismatch", {"head": {"sha": expected_head, "repo": {"full_name": "Other/Repo"}}}),
        ("base-mismatch", {"base": {"ref": "release"}}),
    )

    def validate_identity_matrix(
        prefix: str,
        fixture_environment: dict[str, str],
    ) -> None:
        for name, mutation in identity_cases:
            pull = _valid_pull(expected_head)
            pull.update(mutation)
            result = execute(
                f"{prefix}-{name}",
                pull,
                fixture_environment,
            )
            if result is None:
                continue
            code, _stdout, stderr, _github_env = result
            if code != 1:
                errors.append(
                    f"{label} identity fixture {prefix}-{name} must fail with SystemExit(1), "
                    f"got {code}: {stderr.strip()}"
                )

    validate_identity_matrix("pull-request", environment)

    if step_name == "Verify pull request target and metadata":
        dispatch_environment = _metadata_environment(
            step_name,
            expected_head,
            "workflow_dispatch",
        )
        dispatch_baseline = execute(
            "dispatch-baseline",
            _valid_pull(expected_head),
            dispatch_environment,
        )
        if dispatch_baseline is not None:
            code, stdout, stderr, _github_env = dispatch_baseline
            if code != 0:
                errors.append(
                    f"{label} workflow_dispatch baseline must succeed, got exit {code}: "
                    f"{stderr.strip()}"
                )
            if "::warning title=PR metadata guidance::" in stdout:
                errors.append(
                    f"{label} workflow_dispatch baseline must not emit presentation warnings"
                )
        require_target_sha("dispatch-baseline", dispatch_baseline)
        validate_identity_matrix("workflow-dispatch", dispatch_environment)

        dispatch_input_cases = (
            ("bad-pr-number", {"DISPATCH_PR_NUMBER": "0"}),
            (
                "bad-expected-head",
                {
                    "DISPATCH_EXPECTED_HEAD_SHA": "not-a-sha",
                    "EVENT_SHA": "not-a-sha",
                },
            ),
            ("dispatch-ref-head-mismatch", {"EVENT_SHA": "b" * 40}),
        )
        for name, mutation in dispatch_input_cases:
            result = execute(
                name,
                _valid_pull(expected_head),
                dispatch_environment | mutation,
            )
            if result is None:
                continue
            code, _stdout, stderr, _github_env = result
            if code != 1:
                errors.append(
                    f"{label} workflow_dispatch fixture {name} must fail with SystemExit(1), "
                    f"got {code}: {stderr.strip()}"
                )

    return errors

def validate_pr_metadata_advisory_contract() -> list[str]:
    errors: list[str] = []
    for path, job_name, step_name in PR_METADATA_WORKFLOWS:
        if not path.is_file():
            errors.append(f"missing PR metadata workflow: {path.relative_to(ROOT)}")
            continue
        text = path.read_text(encoding="utf-8")
        errors.extend(
            validate_pr_metadata_workflow_text(
                text,
                str(path.relative_to(ROOT)),
                job_name,
                step_name,
            )
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
