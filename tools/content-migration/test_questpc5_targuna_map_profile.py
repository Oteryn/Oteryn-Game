"""Exact Old Treasure Map custody; source qualification never rewrites canonical facts."""
import hashlib
import json
import unittest
from pathlib import Path
import questpc5_targuna_map_profile as author


class TargunaMapProfile(unittest.TestCase):
    def test_exact_portable_map_preserves_canonical_record(self):
        root = Path(__file__).resolve().parents[2]
        paths = sorted((root / 'content/items/definitions').glob('items-*.json'))
        before = {str(p): hashlib.sha256(p.read_bytes()).hexdigest() for p in paths}
        packet = author.capture(root)
        row, = packet['records']
        self.assertEqual(row['production_definition']['production_key'], 'oteryn:item.tibia.i53158')
        self.assertEqual(row['admission'], dict(materializable=True, stack_class='NonStackable',
            legal_destinations=['CharacterInventory', 'Ground']))
        physical = row['semantics']['physical']['value']
        self.assertEqual(physical['weight'], dict(state='KNOWN', value=400))
        self.assertEqual(physical['movable'], dict(state='KNOWN', value=True))
        self.assertEqual(physical['pickupable'], dict(state='KNOWN', value=True))
        self.assertEqual(row['semantics']['presentation']['value']['name'], dict(state='KNOWN', value='old treasure map'))
        self.assertEqual({str(p): hashlib.sha256(p.read_bytes()).hexdigest() for p in paths}, before)

    def test_only_pinned_xml_and_served_appearance_facts_are_cited(self):
        packet = author.capture(Path(__file__).resolve().parents[2])
        self.assertEqual(packet['sources'][0]['sha256'], 'c847293e980b40ec146e2b7f68a62366513a1c0566d16b7c3a011136087021eb')
        self.assertEqual(packet['sources'][1]['sha256'], '2dfa943b548472a1ddc7bc5afe97945bc75e14f1f41d74f728f8e622f5dae7e2')


if __name__ == '__main__':
    unittest.main()
