import copy
import unittest

from questpc4_stag_equipment_pin import refresh


class StagEquipmentPinTests(unittest.TestCase):
    def packet(self):
        return {
            "schema": "OTERYN_QUESTPC4_STAG_ITEM_PROFILES/v3",
            "producer": "tools/content-migration/build_questpc4_stag_item_profiles.py",
            "records": [
                {"production_definition": {"production_key": f"oteryn:item.tibia.i{i}"}}
                for i in (52586, 52635, 52743, 52758, 52759, 52824)
            ],
        }

    def test_exact_source_cohort_refresh_is_idempotent(self):
        result = refresh('    ("QUESTPC4-STAG-ITEMS", 1),', self.packet())
        self.assertEqual(result, '    ("QUESTPC4-STAG-ITEMS", 6),')
        self.assertEqual(refresh(result, self.packet()), result)

    def test_missing_duplicate_or_unrelated_source_identity_rejected(self):
        original = self.packet()
        for records in (
            original["records"][:-1],
            [original["records"][0]] * len(original["records"]),
            [{"production_definition": {"production_key": "oteryn:item.tibia.i1"}}]
            + original["records"][1:],
        ):
            packet = copy.deepcopy(original)
            packet["records"] = records
            with self.assertRaisesRegex(ValueError, "SOURCE_COHORT"):
                refresh('("QUESTPC4-STAG-ITEMS", 1),', packet)

    def test_ambiguous_pin_selector_rejected(self):
        for source in ("", '("QUESTPC4-STAG-ITEMS", 1),' * 2):
            with self.assertRaisesRegex(ValueError, "PIN_SELECTOR"):
                refresh(source, self.packet())


if __name__ == "__main__":
    unittest.main()
