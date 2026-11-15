"""Coverage of the real source union and deterministic pack generation."""
import unittest
from unittest.mock import patch

import test_merchant as merchant


class TestMerchant(unittest.TestCase):
    @classmethod
    def setUpClass(cls):
        cls.pack = merchant.build()
        cls.offers = [offer for service in cls.pack['declarations'][1:]
                      for offer in service['offers']]
        cls.keys = {offer['item']['key'] for offer in cls.offers}

    def test_services_are_bounded_and_every_offer_costs_one_gold(self):
        for service in self.pack['declarations'][1:]:
            self.assertGreater(len(service['offers']), 0)
            self.assertLessEqual(len(service['offers']), 1024)
        self.assertEqual(len(self.keys), len(self.offers))
        for offer in self.offers:
            self.assertEqual(offer['direction'], 'SellToPlayer')
            self.assertEqual(offer['unit_price'], 1)

    def test_all_four_sources_are_represented(self):
        source = merchant.sources(merchant.ROOT)
        for name in ('loot', 'trade', 'quest', 'wheel'):
            self.assertTrue(source[name] & self.keys, name)
            self.assertEqual(len(source[name] & self.keys), self.pack['counts']['included_sources'][name])
        packet = merchant.read(merchant.ROOT, 'docs/agents/evidence/OTV2-20261001-quest-reward-item-admission.json')
        self.assertEqual(len(packet['admissions']), 338)
        self.assertTrue({row['item_key'] for row in packet['admissions']} <= self.keys)

    def test_all_gems_and_fragments(self):
        self.assertTrue({f'oteryn:item.tibia.i{i}' for i in merchant.GEMS} <= self.keys)

    def test_stackable_runes_do_not_treat_legacy_charges_as_a_subtype(self):
        offers = {offer['item']['key']: offer for offer in self.offers}
        for row in merchant.records(merchant.ROOT, 'content/items/definitions'):
            definition = row.get('definition', {})
            key = definition.get('identity', {}).get('key')
            stackable = definition.get('semantics', {}).get('stack', {}).get('value', {}).get('stackable')
            if key in offers and (definition.get('stack_class') == 'StackCapable'
                                  or stackable == dict(state='KNOWN', value=True)):
                self.assertNotIn('sub_type', offers[key], key)

    def test_placement_is_next_to_the_pinned_default_start(self):
        start = merchant.read(merchant.ROOT, 'content/world/pins/oteryn.json')['entry_start']
        position = self.pack['placement']
        self.assertEqual((position['x'], position['y'], position['floor']),
                         (start['x'] + 1, start['y'], start['floor']))

    def test_unknown_movement_is_not_an_explicit_prohibition(self):
        self.assertIn('oteryn:item.tibia.i13995', self.keys)  # Looted depth galea.
        for row in merchant.records(merchant.ROOT, 'content/items/definitions'):
            definition = row.get('definition', {})
            if definition.get('identity', {}).get('key') in self.keys:
                physical = definition.get('semantics', {}).get('physical', {}).get('value', {})
                for field in ('movable', 'pickupable'):
                    self.assertNotEqual(physical.get(field), dict(state='KNOWN', value=False))

    def test_explicit_physical_prohibitions_are_skipped(self):
        records = merchant.records
        for field in ('pickupable', 'movable'):
            def prohibited(root, directory):
                for row in records(root, directory):
                    definition = row.get('definition', {})
                    if definition.get('identity', {}).get('key') == 'oteryn:item.tibia.i13995':
                        definition['semantics']['physical']['value'][field] = dict(state='KNOWN', value=False)
                    yield row
            with patch.object(merchant, 'records', side_effect=prohibited):
                pack = merchant.build()
            keys = {offer['item']['key'] for service in pack['declarations'][1:]
                    for offer in service['offers']}
            self.assertNotIn('oteryn:item.tibia.i13995', keys)

    def test_rerun_is_byte_identical(self):
        expected = merchant.encode(self.pack)
        self.assertEqual(expected, merchant.encode(merchant.build()))
        self.assertEqual(expected, (merchant.ROOT / merchant.PACK).read_bytes())
        self.assertEqual(merchant.inventory(self.pack), merchant.inventory(merchant.build()))
        self.assertEqual(merchant.inventory(self.pack),
                         (merchant.ROOT / merchant.PACK.parent / 'inventory.md').read_bytes())

    def test_large_category_is_split_into_numbered_parts(self):
        read = merchant.read
        def one_category(root, path):
            value = read(root, path)
            if path == 'content/items/taxonomy/items.json':
                for row in value['records']:
                    row['source_taxonomy'] = {'primary': 'Equipment'}
            return value
        with patch.object(merchant, 'read', side_effect=one_category):
            pack = merchant.build()
        equipment = [category for category in pack['categories']
                     if category['keyword'].startswith('equipment')]
        self.assertGreaterEqual(len(equipment), 3)
        self.assertEqual([category['keyword'] for category in equipment[:3]],
                         ['equipment', 'equipment 2', 'equipment 3'])
        for service in pack['declarations'][1:]:
            self.assertLessEqual(len(service['offers']), 1024)


if __name__ == '__main__':
    unittest.main()
