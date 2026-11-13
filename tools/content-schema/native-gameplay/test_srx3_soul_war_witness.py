"""The real witness generator cannot bless substituted callback participants."""
import copy
import importlib.util
import json
from pathlib import Path
import tempfile
import unittest
from unittest.mock import patch

SPEC = importlib.util.spec_from_file_location(
    "srx3_soul_war_witness", Path(__file__).with_name("build_srx3_soul_war_witness.py"))
TOOL = importlib.util.module_from_spec(SPEC)
SPEC.loader.exec_module(TOOL)


class SoulWarWitnessTests(unittest.TestCase):
    @classmethod
    def setUpClass(cls):
        cls.witness = json.loads(TOOL.OUTPUT.read_bytes())

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
                return TOOL.build()

    def test_actual_bounded_source_closure_and_full_source_digest_are_retained(self):
        self.assertEqual(self.build(), self.witness)

    def test_all_effective_fourth_taint_participants_have_exact_native_bodies(self):
        # Literal final monster.events assignments; Mirror Image overwrites its
        # earlier FourthTaint event and is intentionally not in this set.
        names = ("bony_sea_devil", "brachiodemon", "branchy_crawler",
                 "capricious_phantom", "distorted_phantom", "cloak_of_terror",
                 "courage_leech", "vibrant_phantom", "infernal_demon",
                 "infernal_phantom", "many_faces", "mould_phantom",
                 "rotten_golem", "turbulent_elemental")
        actual = {row["profile"]["target"]["key"] for row in self.witness["records"]}
        self.assertTrue({"oteryn:creature." + name for name in names} <= actual)

    def test_remaining_encounter_bodies_preserve_exact_cross_donor_identity(self):
        expected = {"oteryn:creature." + slug for slug in TOOL.EXTERNAL_IDS}
        actual = {row["profile"]["target"]["key"] for row in self.witness["records"]}
        self.assertTrue(expected <= actual)
        phantom = next(row for row in self.witness["source_bindings"]
                       if row["target"]["key"] == "oteryn:creature.hazardous_phantom")
        self.assertEqual(phantom["external_id"], "undeads/hazardous_phantom")
        self.assertIn("data-global/monster/quests/soul_war/hazardous_phantom.lua",
                      self.witness["callbacks"])

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
