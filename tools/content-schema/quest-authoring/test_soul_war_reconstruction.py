import copy
import json
import unittest
from pathlib import Path

import quest_soul_war_followup as followup
import soul_war_reconstruction as reconstruction


class SoulWarReconstructionTests(unittest.TestCase):
    @classmethod
    def setUpClass(cls):
        cls.root = Path(__file__).resolve().parents[3]
        packet = json.loads(
            (cls.root / "tools/content-schema/quest-authoring/samples/completion242/recipes.json")
            .read_text(encoding="utf-8")
        )
        cls.recipe_row = next(
            row for row in packet["records"]
            if row["identity"]["key"] == "oteryn:quest.soul_war_quest"
        )

    def wrapper(self):
        return {
            "definition": {
                "identity": {"key": "oteryn:quest.soul_war_quest", "revision": "quest-r1"},
                "native_lowering": {"state": "WAITING_IMPLEMENTATION"},
                "oteryn_recipe": {
                    "runtime_enabled": False,
                    "payload": copy.deepcopy(self.recipe_row),
                },
            }
        }

    def test_reconstruction_packet_is_closed_and_semantically_qualified(self):
        packet = reconstruction.validate(self.root)
        self.assertFalse(packet["runtime_activation"])
        self.assertEqual(packet["source_policy"]["primary_revision"], reconstruction.CRYSTAL)
        self.assertEqual({row["part"] for row in packet["video_sources"]}, set(range(1, 7)))
        mechanics = {row["key"]: row for row in packet["mechanics"]}
        self.assertEqual(mechanics["claustrophobic_inferno_three_raids"]["classification"], "CONFLICT")
        self.assertEqual(mechanics["spite_soul_fire"]["classification"], "CONFLICT")
        self.assertEqual(mechanics["malice_white_safe_tiles"]["chosen"]["unsafe_tile_damage"], 8000)
        self.assertEqual(mechanics["megalomania_aspect_vulnerability"]["chosen"]["aspect_kills_required"], 4)

    def test_followup_replaces_false_fixed_order_without_mutating_source_input(self):
        original = self.wrapper()
        records = [original]
        updated, provenance = followup.apply(self.root, records)
        self.assertEqual(records, [original])
        self.assertEqual(provenance["sha256"], followup.SHA256)

        definition = updated[0]["definition"]
        payload = definition["oteryn_recipe"]["payload"]
        recipe = payload["recipe"]
        self.assertFalse(definition["oteryn_recipe"]["runtime_enabled"])
        self.assertEqual([stage["key"] for stage in recipe["stages"]], ["s1", "s2", "s3", "s4", "s5"])
        self.assertEqual(recipe["stages"][1]["count"], 5)
        self.assertEqual(
            set(recipe["stages"][1]["targets"]),
            {
                "Goshnar's Malice",
                "Goshnar's Greed",
                "Goshnar's Spite",
                "Goshnar's Cruelty",
                "Goshnar's Hatred",
            },
        )
        self.assertEqual(len(recipe["reward_intents"]), 2)
        self.assertEqual({row["kind"] for row in recipe["reward_intents"]}, {"item", "outfit"})
        self.assertEqual(recipe["stages"][0]["kind"], "talk")
        self.assertEqual(recipe["stages"][0]["targets"], ["Flickering Soul"])
        self.assertEqual(payload["title_stage_keys"]["Soul War Quest"], ["s1", "s2", "s3", "s4", "s5"])
        self.assertEqual(payload["donor_source_refs"], reconstruction.validate(self.root)["source_files"])

    def test_followup_rejects_stale_recipe(self):
        record = self.wrapper()
        record["definition"]["oteryn_recipe"]["payload"]["recipe"]["stages"][0]["count"] += 1
        with self.assertRaisesRegex(ValueError, "recipe fence"):
            followup.apply(self.root, [record])

    def test_followup_rejects_runtime_promotion(self):
        record = self.wrapper()
        record["definition"]["oteryn_recipe"]["runtime_enabled"] = True
        with self.assertRaisesRegex(ValueError, "missing or runtime-enabled"):
            followup.apply(self.root, [record])

    def test_followup_refuses_preexisting_donor_replacement(self):
        record = self.wrapper()
        record["definition"]["oteryn_recipe"]["payload"]["donor_source_refs"] = [
            {"kind": "git", "repository": "zimbadev/crystalserver", "revision": "0" * 40,
             "path": "wrong", "blob_sha1": "0" * 40}
        ]
        with self.assertRaisesRegex(ValueError, "already has donor refs"):
            followup.apply(self.root, [record])


if __name__ == "__main__":
    unittest.main()
