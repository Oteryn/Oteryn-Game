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


def run_classifier(files, initial_change=None, final_change=None, expected_base="b" * 40, scope=False, immutable_files=None):
    """Execute the real workflow script with only GitHub HTTP responses replaced."""
    validator = load_validator()
    step = validator.step_block(
        validator.job_block(MERGE_GATE.read_text(encoding="utf-8"), "scope" if scope else "rust_linux"),
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

    def urlopen(request, timeout):
        assert timeout == 30
        prefix = "https://api.github.com/repos/Oteryn/Oteryn-Game/pulls/287"
        if request.full_url == prefix:
            return io.StringIO(json.dumps(next(pulls)))
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
        with patch.dict(os.environ, env), patch("urllib.request.urlopen", urlopen), redirect_stdout(io.StringIO()), redirect_stderr(io.StringIO()):
            try:
                exec(compile(script, "merge-gate.yml:pg_target", "exec"), {})
            except SystemExit as error:
                failure = str(error)
        return failure, output.read_text(encoding="utf-8") if output.exists() else ""


def test_both_classifiers_bind_aba_to_immutable_diff() -> None:
    mutable = [{"filename": "docs/new.md", "status": "modified"}]
    immutable = [{"filename": "apps/game-server/tests/durability_postgres.rs", "status": "removed"}]
    failures = []
    for scope in (True, False):
        failure, output = run_classifier(mutable, immutable_files=immutable, scope=scope)
        expected = "rust=true\n" if scope else "removed=true\n"
        if failure is not None or not output.endswith(expected):
            failures.append((scope, failure, output))
    assert not failures, f"A-to-B-to-A substituted mutable diff at authority boundary: {failures}"


def test_both_classifiers_reject_comparison_truncation() -> None:
    files = [{"filename": f"docs/{i}.md"} for i in range(301)]
    for scope in (True, False):
        failure, output = run_classifier(files[:300], scope=scope)
        assert failure is None and output, (scope, failure, output)
        failure, output = run_classifier(files, immutable_files=files[:300], scope=scope)
        assert failure is not None and not output, "over-cap diff must fail closed"
        failure, output = run_classifier(files[:2], immutable_files=files[:1], scope=scope)
        assert failure is not None and not output, "incomplete immutable diff must fail closed"


def test_scope_rejects_identity_races() -> None:
    # A Rust event head can otherwise consume a replacement docs-only listing of the same size.
    files = [{"filename": f"docs/file-{index}.md"} for index in range(101)]
    failure, output = run_classifier(files, scope=True)
    assert failure is None and output.endswith("rust=false\n"), (failure, output)
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
        expected = f"pr_number=287\ntarget_sha={'a' * 40}\nbase_sha={'b' * 40}\nrust={str(rust).lower()}\n"
        assert failure is None and output == expected, (failure, output)


def test_classifier_rejects_identity_races() -> None:
    # Two pages and an unchanged file count reproduce the review's race, not a count mismatch.
    files = [{"filename": f"docs/file-{index}.md", "status": "modified"} for index in range(101)]
    failure, output = run_classifier(files)
    assert failure is None and output == "removed=false\n", (failure, output)
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


def test_classifier_preserves_stable_removal_classification() -> None:
    target = "apps/game-server/tests/durability_postgres.rs"
    cases = (
        ([], "removed=false\n"),
        ([{"filename": target, "status": "modified"}], "removed=false\n"),
        ([{"filename": target, "status": "removed"}], "removed=true\n"),
        ([{"filename": "other.rs", "previous_filename": target, "status": "renamed"}], "removed=true\n"),
    )
    for files, expected in cases:
        failure, output = run_classifier(files)
        assert failure is None, failure
        assert output == expected, (files, output)


def test_evidence_step_condition_family() -> None:
    baseline = MERGE_GATE.read_text(encoding="utf-8")
    assert not validate_mutated_gate(baseline), "unmodified gate must pass before mutation checks"
    for job, name in (
        ("rust_linux", "Run Durability PostgreSQL E2E when allocated"),
        ("rust_windows", "Verify deterministic simulation golden fixtures"),
    ):
        marker = f"      - name: {name}\n"
        for condition in ('"if": false', "'if': false", "continue-on-error: true"):
            errors = validate_mutated_gate(baseline.replace(marker, marker + f"        {condition}\n", 1))
            assert any(job in error and "evidence step" in error for error in errors), (name, condition, errors)


def main() -> int:
    tests = (
        test_postgres_evidence_step_cannot_be_skipped,
        test_simulation_evidence_step_cannot_be_skipped,
        test_classifier_rejects_identity_races,
        test_classifier_rejects_unbound_base,
        test_classifier_preserves_stable_removal_classification,
        test_evidence_step_condition_family,
        test_scope_rejects_identity_races,
        test_scope_preserves_stable_classification,
        test_both_classifiers_bind_aba_to_immutable_diff,
        test_both_classifiers_reject_comparison_truncation,
    )
    for test in tests:
        test()
        print(f"PASS {test.__name__}")
    print(f"Canonical PR PG/SIM validator regressions PASS: {len(tests)}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
