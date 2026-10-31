import json
from pathlib import Path
import unittest

import spellbook_item_profiles as spellbooks

ROOT = Path(__file__).resolve().parents[2]
PROFILES = 'content/items/definitions/spell-native-profiles.json'
MANIFEST = 'content/spells.manifest.json'
ADMITTED = {'oteryn:item.tibia.i3059', 'oteryn:item.tibia.i8073'}


def key(row):
    return row['production_definition']['production_key']


class SpellbookItemProfilesTest(unittest.TestCase):
    def generated(self):
        document = json.loads((ROOT / PROFILES).read_bytes())
        document['records'] = [r for r in document['records'] if key(r) not in ADMITTED]
        return {PROFILES: json.dumps(document).encode(), MANIFEST: (ROOT / MANIFEST).read_bytes()}

    def admit(self, generated):
        spellbooks.admit(ROOT, generated, PROFILES, MANIFEST,
                         lambda r: int(key(r).split('.i')[-1]),
                         lambda value: (json.dumps(value, sort_keys=True) + '\n').encode())

    def test_only_materializable_crystal_spellbooks_are_admitted_once(self):
        generated = self.generated()
        before = {key(r) for r in json.loads(generated[PROFILES])['records']}
        self.admit(generated)
        records = json.loads(generated[PROFILES])['records']
        added = [r for r in records if key(r) not in before]
        self.assertEqual({key(r) for r in added}, ADMITTED)
        for row in added:
            self.assertEqual(row['attributes'], {'speed_bonus': 0})
            self.assertEqual(row['admission']['stack_class'], 'NonStackable')
            self.assertEqual(row['production_binding']['disposition'], 'EXACT')
        # A second pass over its own output admits nothing new.
        again = dict(generated)
        self.admit(again)
        self.assertEqual(again[PROFILES], generated[PROFILES])

    def test_checked_in_profiles_carry_the_admitted_spellbooks(self):
        keys = {key(r) for r in json.loads((ROOT / PROFILES).read_bytes())['records']}
        self.assertTrue(ADMITTED | {'oteryn:item.tibia.i6120'} <= keys)


if __name__ == '__main__':
    unittest.main()
