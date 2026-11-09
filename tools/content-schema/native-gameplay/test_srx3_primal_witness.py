"""The real witness generator cannot bless substituted callback participants."""
import copy
import importlib.util
import json
from pathlib import Path
import tempfile
import unittest
from unittest.mock import patch

SPEC = importlib.util.spec_from_file_location(
    "srx3_primal_witness", Path(__file__).with_name("build_srx3_primal_witness.py"))
TOOL = importlib.util.module_from_spec(SPEC)
SPEC.loader.exec_module(TOOL)


class PrimalWitnessTests(unittest.TestCase):
    @classmethod
    def setUpClass(cls):
        cls.witness = json.loads(TOOL.OUTPUT.read_bytes())
        cls.documents = TOOL.canonical_witness.read_documents(TOOL.ROOT)

    def build(self, records=None, bindings=None, digest=None):
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            definitions = root / "content/creatures/definitions"
            definitions.mkdir(parents=True)
            (definitions / "spell-native-profiles.json").write_text(json.dumps({
                "records": records if records is not None else self.witness["records"],
                "source_definitions_sha256": digest or self.witness["source_definitions_sha256"],
            }))
            bindings = bindings if bindings is not None else self.witness["source_bindings"]
            (definitions / "creatures-00000.json").write_text(json.dumps({
                "records": [{"definition": {"identity": binding["target"]},
                             "source_bindings": [binding]} for binding in bindings],
            }))
            with patch.object(TOOL, "ROOT", root):
                return TOOL.build(documents=self.documents)

    def test_actual_bounded_source_closure_and_full_source_digest_are_retained(self):
        self.assertEqual(self.build(), self.witness)

    def test_actual_referenced_primal_encounter_is_retained_without_reconstruction(self):
        generated = TOOL.build()
        declarations = json.loads((TOOL.ROOT / "content/world/definitions/declarations.json").read_bytes())
        expected = next(row for row in declarations["authoring_profiles"]
                        if row["target"]["key"] == "oteryn:encounter.the_primal_menace")
        self.assertEqual(generated["encounter_profiles"], [expected])
        self.assertEqual(generated["records"], self.witness["records"],
                         "retaining actual metadata must not change any reviewed native participant")

    def test_rebound_full_source_digest_cannot_bless_changed_callback_body(self):
        records = copy.deepcopy(self.witness["records"])
        records[0]["profile"]["data"]["profile"]["health"] += 1
        with self.assertRaisesRegex(ValueError, "source qualification review required"):
            self.build(records=records, digest="1" * 64)

    def test_unchanged_names_cannot_substitute_donor_identity_or_revision(self):
        bindings = copy.deepcopy(self.witness["source_bindings"])
        bindings[0]["source_revision"] = "0" * 40
        with self.assertRaisesRegex(ValueError, "source binding changed"):
            self.build(bindings=bindings)

    def test_missing_or_duplicated_native_participant_is_refused(self):
        for records in (self.witness["records"][1:],
                        [*self.witness["records"], self.witness["records"][0]]):
            with self.subTest(count=len(records)):
                with self.assertRaisesRegex(ValueError, "missing or duplicated native participant"):
                    self.build(records=records)


if __name__ == "__main__":
    unittest.main()
