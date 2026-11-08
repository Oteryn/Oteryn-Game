"""Strict owner coalescing of initialized House source metadata."""
import copy
import json
import unittest
from pathlib import Path

import dispenser_item_profiles as dispenser
import house_decoration_actions as house
from srx1_profile_updates import coalesce_dispenser_top_order

ROOT = Path(__file__).resolve().parents[2]


class SourceOwnerCoalescing(unittest.TestCase):
    def setUp(self):
        self.source = json.loads((ROOT / dispenser.PATH).read_bytes())['records']
        self.house = json.loads((ROOT / house.PROFILES).read_bytes())

    def admit(self, row):
        document = {'records': [row]}
        coalesce_dispenser_top_order(document, self.source, self.house)
        generated = {'items': house.encode(document), 'manifest': house.encode({'item_profiles': {}})}
        dispenser.admit(ROOT, generated, 'items', 'manifest',
                        lambda record: record['production_definition']['production_key'], house.encode)
        return json.loads(generated['items'])

    def test_exact_source_plus_verified_top_metadata_coalesces(self):
        row = copy.deepcopy(self.source[0])
        house.apply_top_order([row], self.house)
        document = self.admit(row)
        self.assertEqual(len(document['records']), len(self.source))
        self.assertIn(self.source[0], document['records'])

    def test_changed_owner_fact_still_refuses(self):
        row = copy.deepcopy(self.source[0])
        house.apply_top_order([row], self.house)
        row['semantics']['physical']['value']['weight'] = {'state': 'KNOWN', 'value': 999_999}
        with self.assertRaisesRegex(ValueError, 'DISPENSER_EXISTING_OWNER_CONFLICT'):
            self.admit(row)

    def test_unverified_top_metadata_still_refuses(self):
        row = copy.deepcopy(self.source[0])
        house.apply_top_order([row], self.house)
        row['attributes']['always_on_top_order'] = 255
        with self.assertRaisesRegex(ValueError, 'DISPENSER_EXISTING_OWNER_CONFLICT'):
            self.admit(row)
