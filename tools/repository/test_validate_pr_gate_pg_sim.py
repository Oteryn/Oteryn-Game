#!/usr/bin/env python3
"""Regression tests for canonical PR PostgreSQL/SIM evidence-step validation."""
from __future__ import annotations

import copy
import importlib.util
import io
import json
import os
import tempfile
import textwrap
import urllib.error
from contextlib import redirect_stderr, redirect_stdout
from pathlib import Path
from unittest.mock import patch

ROOT = Path(__file__).resolve().parents[2]
VALIDATOR_PATH = Path(__file__).with_name("validate_pr_gate_pg_sim.py")
MERGE_GATE = ROOT / ".github/workflows/merge-gate.yml"


def load_validator():
    spec = importlib.util.spec_from_file_location("validate_pr_gate_pg_sim", VALIDATOR_PATH)
    if spec is None or spec.loader is None:
        raise RuntimeError(f"cannot load validator: {VALIDATOR_PATH}")
    module = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(module)
    return module


def validate_mutated_gate(text: str) -> list[str]:
    module = load_validator()
    with tempfile.TemporaryDirectory() as directory:
        path = Path(directory) / "merge-gate.yml"
        path.write_text(text, encoding="utf-8")
        module.MERGE_GATE = path
        return module.validate()


def inject_step_condition(text: str, step_name: str) -> str:
    marker = f"      - name: {step_name}\n"
    if text.count(marker) != 1:
        raise AssertionError(f"expected exactly one step named {step_name!r}")
    return text.replace(marker, marker + "        if: false\n", 1)


def missing_target(message="Not Found", code=404):
    return urllib.error.HTTPError(
        "https://api.github.test/exact",
        code,
        "error",
        {},
        io.BytesIO(json.dumps({"message": message}).encode("utf-8")),
    )


def test_postgres_evidence_step_cannot_be_skipped() -> None:
    baseline = MERGE_GATE.read_text(encoding="utf-8")
    mutated = inject_step_condition(baseline, "Run Durability PostgreSQL E2E when allocated")
    errors = validate_mutated_gate(mutated)
    assert errors, "validator accepted a skipped PostgreSQL evidence step"
    assert any("rust_linux" in error and "if" in error for error in errors), errors


def test_simulation_evidence_step_cannot_be_skipped() -> None:
    baseline = MERGE_GATE.read_text(encoding="utf-8")
    mutated = inject_step_condition(baseline, "Verify deterministic simulation golden fixtures")
    errors = validate_mutated_gate(mutated)
    assert errors, "validator accepted a skipped simulation evidence step"
    assert any("rust_windows" in error and "if" in error for error in errors), errors


def run_classifier(
    files,
    initial_change=None,
    final_change=None,
    expected_base="b" * 40,
    scope=False,
    immutable_files=None,
    target_payloads=None,
    checkout_present=True,
    workflow_text=None,
):
    """Execute the real workflow script with only GitHub HTTP responses replaced."""
    validator = load_validator()
    step = validator.step_block(
        validator.job_block(
            workflow_text or MERGE_GATE.read_text(encoding="utf-8"),
            "scope" if scope else "rust_linux",
        ),
        "Resolve and validate exact pull request head" if scope else "Classify Durability PostgreSQL target",
    )
    assert step is not None
    script = textwrap.dedent(step.split("python - <<'PY'\n", 1)[1].rsplit("          PY", 1)[0])
    initial = {
        "state": "open",
        "head": {"sha": "a" * 40, "repo": {"full_name": "Oteryn/Oteryn-Game"}},
        "base": {"sha": "b" * 40, "ref": "main"},
        "changed_files": len(files),
    }
    final = copy.deepcopy(initial)
    if initial_change:
        initial_change(initial)
    if final_change:
        final_change(final)
    pulls = iter((initial, final))
    target_payloads = target_payloads or {
        "b" * 40: {"path": "apps/game-server/tests/durability_postgres.rs", "type": "file"},
        "a" * 40: {"path": "apps/game-server/tests/durability_postgres.rs", "type": "file"},
    }

    def response(payload):
        if isinstance(payload, BaseException):
            raise payload
        return io.StringIO(json.dumps(payload))

    def urlopen(request, timeout):
        assert timeout == 30
        prefix = "https://api.github.com/repos/Oteryn/Oteryn-Game/pulls/287"
        if request.full_url == prefix:
            return response(next(pulls))
        contents = "https://api.github.com/repos/Oteryn/Oteryn-Game/contents/apps/game-server/tests/durability_postgres.rs?ref="
        if request.full_url.startswith(contents):
            return response(target_payloads[request.full_url.removeprefix(contents)])
        comparison = f"https://api.github.com/repos/Oteryn/Oteryn-Game/compare/{'b' * 40}...{'a' * 40}?per_page=1"
        if request.full_url == comparison:
            return io.StringIO(json.dumps({"files": files if immutable_files is None else immutable_files}))
        page_prefix = prefix + "/files?per_page=100&page="
        assert request.full_url.startswith(page_prefix), request.full_url
        page = int(request.full_url.removeprefix(page_prefix))
        return io.StringIO(json.dumps(files[(page - 1) * 100:page * 100]))

    with tempfile.TemporaryDirectory() as directory:
        output = Path(directory) / "output"
        env = {
            "REPOSITORY": "Oteryn/Oteryn-Game", "PULL_NUMBER": "287",
            "EXPECTED_HEAD": "a" * 40, "EXPECTED_BASE": expected_base,
            "EVENT_PR_NUMBER": "287", "EVENT_PR_HEAD_SHA": "a" * 40,
            "GH_TOKEN": "test-only", "GITHUB_OUTPUT": str(output),
        }
        failure = None
        with patch.dict(os.environ, env), patch("urllib.request.urlopen", urlopen), patch(
            "os.path.isfile", return_value=checkout_present
        ), redirect_stdout(io.StringIO()), redirect_stderr(io.StringIO()):
            try:
                exec(compile(script, "merge-gate.yml:pg_target", "exec"), {})
            except SystemExit as error:
                failure = str(error)
        return failure, output.read_text(encoding="utf-8") if output.exists() else ""


def test_classifiers_bind_aba_to_immutable_authority() -> None:
    mutable = [{"filename": "docs/new.md", "status": "modified"}]
    immutable = [{"filename": "apps/game-server/tests/durability_postgres.rs", "status": "removed"}]
    failure, output = run_classifier(mutable, immutable_files=immutable, scope=True)
    assert failure is None
    assert json.loads(dict(line.split("=", 1) for line in output.splitlines())["file_records"]) == immutable
    failure, output = run_classifier(mutable)
    assert failure is None and output == "present=true\n", (failure, output)


def test_scope_rejects_comparison_truncation_while_pg_accepts_large_diff() -> None:
    files = [{"filename": f"docs/{i}.md"} for i in range(301)]
    failure, output = run_classifier(files, immutable_files=files[:300], scope=True)
    assert failure is None and output.endswith("complete=false\n")
    failure, output = run_classifier([{"filename": f"docs/{i}.md"} for i in range(803)])
    assert failure is None and output == "present=true\n", (failure, output)


def test_protected_classifier_red_for_valid_803_file_target() -> None:
    protected = os.popen("git show 4bc27844ffde2a645b5df85c7268babae283d866:.github/workflows/merge-gate.yml").read()
    assert protected
    files = [{"filename": f"docs/{i}.md"} for i in range(802)] + [
        {"filename": "apps/game-server/tests/durability_postgres.rs", "status": "modified"}
    ]
    failure, output = run_classifier(files, workflow_text=protected)
    assert failure == "invalid or over-cap changed-files count" and not output, (failure, output)


def test_scope_rejects_identity_races() -> None:
    # A Rust event head can otherwise consume a replacement docs-only listing of the same size.
    files = [{"filename": f"docs/file-{index}.md"} for index in range(101)]
    failure, output = run_classifier(files, scope=True)
    assert failure is None and output.endswith("complete=true\n"), (failure, output)
    mutations = {
        "closed": lambda pull: pull.update(state="closed"),
        "head": lambda pull: pull["head"].update(sha="c" * 40),
        "repository": lambda pull: pull["head"]["repo"].update(full_name="other/repository"),
        "base": lambda pull: pull["base"].update(sha="c" * 40),
        "base_ref": lambda pull: pull["base"].update(ref="other"),
        "count": lambda pull: pull.update(changed_files=102),
    }
    accepted = []
    for name, change in mutations.items():
        failure, output = run_classifier(files, final_change=change, scope=True)
        if failure is None or output:
            accepted.append(name)
    assert not accepted, f"scope emitted authority after PR identity changed: {accepted}"


def test_scope_preserves_stable_classification() -> None:
    for files, rust in (
        ([], False),
        ([{"filename": "README.md"}], False),
        ([{"filename": "apps/game-server/src/lib.rs"}], True),
        ([{"filename": "docs/old.md", "previous_filename": "crates/old.rs"}], True),
    ):
        failure, output = run_classifier(files, scope=True)
        expected = f"pr_number=287\ntarget_sha={'a' * 40}\nbase_sha={'b' * 40}\nfile_count={len(files)}\nfile_records={json.dumps(files, separators=(',', ':'))}\ncomplete=true\n"
        assert failure is None and output == expected, (failure, output)


def test_scope_bounds_environment_transport() -> None:
    files = [{"filename": "apps/game-server/src/lib.rs", "status": "modified", "patch": "x" * 140000}]
    failure, output = run_classifier(files, scope=True)
    fields = dict(line.split("=", 1) for line in output.splitlines())
    assert failure is None and fields["complete"] == "true"
    assert json.loads(fields["file_records"]) == [{"filename": files[0]["filename"], "status": "modified"}], "scope must transport classification fields only"
    # Even projected paths must not exceed an environment-variable launch limit.
    files = [{"filename": f"docs/{i}/" + "x" * 200 + ".md", "status": "modified"} for i in range(300)]
    failure, output = run_classifier(files, scope=True)
    fields = dict(line.split("=", 1) for line in output.splitlines())
    assert failure is None and fields["complete"] == "false" and fields["file_records"] == "[]", "oversize scope must select FULL before process launch"


def test_classifier_rejects_identity_races() -> None:
    # Two pages and an unchanged file count reproduce the review's race, not a count mismatch.
    files = [{"filename": f"docs/file-{index}.md", "status": "modified"} for index in range(101)]
    failure, output = run_classifier(files)
    assert failure is None and output == "present=true\n", (failure, output)
    mutations = {
        "closed": lambda pull: pull.update(state="closed"),
        "head": lambda pull: pull["head"].update(sha="c" * 40),
        "repository": lambda pull: pull["head"]["repo"].update(full_name="other/repository"),
        "base": lambda pull: pull["base"].update(sha="c" * 40),
        "count": lambda pull: pull.update(changed_files=102),
    }
    accepted = []
    for name, change in mutations.items():
        failure, output = run_classifier(files, final_change=change)
        if failure is None or output:
            accepted.append(name)
    assert not accepted, f"classifier emitted a result after PR identity changed: {accepted}"


def test_classifier_rejects_unbound_base() -> None:
    cases = (
        {"expected_base": ""},
        {"expected_base": "not-a-sha"},
        {"initial_change": lambda pull: pull["base"].update(sha="c" * 40)},
    )
    for case in cases:
        failure, output = run_classifier([], **case)
        assert failure is not None and not output, "classifier accepted an invalid or changed base"


def test_classifier_exact_target_state_matrix() -> None:
    target = "apps/game-server/tests/durability_postgres.rs"
    present = {"path": target, "type": "file"}
    cases = (
        ("large-present", present, present, True, None, "present=true\n"),
        ("introduced", missing_target(), present, True, None, "present=true\n"),
        ("both-absent", missing_target(), missing_target(), False, None, "present=false\n"),
        ("removed", present, missing_target(), False, "removed or renamed", ""),
        ("renamed-away", present, missing_target(), False, "removed or renamed", ""),
    )
    for name, base, head, checkout, expected_failure, expected_output in cases:
        failure, output = run_classifier(
            [{"filename": target if name != "renamed-away" else "tests/renamed.rs"}] * 803,
            target_payloads={"b" * 40: base, "a" * 40: head},
            checkout_present=checkout,
        )
        assert (expected_failure is None and failure is None) or expected_failure in failure, (name, failure)
        assert output == expected_output, (name, output)


def test_classifier_rejects_bad_target_evidence_and_checkout_mismatch() -> None:
    target = "apps/game-server/tests/durability_postgres.rs"
    present = {"path": target, "type": "file"}
    bad = (
        [],
        {"path": "other.rs", "type": "file"},
        {"path": target, "type": "dir"},
        missing_target("forbidden"),
        missing_target(code=401),
        missing_target(code=403),
        missing_target(code=429),
        missing_target(code=500),
        urllib.error.URLError("transport"),
        ValueError("malformed JSON"),
    )
    for payload in bad:
        failure, output = run_classifier(
            [], target_payloads={"b" * 40: present, "a" * 40: payload}
        )
        assert failure is not None and not output, payload
    for api_present, checkout_present in ((True, False), (False, True)):
        payload = present if api_present else missing_target()
        failure, output = run_classifier(
            [],
            target_payloads={"b" * 40: missing_target(), "a" * 40: payload},
            checkout_present=checkout_present,
        )
        assert "checkout and API target state disagree" in failure and not output


def test_classifier_rejects_invalid_changed_file_counts() -> None:
    for value in (-1, True, "803", None):
        failure, output = run_classifier([], initial_change=lambda pull, value=value: pull.update(changed_files=value))
        assert failure is not None and not output


def test_evidence_step_condition_family() -> None:
    baseline = MERGE_GATE.read_text(encoding="utf-8")
    assert not validate_mutated_gate(baseline), "unmodified gate must pass before mutation checks"
    for job, name in (
        ("rust_linux", "Run Durability PostgreSQL E2E when allocated"),
        ("rust_windows", "Verify deterministic simulation golden fixtures"),
    ):
        marker = f"      - name: {name}\n"
        for condition in ('"if": false', "'if': false", "continue-on-error: true", '"continue-on-error": true', "'continue-on-error': true"):
            errors = validate_mutated_gate(baseline.replace(marker, marker + f"        {condition}\n", 1))
            assert any(job in error and "evidence step" in error for error in errors), (name, condition, errors)


def test_evidence_job_failure_cannot_be_tolerated() -> None:
    baseline = MERGE_GATE.read_text(encoding="utf-8")
    assert not validate_mutated_gate(baseline), "stable gate must pass before mutation checks"
    accepted = []
    for job in ("rust_linux", "rust_windows"):
        marker = f"  {job}:\n"
        assert baseline.count(marker) == 1
        for key in ("continue-on-error", '"continue-on-error"', "'continue-on-error'"):
            errors = validate_mutated_gate(baseline.replace(marker, marker + f"    {key}: true\n", 1))
            if not any(job in error and "continue-on-error" in error for error in errors):
                accepted.append((job, key))
    assert not accepted, f"validator accepted tolerated evidence-job failure: {accepted}"


def test_postgres_early_exit_cannot_preserve_contract_strings() -> None:
    baseline = MERGE_GATE.read_text(encoding="utf-8")
    marker = "          set -euo pipefail\n"
    linux = load_validator().job_block(baseline, "rust_linux")
    assert linux is not None and linux.count(marker) == 1
    mutated = baseline.replace(linux, linux.replace(marker, marker + "          exit 0\n", 1), 1)
    errors = validate_mutated_gate(mutated)
    assert errors, "validator accepted early exit before PG evidence with contract strings intact"


def test_postgres_digest_and_invocation_are_mandatory() -> None:
    baseline = MERGE_GATE.read_text(encoding="utf-8")
    stale = baseline.replace("      - name: Build workspace\n", "      - name: Build workspace # stale\n", 1)
    assert any("rust_linux" in error and "exactly match" in error for error in validate_mutated_gate(stale))
    invocation = "            cargo +1.94.0 test --locked -p oteryn-game-server --test durability_postgres\n"
    assert baseline.count(invocation) == 1
    errors = validate_mutated_gate(baseline.replace(invocation, "", 1))
    assert any("rust_linux" in error and "durability_postgres" in error for error in errors), errors


def main() -> int:
    tests = (
        test_postgres_evidence_step_cannot_be_skipped,
        test_simulation_evidence_step_cannot_be_skipped,
        test_classifier_rejects_identity_races,
        test_classifier_rejects_unbound_base,
        test_classifier_exact_target_state_matrix,
        test_classifier_rejects_bad_target_evidence_and_checkout_mismatch,
        test_classifier_rejects_invalid_changed_file_counts,
        test_evidence_step_condition_family,
        test_scope_rejects_identity_races,
        test_scope_preserves_stable_classification,
        test_scope_bounds_environment_transport,
        test_classifiers_bind_aba_to_immutable_authority,
        test_scope_rejects_comparison_truncation_while_pg_accepts_large_diff,
        test_protected_classifier_red_for_valid_803_file_target,
        test_evidence_job_failure_cannot_be_tolerated,
        test_postgres_early_exit_cannot_preserve_contract_strings,
        test_postgres_digest_and_invocation_are_mandatory,
    )
    for test in tests:
        test()
        print(f"PASS {test.__name__}")
    spec = importlib.util.spec_from_file_location("queue_regressions", Path(__file__).with_name("test_validate_merge_group_pg_sim.py"))
    assert spec is not None and spec.loader is not None
    module = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(module)
    assert module.main() == 0
    spec = importlib.util.spec_from_file_location("risk_regressions", Path(__file__).with_name("test_classify_pr_test_lanes.py"))
    assert spec is not None and spec.loader is not None
    module = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(module)
    assert module.main() == 0
    print(f"Canonical PR PG/SIM validator regressions PASS: {len(tests)}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
