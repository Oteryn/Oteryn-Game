"""Finite source qualification keeps Item holds and conflicting shop keys closed."""
import copy
import hashlib
import json
import unittest
import xml.etree.ElementTree as ET

import grizzly_shop_profiles as tool


class GrizzlyShopProfiles(unittest.TestCase):
    def setUp(self):
        self.packet = json.loads((tool.ROOT / tool.PATH).read_bytes())
        self.bindings = tool.source.binding_index(tool.source.BINDINGS.read_bytes())
        self.canonical = {}
        for path in (tool.ROOT / 'content/items/definitions').glob('items-*.json'):
            for row in json.loads(path.read_bytes())['records']:
                definition = row['definition']
                if definition['identity']['key'] == 'oteryn:item.tibia.i5907':
                    self.canonical = definition
        self.node = ET.fromstring('<item id="5907" name="slingshot">'
                                 '<attribute key="weight" value="200" /></item>')
        self.physical = {'movable': True, 'pickupable': True,
                         'block_solid': False, 'block_projectile': False, 'has_height': False}

    def qualify(self, definition=None):
        return tool.qualify(5907, self.node, {'flags': {}}, self.physical,
                            self.bindings, definition or self.canonical)

    def test_generated_profiles_account_for_all_offers_and_reject_three_weight_conflicts(self):
        keys, _ = tool.selected(tool.ROOT)
        rows = self.packet['records']
        admitted = {row['production_definition']['production_key'] for row in rows}
        rejected = {row['key'] for row in self.packet['rejected']}
        self.assertEqual(len(admitted), 53)
        self.assertEqual(rejected, {f'oteryn:item.tibia.i{i}' for i in (10454, 11489, 18994)})
        self.assertEqual(admitted | rejected, keys)
        self.assertFalse(admitted & rejected)
        for row in rows:
            self.assertTrue(row['admission']['materializable'])
            self.assertEqual(row['production_binding']['disposition'], 'EXACT')
            self.assertEqual(row['attributes']['speed_bonus'], 0)
            for name, group in row['semantics'].items():
                if group.get('state') == 'KNOWN' and isinstance(group['value'], dict):
                    self.assertNotIn('state', group['value'], (row['production_definition'], name))
        self.assertIn('oteryn:item.tibia.i11488', admitted)
        self.assertIn('oteryn:item.tibia.i9601', admitted)

    def test_source_profile_fills_unknown_admission_without_mutating_canonical_hold(self):
        before = copy.deepcopy(self.canonical)
        row = self.qualify()
        self.assertEqual(row['admission'], {'materializable': True, 'stack_class': 'NonStackable',
                                           'legal_destinations': ['CharacterInventory', 'Ground']})
        self.assertEqual(self.canonical, before)
        self.assertFalse(self.canonical['materializable'])
        self.assertEqual(row['semantics']['presentation'], self.canonical['semantics']['presentation'])

    def test_known_weight_stack_and_container_conflicts_fail_closed(self):
        for group, value in (
            ('physical', {'weight': tool.source.K(201)}),
            ('stack', {'stackable': tool.source.K(True)}),
            ('container', {'capacity': tool.source.K(20)}),
        ):
            changed = copy.deepcopy(self.canonical)
            changed['semantics'][group] = tool.source.K(value)
            with self.subTest(group=group), self.assertRaisesRegex(ValueError, 'CANONICAL_' + group.upper() + '_CONFLICT'):
                self.qualify(changed)

    def test_unsupported_source_rule_and_missing_exact_binding_fail_closed(self):
        ET.SubElement(self.node, 'attribute', key='decayTo', value='1')
        with self.assertRaisesRegex(ValueError, 'UNSUPPORTED_SOURCE_ATTRIBUTES'):
            self.qualify()
        self.node.remove(self.node[-1])
        self.bindings.pop('5907')
        with self.assertRaisesRegex(ValueError, 'MISSING_EXACT_BINDING'):
            self.qualify()

    def test_known_payload_over_unknown_never_inserts_a_fact_wrapper_inside_value(self):
        for name in ('trade_restrictions', 'imbuement'):
            accepted = tool.source.K({'slot_count' if name == 'imbuement' else 'marketable': tool.source.K(1)})
            self.assertEqual(tool.accepted_first(accepted, tool.source.U), accepted)
            self.assertNotIn('state', tool.accepted_first(accepted, tool.source.U)['value'])
        accepted = tool.source.K({'patterns': tool.source.K([
            {'pattern_id': 1, 'level': tool.source.U, 'primary_slot': tool.source.K('CONTAINER')}
        ])})
        observed = tool.source.K({'patterns': tool.source.K([
            {'pattern_id': 1, 'level': tool.source.K(0), 'primary_slot': tool.source.K('CONTAINER')}
        ])})
        self.assertEqual(tool.accepted_first(accepted, observed), observed)

    def test_registration_preserves_profiles_and_pins_the_exact_result(self):
        generated = {'profiles': b'{"records":[]}', 'manifest': b'{"item_profiles":{}}'}
        order = lambda row: row['production_definition']['production_key']
        tool.admit(tool.ROOT, generated, 'profiles', 'manifest', order, tool.encode)
        self.assertEqual(len(json.loads(generated['profiles'])['records']), 53)
        self.assertEqual(json.loads(generated['manifest'])['item_profiles']['sha256'],
                         hashlib.sha256(generated['profiles']).hexdigest())
        once = generated['profiles']
        tool.admit(tool.ROOT, generated, 'profiles', 'manifest', order, tool.encode)
        self.assertEqual(generated['profiles'], once)
        rows = json.loads(once)
        rows['records'][0]['admission']['stack_class'] = 'Unknown'
        generated['profiles'] = tool.encode(rows)
        with self.assertRaisesRegex(ValueError, 'GRIZZLY_SHOP_EXISTING_PROFILE_CONFLICT'):
            tool.admit(tool.ROOT, generated, 'profiles', 'manifest', order, tool.encode)


if __name__ == '__main__':
    unittest.main()
