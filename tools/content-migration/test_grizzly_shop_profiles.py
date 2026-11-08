"""Finite source qualification keeps Item holds and conflicting shop keys closed."""
import copy
import hashlib
import json
from pathlib import Path
import subprocess
import tempfile
import unittest
from unittest import mock
import xml.etree.ElementTree as ET

import grizzly_shop_profiles as tool


class GrizzlyShopProfiles(unittest.TestCase):
    def test_custody_qualification_requires_exact_retained_profile_and_preserves_other_facts(self):
        key = 'oteryn:item.tibia.i7393'
        profile = next(r for r in self.packet['records']
                       if r['production_definition']['production_key'] == key)
        definition = {'identity': {'key': key}, 'materializable': False,
                      'stack_class': 'Unknown', 'semantics': {
                          'physical': copy.deepcopy(profile['semantics']['physical']),
                          'stack': {'state': 'UNKNOWN'}, 'container': {'state': 'UNKNOWN'}}}
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            relative = 'content/items/definitions/items-00000-00499.json'
            shard = root / relative
            shard.parent.mkdir(parents=True)
            shard.write_text(json.dumps({'records': [{'definition': definition}]}))
            packet = {'catalogue': {'path': tool.CATALOGUE, 'sha256': 'catalogue'},
                      'canonical_inputs': [{'path': relative,
                                            'sha256': hashlib.sha256(shard.read_bytes()).hexdigest()}],
                      'records': [profile], 'rejected': []}
            path = root / tool.PATH
            path.parent.mkdir(parents=True)
            path.write_text(json.dumps(packet))
            git = ['git', '-C', str(root)]
            subprocess.run(git + ['init', '-q'], check=True, capture_output=True)
            subprocess.run(git + ['add', '.'], check=True, capture_output=True)
            subprocess.run(git + ['-c', 'user.name=fixture', '-c', 'user.email=fixture@example.invalid',
                                 '-c', 'commit.gpgsign=false', 'commit', '-qm', 'source base'],
                           check=True, capture_output=True)
            definition['materializable'] = profile['admission']['materializable']
            definition['stack_class'] = profile['admission']['stack_class']
            for group in ('stack', 'container'):
                definition['semantics'][group] = copy.deepcopy(profile['semantics'][group])
            with mock.patch.object(tool, 'selected', return_value=({key}, 'catalogue')):
                shard.write_text(json.dumps({'records': [{'definition': definition}]}))
                with self.assertRaisesRegex(ValueError, 'SELECTED_CANONICAL_DRIFT'):
                    tool.refresh_canonical_inputs(root=root)
                refreshed = tool.refresh_canonical_inputs(root=root, qualify_custody=True)
                self.assertEqual(refreshed['records'], packet['records'])
                definition['semantics']['physical']['value']['weight']['value'] += 1
                shard.write_text(json.dumps({'records': [{'definition': definition}]}))
                with self.assertRaisesRegex(ValueError, 'SELECTED_CANONICAL_DRIFT'):
                    tool.refresh_canonical_inputs(root=root, qualify_custody=True)

    def test_hash_refresh_requires_unchanged_shop_facts_and_an_exact_git_base(self):
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            relative = 'content/items/definitions/items-00000-00499.json'
            shard = root / relative
            shard.parent.mkdir(parents=True)
            shop = {'definition': {'identity': {'key': 'oteryn:item.tibia.i5907'}, 'weight': 200}}
            other = {'definition': {'identity': {'key': 'oteryn:item.tibia.i33867'}, 'capacity': None}}
            original = {'records': [shop, other]}
            shard.write_text(json.dumps(original))
            git = ['git', '-C', str(root)]
            subprocess.run(git + ['init', '-q'], check=True, capture_output=True)
            packet = {'catalogue': {'path': tool.CATALOGUE, 'sha256': 'catalogue'},
                      'canonical_inputs': [{'path': relative,
                                            'sha256': hashlib.sha256(shard.read_bytes()).hexdigest()}],
                      'records': ['retained source evidence'], 'rejected': ['retained refusals']}
            path = root / tool.PATH
            path.parent.mkdir(parents=True)
            path.write_text(json.dumps(packet))
            subprocess.run(git + ['add', relative, tool.PATH], check=True, capture_output=True)
            commit = git + ['-c', 'user.name=fixture', '-c', 'user.email=fixture@example.invalid',
                            '-c', 'commit.gpgsign=false', 'commit', '-qm']
            subprocess.run(commit + ['canonical base'], check=True, capture_output=True)
            other['definition']['capacity'] = 48
            shard.write_text(json.dumps(original))
            with mock.patch.object(tool, 'selected', return_value=({'oteryn:item.tibia.i5907'}, 'catalogue')):
                refreshed = tool.refresh_canonical_inputs(root=root)
                self.assertEqual(refreshed['records'], packet['records'])
                self.assertEqual(refreshed['rejected'], packet['rejected'])
                self.assertEqual(refreshed['canonical_inputs'][0]['sha256'],
                                 hashlib.sha256(shard.read_bytes()).hexdigest())
                path.write_text(json.dumps(refreshed))
                self.assertEqual(tool.refresh_canonical_inputs(root=root), refreshed)
                changed_capture = copy.deepcopy(refreshed)
                changed_capture['records'] = ['unreviewed source evidence']
                path.write_text(json.dumps(changed_capture))
                with self.assertRaisesRegex(ValueError, 'SOURCE_CAPTURE_DRIFT'):
                    tool.refresh_canonical_inputs(root=root)
                path.write_text(json.dumps(packet))
                shop['definition']['weight'] = 201
                shard.write_text(json.dumps(original))
                with self.assertRaisesRegex(ValueError, 'SELECTED_CANONICAL_DRIFT'):
                    tool.refresh_canonical_inputs(root=root)
                packet['canonical_inputs'][0]['sha256'] = '0' * 64
                path.write_text(json.dumps(packet))
                subprocess.run(git + ['add', tool.PATH], check=True, capture_output=True)
                subprocess.run(commit + ['invalid baseline descriptor'], check=True, capture_output=True)
                with self.assertRaisesRegex(ValueError, 'CANONICAL_BASE_DRIFT'):
                    tool.refresh_canonical_inputs(root=root)

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
