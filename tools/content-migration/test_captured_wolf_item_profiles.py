import json
from pathlib import Path
import unittest

import captured_wolf_item_profiles as captured

ROOT = Path(__file__).resolve().parents[2]
PROFILES = 'content/items/definitions/spell-native-profiles.json'
MANIFEST = 'content/spells.manifest.json'


def key(row):
    return row['production_definition']['production_key']


class CapturedWolfItemProfilesTest(unittest.TestCase):
    def generated(self):
        document = json.loads((ROOT / PROFILES).read_bytes())
        document['records'] = [r for r in document['records'] if key(r) != captured.KEY]
        return {PROFILES: json.dumps(document).encode(), MANIFEST: (ROOT / MANIFEST).read_bytes()}

    def admit(self, generated):
        captured.admit(ROOT, generated, PROFILES, MANIFEST,
                       lambda r: int(key(r).split('.i')[-1]),
                       lambda value: (json.dumps(value, sort_keys=True) + '\n').encode())

    def test_the_captured_wolf_is_admitted_once_for_ground_and_inventory(self):
        generated = self.generated()
        self.admit(generated)
        rows = [r for r in json.loads(generated[PROFILES])['records'] if key(r) == captured.KEY]
        self.assertEqual(len(rows), 1)
        self.assertEqual(rows[0]['admission'], {'materializable': True, 'stack_class': 'NonStackable',
                                                'legal_destinations': ['CharacterInventory', 'Ground']})
        self.assertEqual(rows[0]['attributes'], {'speed_bonus': 0})
        self.assertEqual(rows[0]['production_binding']['disposition'], 'EXACT')
        again = dict(generated)
        self.admit(again)
        self.assertEqual(again[PROFILES], generated[PROFILES])

    def test_checked_in_profiles_carry_the_captured_wolf(self):
        keys = {key(r) for r in json.loads((ROOT / PROFILES).read_bytes())['records']}
        self.assertIn(captured.KEY, keys)


if __name__ == '__main__':
    unittest.main()
