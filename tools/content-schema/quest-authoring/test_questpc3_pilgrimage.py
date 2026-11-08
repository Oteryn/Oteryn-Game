import json
import unittest

import questpc3_pilgrimage as authoring


class PilgrimageAuthoringTests(unittest.TestCase):
    def test_route_grants_exact_blessings_and_once_only_completion(self):
        documents = authoring.documents()
        callbacks = documents["content/quests/shared-reward-callbacks.json"]["callbacks"]
        owned = {c["key"].removeprefix(authoring.PREFIX): c for c in callbacks
                 if c["key"].startswith(authoring.PREFIX)}
        self.assertEqual(len(owned), 30)
        for stage, (npc, blessing) in enumerate(authoring.VISITS, 1):
            callback = owned[npc]
            self.assertEqual(callback["storage"][0], {"track": authoring.STAGE,
                             "minimum": stage, "maximum": stage, "set": stage + 1,
                             "preserve": False})
            self.assertEqual(callback["blessing"],
                             "oteryn:service.blessing." + blessing if blessing else None)
            self.assertTrue(callback["premium"])
            self.assertEqual(callback["take"], [])
            self.assertIsNone(callback["give"])
        for guide in authoring.GUIDES:
            final = owned["guide_" + guide + "-finish"]
            self.assertEqual(final["give"], {"item": "oteryn:item.tibia.i3035", "count": 20})
            self.assertTrue(final["completes"])
            self.assertEqual(final["storage"][0]["minimum"], 7)
            self.assertEqual(final["storage"][0]["set"], 8)

    def test_preserves_other_programs_and_declares_four_regular_services(self):
        documents = authoring.documents()
        existing = json.loads((authoring.ROOT / "content/quests/shared-reward-callbacks.json").read_text())
        foreign = [c for c in existing["callbacks"] if not c["key"].startswith(authoring.PREFIX)]
        output = documents["content/quests/shared-reward-callbacks.json"]["callbacks"]
        self.assertEqual([c for c in output if not c["key"].startswith(authoring.PREFIX)], foreign)
        index = documents["content/services/blessings/index.json"]
        records = documents[index["shards"][0]]["records"]
        keys = {r["declaration"]["identity"]["key"] for r in records}
        for stem in ("the_spiritual_shielding", "the_embrace_of_tibia",
                     "the_fire_of_the_suns", "the_spark_of_the_phoenix"):
            self.assertIn("oteryn:service.blessing." + stem, keys)
        self.assertIn("oteryn:service.blessing.henricus", keys)
        self.assertEqual(len(keys), 10)
        before = json.loads((authoring.ROOT / index['shards'][0]).read_text())['records']
        self.assertTrue({r['declaration']['identity']['key'] for r in before} <= keys)


if __name__ == "__main__":
    unittest.main()
