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

    def test_accepted_semantics_reject_numeric_and_gate_regressions(self):
        baseline = reconstruction.load(self.root, reconstruction.PACKET)
        for change in ("mirror_weight", "mirror_other", "hp_floor", "taint", "order", "gate", "reward", "shrines"):
            packet = copy.deepcopy(baseline)
            mechanics = {row["key"]: row for row in packet["mechanics"]}
            if change == "mirror_weight":
                mechanics["mirror_image"]["chosen"]["own_player_damage_transforms_to_attacker_base_vocation_percent"] = 60
            elif change == "mirror_other":
                mechanics["mirror_image"]["chosen"]["other_vocations_percent_each"] = 10
            elif change == "hp_floor":
                mechanics["mirror_image"]["chosen"]["other_damage_hp_floor"] = 0
            elif change == "taint":
                packet["progression"]["taint_effects"][0]["basis"] = "CRYSTAL_SOURCE"
            elif change == "order":
                packet["progression"]["mini_boss_order"] = "FIXED_LINEAR_ORDER"
            elif change == "gate":
                packet["progression"]["final_access"] = "any mini-boss flag"
            elif change == "reward":
                packet["progression"]["final_reward"] = "all finalRewards items"
            else:
                mechanics["rotten_wasteland_shrines"]["chosen"]["required_activated_shrines"] = 3
            with self.subTest(change=change), self.assertRaises(ValueError):
                reconstruction.validate_accepted_semantics(packet)
        reconstruction.validate_accepted_semantics(baseline)

    def test_followup_rejects_wrong_final_boss_outfit_and_progression(self):
        baseline = json.loads((self.root / followup.PATH).read_bytes())
        for change in ("start", "gate", "final_boss", "outfit", "reward_count", "order", "duplicate_boss"):
            packet = copy.deepcopy(baseline)
            stages = packet["replacement"]["stages"]
            rewards = packet["replacement"]["reward_intents"]
            if change == "start":
                stages[0]["targets"] = ["Soul Mender"]
            elif change == "gate":
                stages[2]["targets"] = ["unrelated entrance"]
            elif change == "final_boss":
                stages[3]["targets"] = ["Goshnar's Hatred"]
            elif change == "outfit":
                rewards[0]["name"] = "Wrong Outfit"
            elif change == "reward_count":
                rewards[1]["count"] = 18
            elif change == "order":
                stages[1]["next"] = ["s4"]
            else:
                stages[1]["targets"].append(stages[1]["targets"][0])
            with self.subTest(change=change), self.assertRaises(ValueError):
                followup._validate_packet(packet)
        followup._validate_packet(baseline)

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
