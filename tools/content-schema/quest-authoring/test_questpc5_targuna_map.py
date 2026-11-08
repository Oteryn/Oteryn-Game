import unittest
import questpc5_targuna_map as source

class SterlingMapBinding(unittest.TestCase):
    def test_exact_source_bytes_and_emitted_documents(self):
        source.source_bytes()
        source.write(check=True)
    def test_source_progress_and_map_fact_are_one_callback_without_item_charge(self):
        row=source.DOCUMENTS["content/quests/npc-callbacks/questpc5-targuna-map.json"]["callbacks"][0]
        self.assertEqual(row["topic"],2)
        self.assertEqual(row["map_mark"],{"position":{"x":31948,"y":31924,"floor":-7},"kind":"cross","text":"Treasure"})
        self.assertEqual(row["take"],[])
        self.assertIsNone(row["give"])
        self.assertEqual([(s["minimum"],s["maximum"],s["set"]) for s in row["storage"]],[(3,3,4),(0,0,1)])
        self.assertTrue(row["requires"][0]["track"].endswith("morla_done"))
        self.assertEqual(row["requires"][0]["minimum"],1)
        self.assertEqual(row["requires"][0]["maximum"],1)
    def test_mission_offer_opens_topic_two_and_no_morla_refusal_resets_it(self):
        rows=source.DOCUMENTS["content/quests/npc-offers/questpc5-targuna-map.json"]["offers"]
        self.assertEqual((rows[0]["keywords"],rows[0]["next_topic"]),(["mission"],2))
        self.assertTrue(rows[0]["storage"][0]["preserve"])
        self.assertEqual((rows[1]["topic"],rows[1]["next_topic"]),(2,0))
        self.assertEqual(rows[1]["storage"][0]["maximum"],0)

if __name__=="__main__": unittest.main()
