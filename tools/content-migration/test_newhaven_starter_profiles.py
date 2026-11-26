"""C19b starter equipment must be admitted in the real active native Item provider."""
import json
import unittest
import tempfile

import newhaven_starter_profiles as starter
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]


class NewhavenStarterProfilesTests(unittest.TestCase):
    def test_newhaven_starter_definitions_are_carriable_native_equipment(self):
        profiles = json.loads((ROOT / 'content/items/definitions/spell-native-profiles.json').read_bytes())
        fragment = json.loads((ROOT / 'content/items/definitions/starting-loadout-profiles.json').read_bytes())
        by_key = {row['production_definition']['production_key']: row for row in profiles['records'] + fragment['records']}
        for item in (34017, 21348, 21400, 21350, 50166):
            key = f'oteryn:item.tibia.i{item}'
            self.assertTrue(key in by_key, f"missing active native starter profile: {key}")
            profile = by_key[key]
            self.assertTrue(profile['admission']['materializable'])
            self.assertEqual(profile['semantics']['equipment']['state'], 'KNOWN')
            self.assertIn('CharacterInventory', profile['admission']['legal_destinations'])
            self.assertEqual(profile['semantics']['physical']['state'], 'KNOWN')
            self.assertGreater(profile['semantics']['physical']['value']['weight']['value'], 0)


    def test_novice_spellbook_type_matches_item_shard_and_native_provider(self):
        key = 'oteryn:item.tibia.i21400'
        expected = {'state': 'KNOWN', 'value': 'SPELLBOOK'}
        items = json.loads((ROOT / 'content/items/definitions/items-10500-10999.json').read_bytes())
        native = json.loads((ROOT / 'content/items/definitions/starting-loadout-profiles.json').read_bytes())
        item = next(row['definition'] for row in items['records']
                    if row['definition']['identity']['key'] == key)
        profile = next(row for row in native['records']
                       if row['production_definition']['production_key'] == key)
        for entry in (item, profile):
            self.assertEqual(entry['semantics']['weapon']['value']['weapon_type'], expected)

    def test_duplicate_profile_refuses_before_changing_generated_bytes(self):
        overlay = json.loads((ROOT / starter.PATH).read_bytes())
        planned = {'profiles': json.dumps({'records': [overlay['records'][0]]}).encode(), 'manifest': b'{}'}
        before = dict(planned)
        with self.assertRaisesRegex(ValueError, 'NEWHAVEN_STARTER_PROFILE_DUPLICATE'):
            starter.admit(ROOT, planned, 'profiles', 'manifest', lambda row: 0, lambda value: b'')
        self.assertEqual(planned, before)

    def test_missing_profile_refuses_before_changing_generated_bytes(self):
        overlay = json.loads((ROOT / starter.PATH).read_bytes())
        overlay['records'].pop()
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            target = root / starter.PATH
            target.parent.mkdir(parents=True)
            target.write_text(json.dumps(overlay))
            planned = {'profiles': b'{"records":[]}', 'manifest': b'{}'}
            before = dict(planned)
            with self.assertRaisesRegex(ValueError, 'NEWHAVEN_STARTER_PROFILE_SET'):
                starter.admit(root, planned, 'profiles', 'manifest', lambda row: 0, lambda value: b'')
            self.assertEqual(planned, before)


if __name__ == '__main__':
    unittest.main()
