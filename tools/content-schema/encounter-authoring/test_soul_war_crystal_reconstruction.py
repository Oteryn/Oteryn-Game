import json
import unittest
from pathlib import Path

import validate_encounter as validator


class SoulWarCrystalReconstructionTests(unittest.TestCase):
    @classmethod
    def setUpClass(cls):
        cls.root = Path(__file__).resolve().parent
        cls.sample_root = cls.root / "samples" / "soul-war-crystal-reconstruction"
        cls.index = json.loads((cls.sample_root / "index.json").read_text(encoding="utf-8"))

    def test_index_is_closed_and_non_runtime(self):
        self.assertFalse(self.index["runtime_qualified"])
        self.assertEqual(
            self.index["encounters"],
            [
                "claustrophobic_inferno_raid_1",
                "claustrophobic_inferno_raid_2",
                "claustrophobic_inferno_raid_3",
                "goshnars_greed",
                "goshnars_malice",
                "goshnars_spite",
                "goshnars_cruelty",
                "goshnars_hatred",
                "goshnars_megalomania",
            ],
        )
        self.assertEqual(
            self.index["source"]["revision"],
            "00ce02a57ca5a12e48f32a3476e37471167e4c3f",
        )

    def test_every_bundle_validates_and_keeps_hard_holds(self):
        for name in self.index["encounters"]:
            with self.subTest(name=name):
                base = self.sample_root / name
                encounter = json.loads((base / "encounter.json").read_text(encoding="utf-8"))
                catalog = json.loads((base / "catalog.json").read_text(encoding="utf-8"))
                manifest = json.loads((base / "manifest.json").read_text(encoding="utf-8"))
                self.assertEqual(validator.validate(encounter, catalog, manifest), [])
                self.assertEqual(manifest["classification"], "SOURCE_BACKED_OTERYN_RECONSTRUCTION")
                self.assertEqual(
                    {src["revision"] for src in manifest["sources"]},
                    {"00ce02a57ca5a12e48f32a3476e37471167e4c3f"},
                )
                self.assertTrue(manifest["entries"])

    def test_known_unrepresentable_mechanics_stay_explicit(self):
        expected = {
            "claustrophobic_inferno_raid_1": ("players", "cooldown"),
            "claustrophobic_inferno_raid_2": ("players", "bare 'Pos'"),
            "claustrophobic_inferno_raid_3": ("players", "cooldown"),
            "goshnars_malice": ("white", "reflect"),
            "goshnars_greed": ("Soul Sphere", "Soulsnatcher"),
            "goshnars_spite": ("per-player", "Weeping Soul"),
            "goshnars_cruelty": ("per-player", "transform elemental"),
            "goshnars_hatred": ("per-player", "Condensed Remorse"),
            "goshnars_megalomania": ("player", "white"),
        }
        for name, needles in expected.items():
            with self.subTest(name=name):
                manifest = json.loads(
                    (self.sample_root / name / "manifest.json").read_text(encoding="utf-8")
                )
                unresolved = "\n".join(
                    row["resolution"]
                    for row in manifest["entries"]
                    if row["status"] == "unresolved_semantics"
                )
                self.assertTrue(unresolved, f"{name} unexpectedly lost all hard holds")
                for needle in needles:
                    self.assertIn(needle.lower(), unresolved.lower())


if __name__ == "__main__":
    unittest.main()
