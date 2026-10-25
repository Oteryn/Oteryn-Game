"""Qualification and preservation of the closed READ-WRITE-1a Item owner."""
import copy
import json
import unittest

import readable_item_profiles as reading
import register_spell_families as registration


class ReadableProfilesTests(unittest.TestCase):
    def test_pinned_xml_qualifies_signs_but_not_flags_or_goblets(self):
        facts = reading.source_facts((reading.ROOT / reading.SOURCE).read_bytes())
        self.assertEqual(set(facts), reading.IDS)
        self.assertFalse(set(facts) & {2018, 2019, 2020, 7369, 7370, 7371})
        for row in facts.values():
            self.assertIs(row["facts"]["distance_read"], True)
        self.assertEqual(facts[2598]["facts"], {
            "distance_read": True, "writeable": True, "readable": True, "max_text_length": 1023})
        self.assertEqual(facts[2012]["facts"], {"distance_read": True})
        with self.assertRaisesRegex(ValueError, "READABLE_SOURCE_DIGEST"):
            reading.source_facts(b"<items/>")

    def test_native_generation_preserves_admission_and_all_unowned_facts(self):
        rows = reading.item_rows(reading.ROOT)
        document = json.loads((reading.ROOT / "content/items/definitions/spell-native-profiles.json").read_bytes())
        # Reapplying to the generated native provider is idempotent and does not
        # create a materializable policy, even for the writeable blackboards.
        generated = {"profiles": json.dumps(document).encode(), "manifest": b'{"item_profiles":{}}'}
        before = copy.deepcopy(document)
        reading.apply_native(reading.ROOT, generated, "profiles", "manifest", registration.provider_order, reading.encode)
        self.assertEqual(json.loads(generated["profiles"]), before)
        by_key = {r["production_definition"]["production_key"]: r for r in document["records"]}
        for item_id in reading.IDS:
            key = f"oteryn:item.tibia.i{item_id}"
            self.assertIs(by_key[key]["admission"]["materializable"], False)
            self.assertIs(rows[key]["materializable"], False)
            source = rows[key].get("semantics", {})
            for field, value in source.items():
                if field != "readable_writeable":
                    self.assertEqual(by_key[key]["semantics"][field], value)
        manifest = json.loads((reading.ROOT / "content/manifest.json").read_bytes())
        self.assertIn({"path": reading.PATH}, manifest["managed_files"])

    def test_leaf_conflict_is_refused(self):
        document = json.loads((reading.ROOT / "content/items/definitions/spell-native-profiles.json").read_bytes())
        target = next(r for r in document["records"] if r["production_definition"]["production_key"] == "oteryn:item.tibia.i2012")
        target["semantics"]["readable_writeable"]["value"]["distance_read"]["value"] = False
        generated = {"profiles": json.dumps(document).encode(), "manifest": b'{"item_profiles":{}}'}
        original = generated.copy()
        with self.assertRaisesRegex(ValueError, "READABLE_LEAF_CONFLICT"):
            reading.apply_native(reading.ROOT, generated, "profiles", "manifest", registration.provider_order, reading.encode)
        self.assertEqual(generated, original)


if __name__ == "__main__":
    unittest.main()
