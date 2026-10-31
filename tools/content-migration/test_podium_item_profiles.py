"""W18: the active provider exposes the five house podiums as placeable furniture."""
import json
from pathlib import Path
import unittest
import podium_item_profiles as podiums

ROOT = Path(__file__).resolve().parents[2]


class PodiumItemProfiles(unittest.TestCase):
    @classmethod
    def setUpClass(cls):
        rows = json.loads((ROOT / 'content/items/definitions/spell-native-profiles.json').read_bytes())['records']
        cls.profiles = {r['production_definition']['production_key']: r for r in rows}

    def test_podiums_are_materializable_ground_furniture(self):
        self.assertEqual(sorted(podiums.IDS), [35973, 35974, 38707, 42367, 42368])
        for number in sorted(podiums.IDS):
            key = f'oteryn:item.tibia.i{number}'
            row = self.profiles[key]
            self.assertEqual(row['production_binding']['disposition'], 'EXACT')
            self.assertTrue(row['admission']['materializable'])
            self.assertEqual(row['admission']['stack_class'], 'NonStackable')
            self.assertEqual(row['admission']['legal_destinations'], ['Ground'])
            physical = row['semantics']['physical']['value']
            self.assertEqual(physical['pickupable'], podiums.known(False))
            self.assertEqual(physical['movable'], podiums.known(True))


if __name__ == '__main__':
    unittest.main()
