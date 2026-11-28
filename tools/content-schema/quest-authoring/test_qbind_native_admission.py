import copy
import json
import os
from pathlib import Path
import subprocess
import unittest
from unittest.mock import patch

import qbind_native_admission as qbind
from quest_completion_import import OUTPUT, PLAN, RECEIPT

ROOT = Path(__file__).resolve().parents[3]


class QbindNativeAdmissionTest(unittest.TestCase):
    @classmethod
    def setUpClass(cls):
        cls.candidate = json.loads((ROOT / OUTPUT).read_bytes())
        cls.plan = json.loads((ROOT / PLAN).read_bytes())

    def test_deterministic_bytes(self):
        report = qbind._report(ROOT)
        raw = qbind._encode(report)
        self.assertEqual(raw, qbind._encode(qbind._report(ROOT)))
        self.assertEqual(raw, (ROOT / qbind._OUTPUT).read_bytes())
        self.assertEqual({"admitted": 1, "held": 303, "production": 0}, report["counts"])
        self.assertEqual({"schema": 1, "quests": []}, report["production_bindings"])

    def test_held_on_ambiguity(self):
        ambiguous = copy.deepcopy(self.plan)
        control = next(row for row in ambiguous["records"] if row["quest"] == qbind._CONTROL)
        control["stages"].append(copy.deepcopy(control["stages"][-1]))
        report = qbind._admit(ROOT, ambiguous, self.candidate)
        self.assertEqual([], report["admitted"])
        self.assertIn("ambiguous stage identity", next(
            row["reason"] for row in report["held"] if row["quest"] == qbind._CONTROL
        ))

    def test_held_on_pinned_input_errors(self):
        original = Path.read_bytes
        malformed = b"{"
        for case, reason in (
            ("digest mismatch", "digest mismatch"),
            ("missing plan", "missing plan"),
            ("malformed receipt", "Expecting property name"),
            ("malformed plan", "Expecting property name"),
            ("malformed candidate", "Expecting property name"),
        ):
            with self.subTest(case=case):
                def read_bytes(path):
                    if path == ROOT / PLAN:
                        if case == "digest mismatch":
                            return original(path)[:-1] + b"%"
                        if case == "missing plan":
                            raise FileNotFoundError("missing plan")
                        if case == "malformed plan":
                            return malformed
                    if path == ROOT / OUTPUT and case == "malformed candidate":
                        return malformed
                    if path == ROOT / RECEIPT:
                        if case == "malformed receipt":
                            return malformed
                        receipt = json.loads(original(path))
                        if case == "malformed plan":
                            receipt["binding_plan"]["sha256"] = qbind.sha(malformed)
                        if case == "malformed candidate":
                            receipt["sha256"] = qbind.sha(malformed)
                        return qbind._encode(receipt)
                    return original(path)

                with patch.object(Path, "read_bytes", read_bytes):
                    report = qbind._report(ROOT)
                self.assertEqual({"admitted": 0, "held": 304, "production": 0}, report["counts"])
                self.assertTrue(all(reason in row["reason"] for row in report["held"]
                                    if row["quest"] != qbind._BANK))

    def test_held_on_recipe_digest_mismatch(self):
        original = Path.read_bytes

        def read_bytes(path):
            raw = original(path)
            if path.name == "qmech-m01c-pilgrimage.json":
                owner = json.loads(raw)
                owner["quests"][0]["recipe_sha256"] = "f" * 64
                return qbind._encode(owner)
            return raw

        with patch.object(Path, "read_bytes", read_bytes):
            report = qbind._admit(ROOT, self.plan, self.candidate)
        self.assertEqual([], report["admitted"])
        self.assertIn("recipe digest mismatch", next(
            row["reason"] for row in report["held"] if row["quest"] == qbind._CONTROL
        ))

    def test_bank_robbery_never_admitted(self):
        enabled = copy.deepcopy(self.plan)
        bank = next(row for row in enabled["records"] if row["quest"] == qbind._BANK)
        enabled["runtime_enabled"] = bank["runtime_enabled"] = True
        for stage in bank["stages"]:
            stage["runtime_enabled"] = True
        report = qbind._admit(ROOT, enabled, self.candidate)
        self.assertEqual([], report["production_bindings"]["quests"])
        self.assertTrue(all(row["quest"] != qbind._BANK for row in report["admitted"]))
        self.assertEqual("Bank Robbery occurrence unavailable", next(
            row["reason"] for row in report["held"] if row["quest"] == qbind._BANK
        ))

    def test_rust_loader_accepts_emitted_fragment(self):
        subprocess.run(
            ["cargo", "test", "--locked", "-p", "oteryn-gameplay-quests", "--lib",
             "qbind_budget_tests::qbind_fragment_matches_boot_verifier"],
            cwd=ROOT, env={**os.environ, "CARGO_BUILD_JOBS": "2"}, check=True,
        )
