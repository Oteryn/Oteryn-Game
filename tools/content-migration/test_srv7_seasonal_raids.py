"""W05d real generated raid catalogue admission at the existing scheduler boundary."""
import json
import unittest

import raid_catalogue as raids


class SeasonalRaidTest(unittest.TestCase):
    def test_halloween_hare_has_one_source_bound_wave_on_october_31(self):
        outputs = raids.outputs(raids.ROOT, register=False)
        records = json.loads(outputs[raids.CATALOGUE])["records"]
        hare = next(row for row in records if row["key"] == "oteryn:raid.kazordoon_raids.the_halloween_hare")
        self.assertTrue(hare["active"], hare["inactive_reasons"])
        self.assertEqual(hare["schedule"]["interval_ms"], 86_400_000)
        self.assertEqual(hare["schedule"]["window"], {
            "start_month": 10, "start_day": 31, "end_month": 10, "end_day": 31})
        self.assertEqual(len(hare["variants"]), 1)
        variant = hare["variants"][0]
        self.assertEqual(variant["announcements"][0]["text"], "Beware, beware the halloween hare.")
        self.assertEqual(len(variant["waves"]), 1)
        entry = variant["waves"][0]["entries"][0]
        self.assertEqual(entry["creature"], "oteryn:creature.the_halloween_hare")
        self.assertEqual(entry["amount"], {"min": 1, "max": 1})
        self.assertEqual(entry["area"]["from"], {"x": 32624, "y": 32018, "floor": 7})
        self.assertEqual(entry["area"]["to"], entry["area"]["from"])
        self.assertTrue(entry["area"]["regions"])
        self.assertEqual(hare["end"]["kind"], "until_reset")


if __name__ == "__main__":
    unittest.main()
