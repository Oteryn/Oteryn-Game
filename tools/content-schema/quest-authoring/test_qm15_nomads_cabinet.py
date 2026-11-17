import unittest

import qm15_nomads_cabinet as source


class NomadsCabinet(unittest.TestCase):
    def test_source_items_position_and_readable_child_stay_together(self):
        row = source.document()["callbacks"][0]
        self.assertEqual(row["selector"]["position"], {"x": 33220, "y": 32858, "floor": -9})
        self.assertEqual(row["selector"]["appearance"], 2434)
        reward = row["effects"]["item_reward"]
        self.assertEqual(reward["item"], "oteryn:item.tibia.i2853")
        self.assertEqual(reward["contents"], [
            {"item": "oteryn:item.tibia.i7527", "count": 1},
            {"item": "oteryn:item.tibia.i2833", "count": 1},
        ])
        self.assertEqual(reward["text"], {"contents": [{"index": 1, "text": source.TEXT}]})

    def test_quest_stage_and_once_only_claim_do_not_charge_an_item(self):
        row = source.document()["callbacks"][0]
        self.assertEqual(row["storage"], [
            {"track": source.CLAIMED, "minimum": 0, "maximum": 0, "set": 1},
            {"track": source.ARITO, "minimum": 2, "maximum": 2, "set": 2, "preserve": True},
        ])
        self.assertEqual(row["offerings"], [])
        self.assertTrue(row["requires_world_admission"])
        self.assertTrue(row["admission"]["premium"])
        self.assertTrue(any("#L803-L822" in ref for ref in row["sources"]))

    def test_literal_body_retains_both_source_paragraphs_within_native_bounds(self):
        self.assertTrue(source.TEXT.startswith("We are not far from revealing"))
        self.assertIn("\n\nSo far I found out", source.TEXT)
        self.assertTrue(source.TEXT.endswith("without being overrun by rotworms."))
        self.assertLessEqual(len(source.TEXT.encode()), 7996)
        self.assertLessEqual(len(source.TEXT), 4000)


if __name__ == "__main__":
    unittest.main()
