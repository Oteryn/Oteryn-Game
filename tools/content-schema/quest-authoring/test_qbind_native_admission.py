import copy
import json
import os
from pathlib import Path
import subprocess

import pytest

import qbind_native_admission as qbind
from quest_completion_import import OUTPUT, PLAN

ROOT = Path(__file__).resolve().parents[3]


@pytest.fixture(scope="module")
def candidate():
    return json.loads((ROOT / OUTPUT).read_bytes())


@pytest.fixture(scope="module")
def plan():
    return json.loads((ROOT / PLAN).read_bytes())


def test_deterministic_bytes():
    report = qbind._report(ROOT)
    raw = qbind._encode(report)
    assert raw == qbind._encode(qbind._report(ROOT))
    assert raw == (ROOT / qbind._OUTPUT).read_bytes()
    assert report["counts"] == {"admitted": 1, "held": 303, "production": 0}
    assert report["production_bindings"] == {"schema": 1, "quests": []}


def test_held_on_ambiguity(plan, candidate):
    ambiguous = copy.deepcopy(plan)
    control = next(row for row in ambiguous["records"] if row["quest"] == qbind._CONTROL)
    control["stages"].append(copy.deepcopy(control["stages"][-1]))
    report = qbind._admit(ROOT, ambiguous, candidate)
    assert report["admitted"] == []
    assert "ambiguous stage identity" in next(
        row["reason"] for row in report["held"] if row["quest"] == qbind._CONTROL
    )


def test_held_on_pinned_digest_mismatch(monkeypatch):
    def read_bytes(path):
        raw = original(path)
        if path == ROOT / PLAN:
            return raw[:-1] + b"%"
        return raw

    original = Path.read_bytes
    monkeypatch.setattr(Path, "read_bytes", read_bytes)
    report = qbind._report(ROOT)
    assert report["counts"] == {"admitted": 0, "held": 304, "production": 0}
    assert all("digest mismatch" in row["reason"] for row in report["held"]
               if row["quest"] != qbind._BANK)


def test_held_on_recipe_digest_mismatch(plan, candidate, monkeypatch):
    original = Path.read_bytes

    def read_bytes(path):
        raw = original(path)
        if path.name == "qmech-m01c-pilgrimage.json":
            owner = json.loads(raw)
            owner["quests"][0]["recipe_sha256"] = "f" * 64
            return qbind._encode(owner)
        return raw

    monkeypatch.setattr(Path, "read_bytes", read_bytes)
    report = qbind._admit(ROOT, plan, candidate)
    assert report["admitted"] == []
    assert "recipe digest mismatch" in next(
        row["reason"] for row in report["held"] if row["quest"] == qbind._CONTROL
    )


def test_bank_robbery_never_admitted(plan, candidate):
    enabled = copy.deepcopy(plan)
    bank = next(row for row in enabled["records"] if row["quest"] == qbind._BANK)
    enabled["runtime_enabled"] = bank["runtime_enabled"] = True
    for stage in bank["stages"]:
        stage["runtime_enabled"] = True
    report = qbind._admit(ROOT, enabled, candidate)
    assert report["production_bindings"]["quests"] == []
    assert all(row["quest"] != qbind._BANK for row in report["admitted"])
    assert next(row["reason"] for row in report["held"] if row["quest"] == qbind._BANK
                ) == "Bank Robbery occurrence unavailable"


def test_rust_loader_accepts_emitted_fragment():
    subprocess.run(
        ["cargo", "test", "--locked", "-p", "oteryn-gameplay-quests", "--lib",
         "qbind_budget_tests::qbind_fragment_matches_boot_verifier"],
        cwd=ROOT, env={**os.environ, "CARGO_BUILD_JOBS": "2"}, check=True,
    )
