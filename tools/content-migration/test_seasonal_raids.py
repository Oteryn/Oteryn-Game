import copy
import hashlib
import json
from pathlib import Path
import unittest

import raid_catalogue as raids
import seasonal_raids as seasonal

ROOT = Path(__file__).resolve().parents[2]


class SeasonalRaidTest(unittest.TestCase):
    def test_cake_source_projects_twenty_independent_golems_in_each_city(self):
        record, = seasonal.records(ROOT, raids.qualify, raids.bind_area, raids.citation,
                                  {"oteryn:creature.cake_golem": False}, raids.load(ROOT, raids.PLACEMENTS))
        self.assertTrue(record["active"], record["inactive_reasons"])
        entries = record["variants"][0]["waves"][0]["entries"]
        self.assertEqual(len(entries), 100)
        self.assertEqual(len({entry["area_anchor"] for entry in entries}), 100)
        expected = {"carlin": {(32381, 31785), (32310, 31789)},
                    "thais": {(32324, 32216), (32390, 32214)},
                    "edron": {(33160, 31824), (33222, 31880)},
                    "darashia": {(33264, 32446), (33195, 32463)},
                    "liberty_bay": {(32317, 32835), (32288, 32871)}}
        for city, centers in expected.items():
            selected = [entry for entry in entries if f".{city}." in entry["area_anchor"]]
            self.assertEqual(len(selected), 20)
            for entry in selected:
                self.assertEqual(entry["amount"], {"min": 1, "max": 1})
                self.assertEqual({((area["from"]["x"]+area["to"]["x"])//2,
                                   (area["from"]["y"]+area["to"]["y"])//2)
                                  for area in entry["area_choices"]}, centers)
                for area in entry["area_choices"]:
                    self.assertEqual((area["to"]["x"]-area["from"]["x"], area["to"]["y"]-area["from"]["y"]), (6, 6))
                    self.assertTrue(area["regions"])
        self.assertEqual(record["schedule"]["daily_slot_minutes"], list(range(4, 1440, 60)))

    def test_missing_canonical_creature_never_activates_native_profile_only(self):
        record, = seasonal.records(ROOT, raids.qualify, raids.bind_area, raids.citation, {}, raids.load(ROOT, raids.PLACEMENTS))
        self.assertFalse(record["active"])
        self.assertEqual(record["inactive_reasons"], ["creature_unbound:Cake Golem"])

    def test_captured_source_coordinates_cannot_be_changed_with_new_self_hashes(self):
        document = json.loads((ROOT / seasonal.CAPTURE).read_bytes())
        altered = copy.deepcopy(document)
        altered["files"][0]["text"] = altered["files"][0]["text"].replace("32381", "32382")
        raw = altered["files"][0]["text"].encode()
        altered["files"][0]["sha256"] = hashlib.sha256(raw).hexdigest()
        altered["files"][0]["git_blob"] = hashlib.sha1(b"blob " + str(len(raw)).encode() + b"\0" + raw).hexdigest()
        with self.assertRaisesRegex(ValueError, "SEASONAL_RAID_SOURCE_HASH"):
            seasonal.verify(altered)

if __name__ == "__main__":
    unittest.main()
