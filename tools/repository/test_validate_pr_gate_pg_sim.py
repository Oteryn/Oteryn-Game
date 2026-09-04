#!/usr/bin/env python3
"""Regression tests for canonical PR PostgreSQL/SIM evidence-step validation."""
from __future__ import annotations

import importlib.util
import tempfile
from pathlib import Path

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


def main() -> int:
    tests = (
        test_postgres_evidence_step_cannot_be_skipped,
        test_simulation_evidence_step_cannot_be_skipped,
    )
    for test in tests:
        test()
        print(f"PASS {test.__name__}")
    print(f"Canonical PR PG/SIM validator regressions PASS: {len(tests)}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
