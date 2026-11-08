"""M18's donor exit carpet must have real native movement facts."""
import json
from pathlib import Path
import unittest
ROOT = Path(__file__).resolve().parents[2]

class FirstDragonItemProfileTest(unittest.TestCase):
    def test_source_exit_carpet_is_nonblocking_ground_only(self):
        rows = json.loads((ROOT / 'content/items/definitions/spell-native-profiles.json').read_bytes())['records']
        matches = [r for r in rows if r['production_definition']['production_key'] == 'oteryn:item.tibia.i17400']
        self.assertEqual(len(matches), 1, 'M18 source exit carpet has no native collision profile')
        row = matches[0]
        self.assertFalse(row['attributes']['blocks_movement'])
        self.assertFalse(row['attributes']['blocks_projectile'])
        self.assertEqual(row['admission']['legal_destinations'], ['Ground'])
        self.assertEqual(row['semantics']['physical']['value']['pickupable'], {'state':'KNOWN','value':False})

    def test_source_corpse_decay_closure_is_native(self):
        rows = json.loads((ROOT / 'content/items/definitions/spell-native-profiles.json').read_bytes())['records']
        keys = {r['production_definition']['production_key'] for r in rows}
        for item in (25065, 25066, 25067):
            self.assertIn(f'oteryn:item.tibia.i{item}', keys, 'First Dragon corpse decay is not native')

if __name__ == '__main__':
    unittest.main()
