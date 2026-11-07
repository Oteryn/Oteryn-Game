"""Exercise Forge through the committed native provider, not synthetic profiles."""
import copy
import json
import unittest
from pathlib import Path

import forge_item_profiles as forge

ROOT = Path(__file__).resolve().parents[2]


class NativeForgeProfilesTests(unittest.TestCase):
    @classmethod
    def setUpClass(cls):
        cls.native = json.loads((ROOT / 'content/items/definitions/spell-native-profiles.json').read_bytes())
        cls.owners = forge.owners(ROOT)

    def test_playable_forge_uses_existing_admitted_item_classifications(self):
        records = {row['production_definition']['production_key']: row for row in self.native['records']}
        for source, classification, maximum in ((3264, 1, 1), (34082, 4, 10), (34084, 4, 10)):
            with self.subTest(source=source):
                self.assertEqual(records[f'oteryn:item.tibia.i{source}']['authoring'].get('forge'),
                                 {'classification': classification, 'max_tier': maximum})

    def test_overlay_keeps_native_membership_and_copies_only_authoritative_profiles(self):
        document = copy.deepcopy(self.native)
        keys = [row['production_definition']['production_key'] for row in document['records']]
        forge.project(document, self.owners)
        self.assertEqual(keys, [row['production_definition']['production_key'] for row in document['records']])
        for row in document['records']:
            key = row['production_definition']['production_key']
            if key in self.owners:
                self.assertEqual(row['authoring']['forge'], self.owners[key]['authoring']['forge'])
            else:
                self.assertNotIn('forge', row['authoring'])

    def test_conflicting_or_unowned_profile_cannot_be_admitted(self):
        document = copy.deepcopy(self.native)
        row = next(row for row in document['records']
                   if row['production_definition']['production_key'] == 'oteryn:item.tibia.i3264')
        row['authoring']['forge'] = {'classification': 4, 'max_tier': 10}
        with self.assertRaisesRegex(ValueError, 'FORGE_NATIVE_OWNER_CONFLICT'):
            forge.project(document, self.owners)
        with self.assertRaisesRegex(ValueError, 'FORGE_NATIVE_UNOWNED_PROFILE'):
            forge.project(document, {})


if __name__ == '__main__':
    unittest.main()
