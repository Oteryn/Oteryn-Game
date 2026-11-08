"""Native-only claims remain mandatory in the exact generated inventory."""
from pathlib import Path
import unittest

from questl2_claim_inventory import expected_keys, normalized_keys


class ClaimInventory(unittest.TestCase):
    def test_both_registered_sources_include_sams_armor(self):
        root = Path(__file__).resolve().parents[2]
        keys = expected_keys(root)
        self.assertEqual(len(keys), 337)
        self.assertIn('oteryn:reward-claim.quest.u7_5.sams_old_backpack.armor', keys)

    def test_duplicate_and_unsupported_shapes_are_refused(self):
        claim = {'identity': {'key': 'crystalserver:reward-claim/a/b'},
                 'claim': {'per': 'character', 'repeat': {'kind': 'once'}}}
        with self.assertRaisesRegex(ValueError, 'Duplicate'):
            normalized_keys([claim, claim])
        claim['claim']['per'] = 'world'
        with self.assertRaisesRegex(ValueError, 'Unsupported'):
            normalized_keys([claim])


if __name__ == '__main__':
    unittest.main()
