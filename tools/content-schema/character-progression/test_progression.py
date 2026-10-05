import copy
import hashlib
import json
import subprocess
import sys
import tempfile
import unittest
from pathlib import Path

sys.path.insert(0, str(Path(__file__).parent))
import progression_lib as lib  # noqa: E402

HERE = Path(__file__).parent
K = lib.EXPERIENCE_EVIDENCE_LAST_LEVEL


def evidence(**over):
    doc = lib.build_evidence("2026-10-05")
    doc.update(over)
    return doc


def docs(ev=None):
    ev = ev or evidence()
    return {
        "evidence": ev,
        "table": lib.build_table(ev),
        "death": lib.build_death(),
        "reward": lib.build_reward(),
        "differences": lib.build_differences(),
    }


class Formula(unittest.TestCase):
    def test_known_levels(self):
        self.assertEqual(lib.experience_for_level(1), 0)
        self.assertEqual(lib.experience_for_level(2), 100)
        self.assertEqual(lib.experience_for_level(3), 200)

    def test_table_shape(self):
        table = docs()["table"]
        self.assertEqual(len(table["levels"]), 2000)
        self.assertEqual(table["levels"][0]["minimum_experience"], "0")
        self.assertEqual(table["levels"][1]["minimum_experience"], "100")
        values = [int(r["minimum_experience"]) for r in table["levels"]]
        self.assertEqual(values, sorted(set(values)))
        self.assertGreater(int(table["terminal_exclusive_experience"]), values[-1])

    def test_levels_hash_matches_formula(self):
        lib.verify_levels_hash(evidence())
        with self.assertRaises(lib.ProgressionError):
            lib.verify_levels_hash(evidence(levels_sha256="0" * 64))

    def test_coefficient_change_fails_hash(self):
        original = lib.FORMULA_COEFFICIENTS
        try:
            lib.FORMULA_COEFFICIENTS = (1, -6, 17, -11)
            with self.assertRaises(lib.ProgressionError):
                lib.verify_levels_hash(evidence())
        finally:
            lib.FORMULA_COEFFICIENTS = original


class FormulaEvidence(unittest.TestCase):
    def test_rendered_text(self):
        self.assertEqual(lib.formula_text(), "exp(L) = 50/3 * (L^3 - 6L^2 + 17L - 12)")

    def test_exact_integers_and_divisibility(self):
        for level in range(1, 2002):
            p = level**3 - 6 * level**2 + 17 * level - 12
            self.assertEqual(p % 3, 0)
            self.assertEqual(lib.experience_for_level(level), 50 * (p // 3))

    def test_tampered_metadata_refused(self):
        for over in (
            {"formula": "exp(L) = 50/3 * (L^3 - 6L^2 + 17L - 11)"},
            {"formula": lib.formula_text() + " "},
            {"source_kind": "private_capture"},
            {"last_level": 1999},
            {"extra": "x"},
        ):
            ev = evidence(**over)
            d = docs(evidence())
            d["evidence"] = ev
            with self.assertRaises((lib.ProgressionError, KeyError), msg=over):
                lib.verify_documents(d)

    def test_samples(self):
        with tempfile.TemporaryDirectory() as tmp:
            path = Path(tmp) / "s.txt"
            path.write_text("2,100\n24,%d\n2000,%d\n" % (lib.experience_for_level(24), lib.experience_for_level(2000)))
            self.assertEqual(lib.check_samples(path), 3)
            for bad in ("2,101\n", "5,%d\n2,100\n" % lib.experience_for_level(5), "2,100\n2,100\n", "2001,1\n", "x\n", ""):
                path.write_text(bad)
                with self.assertRaises(lib.ProgressionError, msg=bad):
                    lib.check_samples(path)


class Revisions(unittest.TestCase):
    def test_changes_propagate(self):
        base = lib.revisions(docs())
        for field in ("provenance_url", "recorded_on", "extraction_method"):
            changed = lib.revisions(docs(evidence(**{field: "x" + str(evidence()[field])})))
            self.assertNotEqual(base["evidence"], changed["evidence"], field)
            self.assertNotEqual(base["experience_table_revision"], changed["experience_table_revision"], field)
            self.assertNotEqual(base["policy_revision"], changed["policy_revision"], field)
        d = docs()
        d["death"]["rounding"] = "ceil"
        d["death"]["revision"] = lib.document_revision("death", d["death"])
        self.assertNotEqual(base["death_policy_revision"], lib.revisions(d)["death_policy_revision"])

    def test_all_valid_revision_syntax(self):
        import re
        for value in lib.revisions(docs()).values():
            self.assertTrue(re.fullmatch(r"[A-Za-z0-9][A-Za-z0-9._:-]{0,127}", value), value)

    def test_verify_refuses_stale_revision(self):
        d = docs()
        d["table"]["levels"][3]["minimum_experience"] = "1"
        with self.assertRaises(lib.ProgressionError):
            lib.verify_documents(d)

    def test_section_is_canonical(self):
        section = lib.build_section(docs())
        raw = lib.canonical(section)
        self.assertEqual(lib.canonical(lib.strict_loads(raw)), raw)
        self.assertLessEqual(len(raw), 256 * 1024)


class Canonical(unittest.TestCase):
    def test_vectors(self):
        vectors = json.loads((HERE / "canonical-vectors.json").read_text("utf-8"))
        for case in vectors["cases"]:
            self.assertEqual(lib.canonical(lib.strict_loads(case["input"].encode())).decode(), case["canonical"])

    def test_refusals(self):
        for bad in (
            '{"a":1,"a":2}', '{"a":{"b":1,"b":1}}', '{"a":1.5}', '{"a":1e2}', '{"a":-1}',
            '{"a":null}', '{"é":1}', '{"a":9007199254740992}', '{"a":"\\ud800"}',
        ):
            with self.assertRaises(lib.ProgressionError, msg=bad):
                lib.strict_loads(bad.encode())
        lib.strict_loads(b'{"a":9007199254740991}')


class Committed(unittest.TestCase):
    def test_evidence_last_level_matches(self):
        ev = lib.strict_loads(lib.PATHS["evidence"].read_bytes())
        self.assertEqual(ev["last_level"], lib.EXPERIENCE_EVIDENCE_LAST_LEVEL)
        lib.verify_levels_hash(ev)

    def test_documents_verify_and_table_coverage(self):
        loaded = lib.load_docs()
        lib.verify_documents(loaded)
        self.assertEqual(
            loaded["table"]["evidence_coverage"],
            {"first_level": 1, "last_level": lib.EXPERIENCE_EVIDENCE_LAST_LEVEL},
        )

    def test_readme_is_not_digest_input(self):
        ev = lib.strict_loads(lib.PATHS["evidence"].read_bytes())
        self.assertNotIn("revision", ev)
        self.assertEqual(lib.evidence_revision(ev), lib.evidence_revision(copy.deepcopy(ev)))

    def test_only_evidence_and_readme_present(self):
        names = sorted(p.name for p in lib.EVIDENCE_DIR.iterdir())
        self.assertEqual(names, ["README.md", "evidence.json"])


if __name__ == "__main__":
    unittest.main()
