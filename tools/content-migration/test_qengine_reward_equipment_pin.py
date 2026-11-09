import copy
import json
import unittest
from pathlib import Path

from qengine_reward_equipment_pin import refresh


class SourceCohortTests(unittest.TestCase):
    def setUp(self):
        self.packet = json.loads((Path(__file__).resolve().parents[1] /
            "content-schema/native-gameplay/qengine-reward-item-profiles.json").read_bytes())
        self.source = "assert_eq!(added_keys.len(), 60);"

    def test_regenerates_stale_cardinality_from_the_producer_cohort(self):
        self.assertEqual("assert_eq!(added_keys.len(), 63);", refresh(self.source, self.packet))

    def test_refuses_a_missing_source_identity(self):
        self.packet["records"].pop()
        with self.assertRaisesRegex(ValueError, "SOURCE_COHORT"):
            refresh(self.source, self.packet)

    def test_refuses_an_extra_or_duplicated_identity(self):
        self.packet["records"].append(copy.deepcopy(self.packet["records"][0]))
        with self.assertRaisesRegex(ValueError, "SOURCE_COHORT"):
            refresh(self.source, self.packet)

    def test_refuses_a_changed_source_pin(self):
        self.packet["sources"][0]["sha256"] = "0" * 64
        with self.assertRaisesRegex(ValueError, "SOURCE_COHORT"):
            refresh(self.source, self.packet)


if __name__ == "__main__":
    unittest.main()
