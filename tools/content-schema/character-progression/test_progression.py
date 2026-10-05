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


def evidence(last=K, **over):
    doc = lib.build_evidence(
        last, "https://example.invalid/table", "2026-10-05", "test",
        hashlib.sha256(lib.levels_csv(last)).hexdigest(),
    )
    doc.update(over)
    return doc


def docs(ev=None):
    ev = ev or evidence()
    return {
        "evidence": ev,
        "table": lib.build_table(ev),
        "death": lib.build_death(),
        "reward": lib.build_reward(),
        "differences": lib.build_differences(ev["last_level"]),
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


class Capture(unittest.TestCase):
    def good(self):
        return lib.levels_csv(K)

    def refused(self, data):
        with self.assertRaises(lib.ProgressionError):
            lib.check_capture(data, K)

    def test_good(self):
        self.assertTrue(lib.check_capture(self.good(), K))

    def test_mutations(self):
        text = self.good().decode()
        rows = text[:-1].split("\n")
        self.refused(text.replace("\n", "\r\n").encode())
        self.refused(("level,experience\n" + text).encode())
        self.refused(text[:-1].encode())
        self.refused(("\n".join(rows[1:]) + "\n").encode())
        self.refused(("\n".join(rows[:-1]) + "\n").encode())
        self.refused(("\n".join(rows[:5] + rows[6:]) + "\n").encode())
        self.refused(("\n".join(rows[:5] + [rows[4]] + rows[5:]) + "\n").encode())
        self.refused(("\n".join(rows[:5] + [rows[6], rows[5]] + rows[7:]) + "\n").encode())
        self.refused(("\n".join(["0,0"] + rows[1:]) + "\n").encode())

    def test_single_value_changes_hash(self):
        rows = self.good().decode()[:-1].split("\n")
        level, value = rows[10].split(",")
        rows[10] = f"{level},{int(value) + 1}"
        mutated = ("\n".join(rows) + "\n").encode()
        lib.check_capture(mutated, K)
        self.assertNotEqual(hashlib.sha256(mutated).hexdigest(), evidence()["levels_sha256"])

    def test_capture_file(self):
        with tempfile.TemporaryDirectory() as tmp:
            path = Path(tmp) / "c.csv"
            path.write_bytes(self.good())
            lib.verify_capture_file(path, evidence())
            path.write_bytes(self.good()[:-1])
            with self.assertRaises(lib.ProgressionError):
                lib.verify_capture_file(path, evidence())


class Revisions(unittest.TestCase):
    def test_changes_propagate(self):
        base = lib.revisions(docs())
        for field in ("source_url", "captured_on", "extraction_method"):
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

    def test_only_evidence_and_readme_tracked(self):
        out = subprocess.run(
            ["git", "ls-files", "docs/reference/experience-table-20261005"],
            cwd=lib.ROOT, capture_output=True, text=True, check=True,
        ).stdout.split()
        self.assertEqual(
            sorted(out),
            ["docs/reference/experience-table-20261005/README.md",
             "docs/reference/experience-table-20261005/evidence.json"],
        )


if __name__ == "__main__":
    unittest.main()
