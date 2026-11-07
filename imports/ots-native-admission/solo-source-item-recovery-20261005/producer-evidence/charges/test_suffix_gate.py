"""Whole-parent Source-only inverse fixtures; no Native pool reads."""

import copy
import unittest

from suffix_gate import FIELD, KIND, extend_parent, project_parent


class SuffixGate(unittest.TestCase):
    def setUp(self):
        self.record = {
            "kind": "Item",
            "identity": {"key": "oteryn:item.tibia.i123", "revision": "own"},
            "semantics": {
                "classification": {"state": "KNOWN", "value": "retained"},
                FIELD: {
                    "state": "KNOWN",
                    "value": [{"parameter": {"kind": "DURATION"}}],
                },
            },
        }
        self.entries = [{"source_cut": "CANARY_47DF", "parameter": {"kind": KIND}}]

    def test_exact_old_known_vector_and_full_parent(self):
        after, witness = extend_parent(self.record, self.entries)
        self.assertEqual(project_parent(after, witness), self.record)
        self.assertEqual(
            after["semantics"][FIELD]["value"][:-1],
            self.record["semantics"][FIELD]["value"],
        )

    def test_native_drift_and_old_vector_drift_rejected(self):
        after, witness = extend_parent(self.record, self.entries)
        for field in ("classification", FIELD):
            bad = copy.deepcopy(after)
            bad["semantics"][field] = {"state": "UNKNOWN"}
            with self.assertRaisesRegex(ValueError, "AFTER_FULL_PARENT"):
                project_parent(bad, witness)

    def test_na_conflict_core_and_duplicate_owncut_held(self):
        for state in ("NOT_APPLICABLE", "CONFLICT"):
            bad = copy.deepcopy(self.record)
            bad["semantics"][FIELD] = {"state": state}
            with self.assertRaisesRegex(ValueError, "PRESERVE_NA_CONFLICT"):
                extend_parent(bad, self.entries)
        bad = copy.deepcopy(self.record)
        bad["identity"]["key"] = "oteryn:item.tibia.i901"
        with self.assertRaisesRegex(ValueError, "PROTECTED_CORE"):
            extend_parent(bad, self.entries)
        with self.assertRaisesRegex(ValueError, "OWN_CUT_UNIQUE"):
            extend_parent(self.record, self.entries * 2)

    def test_absent_shell_restored_exactly(self):
        self.record.pop("semantics")
        after, witness = extend_parent(self.record, self.entries)
        self.assertEqual(project_parent(after, witness), self.record)


if __name__ == "__main__":
    unittest.main()
