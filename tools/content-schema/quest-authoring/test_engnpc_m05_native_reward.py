"""The shipped Gnomus report must publish its physical reward candidate."""
from pathlib import Path
import unittest

ROOT = Path(__file__).resolve().parents[3]

class M05NativeRewardTests(unittest.TestCase):
    def test_gnomus_reward_is_a_reviewed_physical_qualification_input(self):
        import sys
        directory = ROOT / 'tools/content-migration'
        sys.path.insert(0, str(directory))
        try:
            import quest_batch3_item_profiles as profiles
            self.assertIn('content/dialogues/quest-rules/quest-batch3-qn3-dangerous_depths.json', profiles.EXTRA_INPUTS)
        finally:
            sys.path.remove(str(directory))

if __name__ == '__main__':
    unittest.main()
