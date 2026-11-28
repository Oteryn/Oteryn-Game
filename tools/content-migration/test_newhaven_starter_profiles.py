"""C19b starter equipment must be admitted in the real active native Item provider."""
import json
import os
import tempfile
import unittest
import xml.etree.ElementTree as ET
from pathlib import Path

import newhaven_starter_profiles as starter
import ranged_item_profiles as ranged

ROOT = Path(__file__).resolve().parents[2]


class NewhavenStarterProfilesTests(unittest.TestCase):
    def test_newhaven_starter_attacks_match_items_xml(self):
        fragment = ROOT / 'content/items/definitions/starting-loadout-profiles.json'
        records = {
            row['production_definition']['production_key']: row
            for row in json.loads(fragment.read_bytes())['records']
        }
        # Offline runs use the importer's retained XML. Donor qualification
        # runs the same assertions against Crystal's pinned items.xml.
        source = ET.parse(os.environ.get('OTERYN_TEST_CRYSTAL_ITEMS_XML', ROOT / ranged.SOURCE))
        for item_id, kind in [(21348, 'WAND'), (21350, 'ROD')]:
            with self.subTest(item=item_id):
                item = source.find(f"./item[@id='{item_id}']")
                top = {node.attrib['key']: node.get('value') for node in item.findall('attribute')}
                script = {
                    node.attrib['key']: node.attrib['value']
                    for node in item.findall("attribute[@key='script']/attribute")
                }
                expected = dict(
                    kind=kind,
                    damage_min=int(script['fromDamage']),
                    damage_max=int(script['toDamage']),
                    mana_per_shot=int(script['mana']),
                    range_cells=int(top['range']),
                    element=script['wandType'].upper(),
                )
                actual = records[f'oteryn:item.tibia.i{item_id}']['authoring'].get('ranged')
                self.assertEqual(actual, expected)

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
