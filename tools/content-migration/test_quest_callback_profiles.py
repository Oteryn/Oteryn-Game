"""Rebuild callback Item admissions against retained, pinned source inputs."""
import copy
import hashlib
import json
from pathlib import Path
import tempfile
import unittest
from unittest.mock import patch

import quest_callback_profiles as profiles
import writable_item_profiles as writing

ROOT = Path(__file__).resolve().parents[2]
PROFILE = 'content/items/definitions/spell-native-profiles.json'
MANIFEST = 'content/items/native-manifest.json'
KEY = 'oteryn:item.tibia.i7934'


class QuestCallbackProfiles(unittest.TestCase):
    def setUp(self):
        self.temp = tempfile.TemporaryDirectory(prefix='.qcb3-profile-', dir=ROOT)
        self.addCleanup(self.temp.cleanup)
        self.root = Path(self.temp.name)
        (self.root / 'content/quests').mkdir(parents=True)
        (self.root / 'content/interactions').symlink_to(ROOT / 'content/interactions', target_is_directory=True)
        (self.root / 'content/items').symlink_to(ROOT / 'content/items', target_is_directory=True)
        (self.root / 'imports').symlink_to(ROOT / 'imports', target_is_directory=True)
        self.callbacks = json.loads((ROOT / 'content/quests/world-callbacks.json').read_bytes())
        (self.root / 'content/quests/killing-in-the-name-of.json').write_bytes(
            (ROOT / 'content/quests/killing-in-the-name-of.json').read_bytes())
        self.document = json.loads((ROOT / PROFILE).read_bytes())
        self.document['records'] = [r for r in self.document['records']
                                    if r['production_definition']['production_key'] not in
                                    (KEY, 'oteryn:item.tibia.i6120', 'oteryn:item.tibia.i9019',
                                     'oteryn:item.tibia.i9388')]
        self.generated = {PROFILE: json.dumps(self.document).encode(),
                          MANIFEST: b'{"item_profiles":{"sha256":"pending"}}'}

    def admit(self):
        (self.root / 'content/quests/world-callbacks.json').write_text(json.dumps(self.callbacks))
        profiles.admit(self.root, self.generated, PROFILE, MANIFEST,
                       lambda r: r['production_definition']['production_key'],
                       lambda v: json.dumps(v, sort_keys=True).encode())

    def test_fish_key_is_rebuilt_with_portable_admission_and_manifest_digest(self):
        self.admit()
        rows = [r for r in json.loads(self.generated[PROFILE])['records']
                if r['production_definition']['production_key'] == KEY]
        self.assertEqual(len(rows), 1)
        self.assertEqual(rows[0]['admission'], {'materializable': True,
                         'stack_class': 'NonStackable',
                         'legal_destinations': ['CharacterInventory', 'Ground']})
        self.assertEqual(json.loads(self.generated[MANIFEST])['item_profiles']['sha256'],
                         hashlib.sha256(self.generated[PROFILE]).hexdigest())

    def test_quest_chest_rewards_preserve_canonical_item_admission(self):
        self.admit()
        rows = {r['production_definition']['production_key']: r
                for r in json.loads(self.generated[PROFILE])['records']}
        for key in ('oteryn:item.tibia.i6120', 'oteryn:item.tibia.i9019'):
            row = rows[key]
            self.assertEqual(row['admission'], {'materializable': True,
                             'stack_class': 'NonStackable',
                             'legal_destinations': ['CharacterInventory', 'Ground']})
            self.assertEqual(row['production_binding']['disposition'], 'EXACT')
            self.assertEqual(row['production_binding']['target']['key'], key)
            self.assertEqual(row['semantics']['physical']['value']['pickupable'],
                             {'state': 'KNOWN', 'value': True})

    def test_unadmitted_replacement_refuses_instead_of_manufacturing_a_profile(self):
        callback = next(c for c in self.callbacks['callbacks'] if c.get('item_on_target'))
        callback['item_on_target']['replacement'] = 'oteryn:item.unadmitted'
        with self.assertRaisesRegex(ValueError, 'QUEST_CALLBACK_ITEM_MISSING'):
            self.admit()

    def test_grizzly_icon_and_every_complete_chest_child_have_native_profiles(self):
        self.admit()
        rows = {row['production_definition']['production_key']: row
                for row in json.loads(self.generated[PROFILE])['records']}
        expected = {'oteryn:item.tibia.i9388'}
        for path in sorted((ROOT / 'content/interactions/reward_claims').glob('reward-claim*.json')):
            for record in json.loads(path.read_bytes())['records']:
                for placement in record['definition'].get('native_contents', {}).get('placements', []):
                    expected.add(placement['root']['key'])
                    expected.update(child['item']['key'] for child in placement['contents'])
        self.assertEqual(len(expected), 130)
        for key in expected:
            row = rows[key]
            self.assertTrue(row['admission']['materializable'], key)
            self.assertIn('CharacterInventory', row['admission']['legal_destinations'], key)
            self.assertEqual(row['production_binding']['target']['key'], key)
            self.assertEqual(row['production_definition']['revision_ref'],
                             row['production_binding']['target']['revision'], key)
            weight = row['semantics']['physical']['value']['weight']
            self.assertEqual(weight['state'], 'KNOWN', key)
            self.assertIs(type(weight['value']), int, key)
            if row['admission']['stack_class'] == 'StackCapable':
                stack = row['semantics']['stack']
                self.assertEqual(stack['state'], 'KNOWN', key)
                maximum = stack['value']['stack_max']
                self.assertEqual(maximum['state'], 'KNOWN', key)
                self.assertTrue(1 <= maximum['value'] <= 100, key)

    def select_potion(self):
        self.callbacks['callbacks'][0]['offerings'].append(
            {'item': 'oteryn:item.tibia.i237', 'count': 1})

    def generated_record(self, key):
        return next(row for row in json.loads(self.generated[PROFILE])['records']
                    if row['production_definition']['production_key'] == key)

    def test_present_potion_stack_is_completed_without_changing_other_native_facts(self):
        self.select_potion()
        for stack in ({'state': 'UNKNOWN'}, {'state': 'KNOWN', 'value': {
                'stackable': {'state': 'KNOWN', 'value': True},
                'stack_max': {'state': 'UNKNOWN'}}}):
            document = copy.deepcopy(self.document)
            before = next(row for row in document['records']
                          if row['production_definition']['production_key'] == 'oteryn:item.tibia.i237')
            before['semantics']['stack'] = stack
            self.generated[PROFILE] = json.dumps(document).encode()
            self.admit()
            actual = self.generated_record('oteryn:item.tibia.i237')
            expected = copy.deepcopy(before)
            expected['semantics']['stack'] = {'state': 'KNOWN', 'value': {
                'stackable': {'state': 'KNOWN', 'value': True},
                'stack_max': {'state': 'KNOWN', 'value': 100}}}
            self.assertEqual(actual, expected)

    def test_known_native_stack_weight_effects_and_repeat_registration_are_preserved(self):
        self.select_potion()
        self.callbacks['callbacks'][0]['offerings'].append(
            {'item': 'oteryn:item.tibia.i3031', 'count': 1})
        before = next(row for row in self.document['records']
                      if row['production_definition']['production_key'] == 'oteryn:item.tibia.i237')
        before['semantics']['stack'] = {'state': 'KNOWN', 'value': {
            'stackable': {'state': 'KNOWN', 'value': True},
            'stack_max': {'state': 'KNOWN', 'value': 50}}}
        gold = copy.deepcopy(next(row for row in self.document['records']
                                  if row['production_definition']['production_key'] == 'oteryn:item.tibia.i3031'))
        self.generated[PROFILE] = json.dumps(self.document).encode()
        self.admit()
        self.assertEqual(self.generated_record('oteryn:item.tibia.i237'), before)
        self.assertEqual(self.generated_record('oteryn:item.tibia.i3031'), gold)
        once = self.generated[PROFILE]
        self.admit()
        self.assertEqual(self.generated[PROFILE], once)

    def test_present_identity_admission_weight_and_stack_holds_fail_closed(self):
        self.select_potion()
        for mutation, error in (
                ('revision', 'NATIVE_IDENTITY'), ('binding', 'NATIVE_IDENTITY'),
                ('held', 'NATIVE_ITEM_NOT_ADMITTED'), ('weight', 'NATIVE_PHYSICAL_INCOMPLETE'),
                ('stack', 'NATIVE_STACK_INCOMPLETE')):
            document = copy.deepcopy(self.document)
            row = next(row for row in document['records']
                       if row['production_definition']['production_key'] == 'oteryn:item.tibia.i237')
            if mutation == 'revision': row['production_definition']['revision_ref'] = 'invented'
            elif mutation == 'binding': row['production_binding']['disposition'] = 'invented'
            elif mutation == 'held': row['admission']['materializable'] = False
            elif mutation == 'weight': row['semantics']['physical']['value']['weight'] = {'state': 'UNKNOWN'}
            else:
                row['semantics']['stack'] = {'state': 'KNOWN', 'value': {
                    'stackable': {'state': 'KNOWN', 'value': True},
                    'stack_max': {'state': 'NOT_APPLICABLE'}}}
            self.generated[PROFILE] = json.dumps(document).encode()
            with self.subTest(mutation=mutation), self.assertRaisesRegex(ValueError, error):
                self.admit()

    def test_unbounded_writable_chest_leaf_grants_without_player_write_authority(self):
        key = 'oteryn:item.tibia.i2820'
        definition = writing.reading.item_rows(ROOT)[key]
        before = copy.deepcopy(definition)
        text = definition['semantics']['readable_writeable']['value']
        self.assertEqual(text['writeable'], {'state': 'KNOWN', 'value': True})
        self.assertEqual(text['max_text_length'], {'state': 'UNKNOWN'})
        # Force the production path that adds a new native profile even when
        # the committed artifact already contains this quest-text carrier.
        self.callbacks['callbacks'][0]['offerings'].append({'item': key, 'count': 1})
        self.document['records'] = [row for row in self.document['records']
                                    if row['production_definition']['production_key'] != key]
        self.generated[PROFILE] = json.dumps(self.document).encode()
        self.admit()
        row = self.generated_record(key)
        expected = copy.deepcopy(definition['semantics'])
        expected['readable_writeable']['value']['writeable'] = {'state': 'UNKNOWN'}
        self.assertEqual(row['semantics'], expected)
        self.assertEqual(row['admission'], {'materializable': True,
                         'stack_class': 'NonStackable',
                         'legal_destinations': ['CharacterInventory', 'Ground']})
        self.assertEqual(writing.reading.item_rows(ROOT)[key], before)
        once = self.generated[PROFILE]
        self.admit()
        self.assertEqual(self.generated[PROFILE], once)

    def test_changed_reference_digest_refuses_admission(self):
        with patch.object(profiles, 'SOURCE_SHA256', '0' * 64):
            with self.assertRaisesRegex(ValueError, 'QUEST_CALLBACK_SOURCE_DIGEST'):
                self.admit()


if __name__ == '__main__':
    unittest.main()
