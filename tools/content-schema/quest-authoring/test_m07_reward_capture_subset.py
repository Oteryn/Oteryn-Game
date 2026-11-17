import sys
import unittest
from pathlib import Path

ROOT = Path(__file__).resolve().parents[3]
sys.path.insert(0, str(ROOT / 'tools/content-migration'))
import qengine_reward_item_profiles as profiles

class RewardProfileSubsetTests(unittest.TestCase):
    def test_explicit_capture_subset_is_not_filtered_by_default_cohort(self):
        packet = profiles.capture(ROOT, ids={2833})
        self.assertEqual({r['production_definition']['production_key'] for r in packet['records']}, {'oteryn:item.tibia.i2833'})
        self.assertEqual({r['production_binding']['external_id'] for r in packet['records']}, {'2833'})

if __name__ == '__main__':
    unittest.main()
