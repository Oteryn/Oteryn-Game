"""Rebuild callback Item admissions against retained, pinned source inputs."""
import copy
import hashlib
import json
import tempfile
import unittest
from pathlib import Path
from unittest.mock import patch

import quest_callback_profiles as profiles
import timed_item_profiles as timed
import writable_item_profiles as writing

ROOT = Path(__file__).resolve().parents[2]
PROFILE = 'content/items/definitions/spell-native-profiles.json'
MANIFEST = 'content/items/native-manifest.json'
KEY = 'oteryn:item.tibia.i7934'
BLOODY_TEAR = 'oteryn:item.tibia.i32594'


class QuestCallbackProfiles(unittest.TestCase):
    def setUp(self):
        self.temp = tempfile.TemporaryDirectory(prefix='.qcb3-profile-', dir=ROOT)
        self.addCleanup(self.temp.cleanup)
        self.root = Path(self.temp.name)
        (self.root / 'content/quests').mkdir(parents=True)
        (self.root / 'content/interactions').symlink_to(ROOT / 'content/interactions', target_is_directory=True)
        (self.root / 'content/items').symlink_to(ROOT / 'content/items', target_is_directory=True)
        (self.root / 'content/assets').symlink_to(ROOT / 'content/assets', target_is_directory=True)
        (self.root / 'imports').symlink_to(ROOT / 'imports', target_is_directory=True)
        (self.root / 'tools').symlink_to(ROOT / 'tools', target_is_directory=True)
        self.callbacks = json.loads((ROOT / 'content/quests/world-callbacks.json').read_bytes())
        (self.root / 'content/quests/killing-in-the-name-of.json').write_bytes(
            (ROOT / 'content/quests/killing-in-the-name-of.json').read_bytes())
        self.document = json.loads((ROOT / PROFILE).read_bytes())
        self.document['records'] = [r for r in self.document['records']
                                    if r['production_definition']['production_key'] not in
                                    (KEY, 'oteryn:item.tibia.i6120', 'oteryn:item.tibia.i9019',
                                     BLOODY_TEAR, 'oteryn:item.tibia.i9388', 'oteryn:item.tibia.i2973')]
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

    def test_fathers_burden_materials_have_exact_native_carrying_facts(self):
        import fathers_burden_item_profiles as father
        self.admit()
        rows = {row['production_definition']['production_key']: row
                for row in json.loads(self.generated[PROFILE])['records']}
        for item, (name, weight) in father.ITEMS.items():
            row = rows[f'oteryn:item.tibia.i{item}']
            self.assertEqual(row['admission'], {'materializable': True,
                             'stack_class': 'NonStackable',
                             'legal_destinations': ['CharacterInventory', 'Ground']})
            self.assertEqual(row['semantics']['physical']['value']['weight'],
                             {'state': 'KNOWN', 'value': weight})
            self.assertEqual(row['semantics']['presentation']['value']['name'],
                             {'state': 'KNOWN', 'value': name})
            self.assertEqual(row['production_binding']['disposition'], 'EXACT')

    def test_jorge_all_replica_choices_and_twenty_token_input_are_carryable(self):
        # Crystal9f5a72c data-global/npc/jorge.lua: source-ordered37offers.
        outputs = [19151, 28897, 23682, 28895, 43511, 25981, 23679, 22027,
                   31265, 25979, 25980, 23678, 19153, 19156, 28896, 23681,
                   34210, 23676, 23683, 31268, 19152, 37058, 31267, 37059,
                   25982, 23680, 23677, 22028, 19150, 28894, 19155, 22026,
                   31266, 43517, 37060, 19154, 43510]
        self.assertEqual(len(set(outputs)), 37)
        self.admit()
        rows = {row['production_definition']['production_key']: row
                for row in json.loads(self.generated[PROFILE])['records']}
        for item in [19083, *outputs]:
            key = f'oteryn:item.tibia.i{item}'
            self.assertTrue(key in rows, f'{key}: source input/output needs native custody')
            row = rows[key]
            self.assertTrue(row['admission']['materializable'])
            self.assertIn('CharacterInventory', row['admission']['legal_destinations'])
            physical = row['semantics']['physical']['value']
            self.assertEqual(physical['weight']['state'], 'KNOWN')
            self.assertEqual(physical['movable'], {'state': 'KNOWN', 'value': True})
            self.assertEqual(physical['pickupable'], {'state': 'KNOWN', 'value': True})
            self.assertEqual(row['admission']['stack_class'],
                             'StackCapable' if item == 19083 else 'NonStackable')
        token = rows['oteryn:item.tibia.i19083']['semantics']['stack']['value']
        self.assertEqual(token['stack_max'], {'state': 'KNOWN', 'value': 100})
        for item, weight in [(23677, 2700), (34210, 6660)]:
            self.assertEqual(rows[f'oteryn:item.tibia.i{item}']['semantics']['physical']['value']['weight'],
                             {'state': 'KNOWN', 'value': weight})

    def test_jorge_rejects_changes_to_known_canonical_weight_and_stack_class(self):
        import raid_tokens_item_profiles as native
        import xml.etree.ElementTree as ET
        definitions = {row['definition']['identity']['key']: row['definition']
                       for path in (ROOT / 'content/items/definitions').glob('items-*.json')
                       for row in json.loads(path.read_bytes())['records']
                       if row['definition']['identity']['key'] in native.KEYS}
        sources = ET.fromstring((ROOT / profiles.SOURCE).read_bytes()).findall('item')
        changed = copy.deepcopy(definitions)
        changed['oteryn:item.tibia.i23677']['semantics']['physical']['value']['weight']['value'] += 1
        with self.assertRaisesRegex(ValueError, 'RAID_TOKENS_CANONICAL_FACTS:23677'):
            native.definitions(self.root, changed, sources)
        changed = copy.deepcopy(definitions)
        changed['oteryn:item.tibia.i19151']['stack_class'] = 'StackCapable'
        with self.assertRaisesRegex(ValueError, 'RAID_TOKENS_STACK_CONFLICT:19151'):
            native.definitions(self.root, changed, sources)

    def test_fathers_burden_rejects_known_weight_drift(self):
        import fathers_burden_item_profiles as father
        import xml.etree.ElementTree as ET
        definitions = {row['definition']['identity']['key']: row['definition']
                       for path in (ROOT / 'content/items/definitions').glob('items-*.json')
                       for row in json.loads(path.read_bytes())['records']}
        sources = ET.fromstring((ROOT / profiles.SOURCE).read_bytes()).findall('item')
        definitions['oteryn:item.tibia.i11548']['semantics']['physical']['value']['weight']['value'] += 1
        with self.assertRaisesRegex(ValueError, 'FATHERS_BURDEN_CANONICAL_FACTS'):
            father.definitions(self.root, definitions, sources)

    def test_marlin_exchange_has_exact_portable_native_fish_and_trophy(self):
        self.admit()
        rows = {row['production_definition']['production_key']: row
                for row in json.loads(self.generated[PROFILE])['records']}
        for item, weight, stack in ((901, 1800, 'StackCapable'), (902, 500, 'NonStackable')):
            row = rows[f'oteryn:item.tibia.i{item}']
            self.assertEqual(row['admission'], {'materializable': True,
                             'stack_class': stack,
                             'legal_destinations': ['CharacterInventory', 'Ground']})
            self.assertEqual(row['semantics']['physical']['value']['weight'],
                             {'state': 'KNOWN', 'value': weight})
            self.assertEqual(row['production_binding']['disposition'], 'EXACT')
        self.assertEqual(rows['oteryn:item.tibia.i901']['semantics']['stack']['value'],
                         {'stackable': {'state': 'KNOWN', 'value': True},
                          'stack_max': {'state': 'KNOWN', 'value': 100}})

    def test_marlin_stack_supersession_rejects_changed_canonical_record(self):
        import marlin_trophy_item_profiles as marlin
        import xml.etree.ElementTree as ET
        definitions = {row['definition']['identity']['key']: row['definition']
                       for path in (ROOT / 'content/items/definitions').glob('items-*.json')
                       for row in json.loads(path.read_bytes())['records']}
        sources = ET.fromstring((ROOT / profiles.SOURCE).read_bytes()).findall('item')
        definitions['oteryn:item.tibia.i901']['stack_class'] = 'Unknown'
        with self.assertRaisesRegex(ValueError, 'MARLIN_CANONICAL_STACK_SUPERSESSION'):
            marlin.definitions(self.root, definitions, sources)

    def test_marlin_trophy_rejects_known_weight_drift(self):
        import marlin_trophy_item_profiles as marlin
        import xml.etree.ElementTree as ET
        definitions = {row['definition']['identity']['key']: row['definition']
                       for path in (ROOT / 'content/items/definitions').glob('items-*.json')
                       for row in json.loads(path.read_bytes())['records']}
        sources = ET.fromstring((ROOT / profiles.SOURCE).read_bytes()).findall('item')
        definitions['oteryn:item.tibia.i902']['semantics']['physical']['value']['weight']['value'] += 1
        with self.assertRaisesRegex(ValueError, 'MARLIN_CANONICAL_FACTS'):
            marlin.definitions(self.root, definitions, sources)

    def test_hydra_egg_has_pinned_native_stack_and_weight(self):
        self.admit()
        rows = {row['production_definition']['production_key']: row
                for row in json.loads(self.generated[PROFILE])['records']}
        row = rows['oteryn:item.tibia.i4839']
        self.assertEqual(row['admission'], {'materializable': True,
                         'stack_class': 'StackCapable',
                         'legal_destinations': ['CharacterInventory', 'Ground']})
        self.assertEqual(row['semantics']['physical']['value']['weight'],
                         {'state': 'KNOWN', 'value': 2000})
        self.assertEqual(row['semantics']['stack']['value'], {
            'stackable': {'state': 'KNOWN', 'value': True},
            'stack_max': {'state': 'KNOWN', 'value': 100}})
        self.assertEqual(row['production_binding']['disposition'], 'EXACT')

    def test_quest_chest_rewards_preserve_canonical_item_admission(self):
        self.admit()
        rows = {r['production_definition']['production_key']: r
                for r in json.loads(self.generated[PROFILE])['records']}
        for key in ('oteryn:item.tibia.i6120', 'oteryn:item.tibia.i9019', 'oteryn:item.tibia.i2973'):
            row = rows[key]
            self.assertEqual(row['admission'], {'materializable': True,
                             'stack_class': 'NonStackable',
                             'legal_destinations': ['CharacterInventory', 'Ground']})
            self.assertEqual(row['production_binding']['disposition'], 'EXACT')
            self.assertEqual(row['production_binding']['target']['key'], key)
            self.assertEqual(row['semantics']['physical']['value']['pickupable'],
                             {'state': 'KNOWN', 'value': True})
        self.assertEqual(rows['oteryn:item.tibia.i2973']['semantics']['physical']['value']['weight'],
                         {'state': 'KNOWN', 'value': 100})

    def test_demon_helmet_chest_reuses_canonical_equipment_without_new_owner(self):
        self.admit()
        rows = {row['production_definition']['production_key']: row
                for row in json.loads(self.generated[PROFILE])['records']}
        key = 'oteryn:item.tibia.i3387'
        self.assertTrue(key in rows, 'Demon Helmet ready chest must qualify its existing Item identity')
        canonical = next(row['definition']
                         for path in sorted((ROOT / 'content/items/definitions').glob('items-*.json'))
                         for row in json.loads(path.read_bytes())['records']
                         if row['definition']['identity']['key'] == key)
        self.assertEqual(rows[key]['semantics'], canonical['semantics'])
        self.assertEqual(rows[key]['semantics']['protection']['value']['armor'],
                         {'state': 'KNOWN', 'value': 10})
        self.assertEqual(rows[key]['semantics']['physical']['value']['weight'],
                         {'state': 'KNOWN', 'value': 2950})
        pattern = rows[key]['semantics']['equipment']['value']['patterns']['value'][0]
        self.assertEqual(pattern['primary_slot'], {'state': 'KNOWN', 'value': 'HEAD'})
        self.assertEqual(rows[key]['production_binding']['target'], canonical['identity'])
        self.assertEqual(rows[key]['production_binding']['disposition'], 'EXACT')
        self.assertEqual(rows[key]['admission'], {'materializable': True,
                         'stack_class': 'NonStackable',
                         'legal_destinations': ['CharacterInventory', 'Ground']})

    def test_every_ready_reward_leaf_has_its_exact_native_item_profile(self):
        self.admit()
        rows = {row['production_definition']['production_key']: row
                for row in json.loads(self.generated[PROFILE])['records']}
        expected = {
            item['item']['key']
            for path in sorted((ROOT / 'content/interactions/reward_claims').glob('reward-claim*.json'))
            for row in json.loads(path.read_bytes())['records']
            if row['definition'].get('readiness') == 'ready'
            and row['definition'].get('quest', {}).get('key') != 'canary:quest/dawnport_quest'
            for placement in row['definition']['placements']
            for item in placement['reward'].get('items', [])
        }
        self.assertEqual(sorted(expected - rows.keys()), [])
        for key in expected:
            self.assertTrue(rows[key]['admission']['materializable'], key)
            self.assertEqual(rows[key]['production_binding']['target']['key'], key)
            self.assertEqual(rows[key]['production_binding']['disposition'], 'EXACT')
        self.assertEqual(rows['oteryn:item.tibia.i5952']['semantics']['physical']['value']['weight'],
                         {'state': 'KNOWN', 'value': 120})

    def test_rotten_blood_rust_callback_admits_the_canonical_bloody_tear_once(self):
        # Rotten Blood's fixed callback is authored in Rust, so the JSON callback
        # inventory cannot supply this already admitted source Item by itself.
        self.assertFalse(any(
            (callback.get('item_on_target') or {}).get('item') == BLOODY_TEAR
            or any(offering['item'] == BLOODY_TEAR for offering in callback['offerings'])
            for callback in self.callbacks['callbacks']))
        self.admit()
        rows = [row for row in json.loads(self.generated[PROFILE])['records']
                if row['production_definition']['production_key'] == BLOODY_TEAR]
        self.assertEqual(len(rows), 1)
        canonical = [row['definition']
                     for path in sorted((ROOT / 'content/items/definitions').glob('items-*.json'))
                     for row in json.loads(path.read_bytes())['records']
                     if row['definition']['identity']['key'] == BLOODY_TEAR]
        self.assertEqual(len(canonical), 1)
        definition = canonical[0]
        identity = definition['identity']
        self.assertEqual(rows[0]['production_definition'], {
            'family': identity['family'], 'production_key': BLOODY_TEAR,
            'revision_ref': identity['revision']})
        self.assertEqual(rows[0]['semantics'], definition['semantics'])
        self.assertEqual(rows[0]['admission'], {
            'materializable': True, 'stack_class': definition['stack_class'],
            'legal_destinations': ['CharacterInventory', 'Ground']})
        self.assertEqual(rows[0]['production_binding']['disposition'], 'EXACT')
        self.assertEqual(rows[0]['production_binding']['target'], identity)
        self.assertEqual(str(rows[0]['production_binding']['external_id']), '32594')
        first = self.generated[PROFILE]
        self.admit()
        self.assertEqual(self.generated[PROFILE], first)

    def test_existing_chest_children_keep_canonical_food_and_potion_consumption(self):
        self.admit()
        rows = {row['production_definition']['production_key']: row
                for row in json.loads(self.generated[PROFILE])['records']}
        for number in (238, 6569, 6574, 7377):
            key = f'oteryn:item.tibia.i{number}'
            with self.subTest(key=key):
                self.assertEqual(rows[key]['semantics']['consumption']['state'], 'KNOWN')

    def test_chest_children_preserve_timed_ring_ability_ownership(self):
        generated = {PROFILE: b'{"records":[]}',
                     MANIFEST: b'{"item_profiles":{"sha256":"pending"}}'}
        timed.admit(ROOT, generated, PROFILE, MANIFEST,
                    lambda r: r['production_definition']['production_key'],
                    lambda v: json.dumps(v, sort_keys=True).encode())
        timed_rows = {r['production_definition']['production_key']: r
                      for r in json.loads(generated[PROFILE])['records']}
        for i, row in enumerate(self.document['records']):
            key = row['production_definition']['production_key']
            if key in timed_rows:
                self.document['records'][i] = copy.deepcopy(timed_rows[key])
        self.generated[PROFILE] = json.dumps(self.document).encode()
        self.admit()
        rows = {r['production_definition']['production_key']: r
                for r in json.loads(self.generated[PROFILE])['records']}
        for key, expected in timed_rows.items():
            with self.subTest(key=key):
                self.assertEqual(rows[key]['semantics']['skill_modifiers'],
                                 expected['semantics']['skill_modifiers'])

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
        # Complete charged/fluid/key/text recipes add thirty identities to the
        # original 130-Item closure, including the fixed Holy Icon entitlement.
        self.assertEqual(len(expected), 160)
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
        with (patch.object(profiles, 'SOURCE_SHA256', '0' * 64),
              self.assertRaisesRegex(ValueError, 'QUEST_CALLBACK_SOURCE_DIGEST')):
            self.admit()


if __name__ == '__main__':
    unittest.main()
