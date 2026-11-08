import copy
import hashlib
import json
from pathlib import Path
import subprocess
import tempfile
import unittest

import refresh_item_overlay_inputs as tool


class OverlayInputRefreshTests(unittest.TestCase):
    def test_native_core_requires_exact_unchanged_profile_and_preserves_known_facts(self):
        definition = {'identity': {'key': 'item'}, 'materializable': False,
                      'stack_class': 'Unknown', 'semantics': {
                          'physical': {'state': 'KNOWN', 'value': {'weight': {'state': 'KNOWN', 'value': 100}}},
                          'charges': {'state': 'KNOWN', 'value': {'count': {'state': 'KNOWN', 'value': 50}}},
                          'temporal': {'state': 'UNKNOWN'}, 'weapon': {'attack': 30},
                          'stack': {'state': 'UNKNOWN'}, 'container': {'state': 'UNKNOWN'}}}
        profile = {'admission': {'materializable': True, 'stack_class': 'NonStackable',
                                'legal_destinations': ['CharacterInventory', 'Ground']},
                   'semantics': copy.deepcopy(definition['semantics'])}
        profile['semantics'].update(stack={'state': 'KNOWN', 'value': {
            'stackable': {'state': 'KNOWN', 'value': False}}},
            container={'state': 'NOT_APPLICABLE'}, temporal={'state': 'NOT_APPLICABLE'})
        before = copy.deepcopy(definition)
        result = tool.qualify_native_core(definition, profile)
        self.assertEqual(definition, before)
        self.assertEqual(result['semantics']['weapon'], before['semantics']['weapon'])
        self.assertEqual(result['semantics']['charges'], before['semantics']['charges'])
        self.assertEqual(result['semantics']['temporal'], {'state': 'NOT_APPLICABLE'})
        for group, field, value in [('physical', 'weight', 101), ('charges', 'count', 49)]:
            changed = copy.deepcopy(profile)
            changed['semantics'][group]['value'][field]['value'] = value
            with self.subTest(group=group), self.assertRaisesRegex(ValueError, 'NATIVE_CORE_CONFLICT'):
                tool.qualify_native_core(definition, changed)

    def test_custody_qualification_preserves_physical_charge_and_combat_facts(self):
        definition = {'identity': {'key': 'item'}, 'materializable': False,
                      'stack_class': 'Unknown', 'semantics': {
                          'physical': {'weight': 100}, 'charges': {'count': 1},
                          'weapon': {'attack': 30}, 'stack': {'state': 'UNKNOWN'},
                          'container': {'state': 'UNKNOWN'}}}
        profile = {'admission': {'materializable': True, 'stack_class': 'NonStackable',
                                'legal_destinations': ['CharacterInventory', 'Ground']},
                   'semantics': {'stack': {'state': 'KNOWN', 'value': {
                       'stackable': {'state': 'KNOWN', 'value': False}}},
                       'container': {'state': 'NOT_APPLICABLE'}}}
        before = copy.deepcopy(definition)
        result = tool.qualify_custody(definition, profile)
        self.assertEqual(definition, before)
        for group in ('physical', 'charges', 'weapon'):
            self.assertEqual(result['semantics'][group], before['semantics'][group])
        result['semantics']['stack']['value']['stackable']['value'] = True
        with self.assertRaisesRegex(ValueError, 'CUSTODY_CONFLICT'):
            tool.qualify_custody(result, profile)
        profile['admission']['legal_destinations'] = ['Ground']
        with self.assertRaisesRegex(ValueError, 'PORTABLE_CUSTODY_REQUIRED'):
            tool.qualify_custody(definition, profile)

    def test_unrelated_shard_edits_refresh_but_selected_facts_and_source_changes_refuse(self):
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            relative = 'content/items/definitions/items-00000-00499.json'
            overlay = 'tools/content-schema/native-gameplay/example.json'
            selected = {'identity': {'key': 'item'}, 'weight': 100}
            other = {'identity': {'key': 'other'}, 'weight': 200}
            rows = {'records': [{'definition': selected}, {'definition': other}]}
            shard = root / relative
            shard.parent.mkdir(parents=True)
            shard.write_text(json.dumps(rows))
            packet = {'canonical_inputs': [{'path': relative,
                                            'sha256': hashlib.sha256(shard.read_bytes()).hexdigest()}],
                      'records': [{'production_definition': {'production_key': 'item'}}],
                      'source': 'unchanged'}
            path = root / overlay
            path.parent.mkdir(parents=True)
            path.write_text(json.dumps(packet))
            git = ['git', '-C', str(root)]
            subprocess.run(git + ['init', '-q'], check=True, capture_output=True)
            subprocess.run(git + ['add', '.'], check=True, capture_output=True)
            subprocess.run(git + ['-c', 'user.name=fixture', '-c', 'user.email=fixture@example.invalid',
                                 '-c', 'commit.gpgsign=false', 'commit', '-qm', 'source base'],
                           check=True, capture_output=True)
            other['weight'] = 201
            shard.write_text(json.dumps(rows))
            result = json.loads(tool.refresh(overlay, root=root))
            self.assertEqual(result['records'], packet['records'])
            self.assertNotEqual(result['canonical_inputs'], packet['canonical_inputs'])
            shard.write_text(json.dumps({'records': rows['records'] + [rows['records'][0]]}))
            with self.assertRaisesRegex(ValueError, 'DUPLICATE_SELECTED_IDENTITY'):
                tool.refresh(overlay, root=root)
            selected['weight'] = 101
            shard.write_text(json.dumps(rows))
            with self.assertRaisesRegex(ValueError, 'SELECTED_FACT_DRIFT'):
                tool.refresh(overlay, root=root)
            changed = copy.deepcopy(packet)
            changed['source'] = 'unreviewed'
            path.write_text(json.dumps(changed))
            with self.assertRaisesRegex(ValueError, 'SOURCE_CAPTURE_DRIFT'):
                tool.refresh(overlay, root=root)


if __name__ == '__main__':
    unittest.main()
