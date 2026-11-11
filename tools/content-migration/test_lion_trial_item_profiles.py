"""M07 uses actual pinned item facts; source and custody drift fail closed."""
import copy
import json
import sys
import unittest
from pathlib import Path
from unittest.mock import patch
import xml.etree.ElementTree as ET

import grizzly_shop_profiles as owner
import lion_trial_item_profiles as profiles
import qengine_reward_item_profiles as shared

ROOT = Path(__file__).resolve().parents[2]


class LionTrialItemProfiles(unittest.TestCase):
    @classmethod
    def setUpClass(cls):
        cls.xml = (ROOT / 'imports/canary/items-xml/items.xml').read_bytes()
        cls.nodes = owner.source.xml_index(cls.xml)
        raw = (ROOT / shared.CLIENT).read_bytes()
        if owner.hashlib.sha256(raw).hexdigest() != shared.CLIENT_SHA:
            raise ValueError('LION_TEST_CLIENT_SOURCE')
        cls.appearances = owner.source.engine_items.load_appearance_objects(raw)
        cls.physical = {r['server_item_id']: r for r in owner.source.policies(
            raw, cls.xml, set(profiles.SOURCE_RECORDS))}
        cls.bindings = owner.source.binding_index(
            (ROOT / 'imports/crystalserver/bindings/items.json').read_bytes())
        cls.definitions = {r['definition']['identity']['key']: r['definition']
                           for r in json.loads((ROOT / 'content/items/definitions/'
                                                'items-10500-10999.json').read_bytes())['records']}
        cls.definitions.update({r['definition']['identity']['key']:r['definition']
                                for r in json.loads((ROOT / 'content/items/definitions/'
                                                     'items-29000-29499.json').read_bytes())['records']})

        cls.definitions.update({r['definition']['identity']['key']:r['definition']
                                for r in json.loads((ROOT / 'content/items/definitions/'
                                                     'items-21500-21999.json').read_bytes())['records']})

    def values(self, item_id):
        return copy.deepcopy([item_id, self.nodes[item_id], self.appearances[item_id],
                              self.physical[item_id], self.bindings,
                              self.definitions[f'oteryn:item.tibia.i{item_id}']])

    def test_flower_retires_after_one_hour_without_inventing_a_decay_target(self):
        record = profiles.qualify(*self.values(21389))
        self.assertEqual(record['semantics']['temporal'], owner.source.K({
            'consumption_mode': owner.source.K('DURABLE_ABSOLUTE_DEADLINE'),
            'duration': owner.source.K(3_600_000), 'stop_duration': owner.source.K(False),
            'decay_target': owner.source.NA}))
        self.assertEqual(record['production_definition']['production_key'],
                         'oteryn:item.tibia.i21389')

    def test_carrot_is_a_bounded_plain_stack_for_the_source_offering(self):
        record = profiles.qualify(*self.values(3595))
        self.assertEqual(record['admission'],{'materializable':True,'stack_class':'StackCapable',
                                             'legal_destinations':['CharacterInventory','Ground']})
        self.assertEqual(record['semantics']['stack'],owner.source.K({
            'stackable':owner.source.K(True),'stack_max':owner.source.K(100)}))
        self.assertEqual(record['semantics']['charges'],owner.source.NA)
        self.assertEqual(record['semantics']['temporal'],owner.source.NA)

    def test_fire_bug_is_a_reusable_uncharged_portable_instance(self):
        record = profiles.qualify(*self.values(5467))
        self.assertEqual(record['production_definition']['production_key'],'oteryn:item.tibia.i5467')
        self.assertEqual(record['semantics']['temporal'],owner.source.NA)
        self.assertEqual(record['semantics']['charges'],owner.source.NA)
        self.assertEqual(record['semantics']['container'],owner.source.NA)

    def test_water_description_cannot_manufacture_a_timer_or_fluid_instance(self):
        record = profiles.qualify(*self.values(21466))
        self.assertEqual(record['semantics']['temporal'], owner.source.NA)
        self.assertEqual(record['semantics']['charges'], owner.source.NA)
        self.assertEqual(record['semantics']['container'], owner.source.NA)
        self.assertEqual(record['admission'], {
            'materializable': True, 'stack_class': 'NonStackable',
            'legal_destinations': ['CharacterInventory', 'Ground']})

    def test_retained_source_resolver_returns_the_complete_exact_pair(self):
        records = profiles.records(ROOT, self.xml, self.definitions)
        self.assertEqual(set(records), {f'oteryn:item.tibia.i{i}' for i in profiles.SOURCE_RECORDS})
        for item_id in profiles.SOURCE_RECORDS:
            self.assertEqual(records[f'oteryn:item.tibia.i{item_id}'],
                             profiles.qualify(*self.values(item_id)))

    def test_native_provider_contains_exact_records_once_and_is_repeatable(self):
        import authored_static_item_constructors as authored
        import quest_callback_profiles as provider

        profile = 'content/items/definitions/spell-native-profiles.json'
        manifest = 'content/items/native-manifest.json'
        original = json.loads((ROOT / profile).read_bytes())
        original['records'] = [row for row in original['records']
                               if row['production_definition']['production_key']
                               not in {f'oteryn:item.tibia.i{i}' for i in profiles.SOURCE_RECORDS}]
        # Match the immediately preceding producer stage in register_spell_families.
        # The final published provider also carries later, unrelated overlays.
        import quests_2_emperor_item_profiles as emperor
        packet = json.loads((ROOT / emperor.PATH).read_bytes())
        before = {row['production_definition']['production_key']: row
                  for row in packet['records']}
        carriers = json.loads((ROOT / provider.CHEST_PROFILES).read_bytes())['records']
        carrier_rows = {row['production_definition']['production_key']: row for row in carriers}
        self.assertEqual(set(carrier_rows), {f'oteryn:item.tibia.i{item_id}'
                                            for item_id in (1777, 2436, 2438, 2473, 2474)})
        self.assertEqual(len(carriers), 5)
        self.assertTrue(before.keys().isdisjoint(carrier_rows))
        before.update(carrier_rows)
        original['records'] = [before.get(row['production_definition']['production_key'], row)
                               for row in original['records']]
        generated = {profile: json.dumps(original).encode(),
                     manifest: b'{"item_profiles":{"sha256":"pending"}}',
                     authored.ADMISSIONS: (ROOT / authored.ADMISSIONS).read_bytes()}
        def admit():
            provider.admit(ROOT, generated, profile, manifest,
                           lambda row: row['production_definition']['production_key'],
                           lambda value: (json.dumps(value, sort_keys=True) + '\n').encode())
        admit()
        first = generated[profile]
        document = json.loads(first)
        for item_id in profiles.SOURCE_RECORDS:
            key = f'oteryn:item.tibia.i{item_id}'
            rows = [row for row in document['records']
                    if row['production_definition']['production_key'] == key]
            self.assertEqual(rows, [profiles.qualify(*self.values(item_id))])
        self.assertEqual(json.loads(generated[manifest])['item_profiles']['sha256'],
                         owner.hashlib.sha256(first).hexdigest())
        admit()
        self.assertEqual(generated[profile], first)
        flower = next(row for row in document['records']
                      if row['production_definition']['production_key'] == 'oteryn:item.tibia.i21389')
        flower['semantics']['physical']['value']['weight']['value'] += 1
        generated[profile] = json.dumps(document).encode()
        with self.assertRaisesRegex(ValueError, 'LION_TRIAL_NATIVE_PROFILE_DRIFT'):
            admit()

    def test_native_profile_conflict_diagnostic_preserves_exact_refusal(self):
        import authored_static_item_constructors as authored
        import quest_callback_profiles as provider
        from qualification_inputs import appearance_input_scope

        profile = 'content/items/definitions/spell-native-profiles.json'
        manifest = 'content/spells.manifest.json'
        qualified = {f'oteryn:item.tibia.i{item_id}': profiles.qualify(*self.values(item_id))
                     for item_id in profiles.SOURCE_RECORDS}
        generated = {
            profile: json.dumps({'schema': 'OTERYN_NATIVE_ITEM_PROFILES/v1',
                                 'records': list(qualified.values())}).encode(),
            manifest: b'{"item_profiles":{"sha256":"pending"}}',
            authored.ADMISSIONS: (ROOT / authored.ADMISSIONS).read_bytes(),
        }

        def admit():
            provider.admit(ROOT, generated, profile, manifest,
                           lambda row: row['production_definition']['production_key'],
                           lambda value: (json.dumps(value, sort_keys=True) + '\n').encode())

        # Isolate the selected cohort, retaining the actual source, binding,
        # canonical definition, map-carrier and authored-admission checks.
        with patch.object(provider, '_requested_inputs',
                          return_value=(set(qualified), set(), [])), patch.object(
                              sys, 'path', [str(ROOT / 'tools/content-schema/reward-claim-authoring'),
                                            *sys.path]), appearance_input_scope():
            admit()
            accepted = copy.deepcopy(generated)
            actual = {row['production_definition']['production_key']: row
                      for row in json.loads(generated[profile])['records']}
            self.assertEqual({key: actual[key] for key in qualified}, qualified)
            admit()
            self.assertEqual(generated, accepted)
            for mutation in ('weight', 'extra_attributes', 'binding'):
                with self.subTest(mutation=mutation):
                    generated = copy.deepcopy(accepted)
                    document = json.loads(generated[profile])
                    key = 'oteryn:item.tibia.i21389'
                    row = next(row for row in document['records']
                               if row['production_definition']['production_key'] == key)
                    if mutation == 'weight':
                        row['semantics']['physical']['value']['weight']['value'] += 1
                    elif mutation == 'extra_attributes':
                        row['attributes']['unqualified_lion_attribute'] = True
                    else:
                        row['production_binding']['target']['key'] = 'oteryn:item.tibia.i3031'
                    expected_existing = copy.deepcopy(row)
                    generated[profile] = json.dumps(document).encode()
                    before = copy.deepcopy(generated)
                    with self.assertRaises(ValueError) as first:
                        admit()
                    self.assertEqual(generated, before)
                    message = str(first.exception)
                    self.assertTrue(message.startswith('LION_TRIAL_NATIVE_PROFILE_DRIFT:' + key + '\n'))
                    lines = message.splitlines(keepends=True)
                    self.assertEqual(lines[1], '--- existing/' + key + '\n')
                    self.assertEqual(lines[2], '+++ qualified/' + key + '\n')
                    body = [line for line in lines[3:] if not line.startswith('@@')]
                    existing = ''.join(line[1:] for line in body if line[0] in ' -')
                    expected = ''.join(line[1:] for line in body if line[0] in ' +')
                    self.assertEqual(json.loads(existing), expected_existing)
                    self.assertEqual(json.loads(expected), qualified[key])
                    with self.assertRaises(ValueError) as repeated:
                        admit()
                    self.assertEqual(str(repeated.exception), message)
                    self.assertEqual(generated, before)

    def quick_loot_fixture(self):
        import authored_static_item_constructors as authored
        import quest_callback_profiles as provider
        import quick_loot_item_profiles as quick_loot

        qualified = {f'oteryn:item.tibia.i{item_id}': profiles.qualify(*self.values(item_id))
                     for item_id in profiles.SOURCE_RECORDS}
        earlier = set(qualified) & {f'oteryn:item.tibia.i{item_id}' for item_id in shared.IDS}
        self.assertEqual(earlier, {f'oteryn:item.tibia.i{item_id}'
                                   for item_id in (5467, 21389, 21466)})
        profile = shared.PROFILE
        manifest = 'content/spells.manifest.json'
        generated = {
            profile: json.dumps({'schema': 'OTERYN_NATIVE_ITEM_PROFILES/v1',
                                 'records': [qualified[key] for key in sorted(earlier)]}).encode(),
            manifest: b'{"item_profiles":{"sha256":"pending"}}',
            authored.ADMISSIONS: (ROOT / authored.ADMISSIONS).read_bytes(),
        }
        order = lambda row: row['production_definition']['production_key']
        canonical = lambda value: (json.dumps(value, sort_keys=True) + '\n').encode()
        quick_loot.admit(ROOT, generated, profile, manifest, order, canonical)
        predecessor = {row['production_definition']['production_key']: row
                       for row in json.loads(generated[profile])['records']}
        self.assertEqual(set(predecessor), earlier | {
            f'oteryn:item.tibia.i{item_id}' for item_id in quick_loot.IDS})

        def admit():
            provider.admit(ROOT, generated, profile, manifest, order, canonical)

        return qualified, generated, predecessor, admit

    def test_prior_quick_loot_taxonomy_is_preserved_from_canonical_owner(self):
        import quest_callback_profiles as provider
        from qualification_inputs import appearance_input_scope

        qualified, generated, predecessor, admit = self.quick_loot_fixture()
        taxonomy = json.loads((ROOT / 'content/items/taxonomy/items.json').read_bytes())
        expected = copy.deepcopy(qualified)
        for key in qualified.keys() & predecessor.keys():
            row, = [row for row in taxonomy['records'] if row['target']['key'] == key]
            self.assertEqual(row['target'], self.definitions[key]['identity'])
            expected[key]['authoring']['taxonomy'] = copy.deepcopy(row['source_taxonomy'])
            self.assertEqual(predecessor[key], expected[key])
        self.assertEqual(expected['oteryn:item.tibia.i21389']['authoring']['taxonomy'],
                         {'primary': 'Quest Items'})
        self.assertNotIn('taxonomy', expected['oteryn:item.tibia.i3595']['authoring'])

        with patch.object(provider, '_requested_inputs',
                          return_value=(set(qualified), set(), [])), patch.object(
                              sys, 'path', [str(ROOT / 'tools/content-schema/reward-claim-authoring'),
                                            *sys.path]), appearance_input_scope():
            admit()
            accepted = copy.deepcopy(generated)
            actual = {row['production_definition']['production_key']: row
                      for row in json.loads(generated[shared.PROFILE])['records']}
            self.assertEqual({key: actual[key] for key in qualified}, expected)
            self.assertEqual({key: actual[key] for key in predecessor}, predecessor)
            admit()
            self.assertEqual(generated, accepted)
            for mutation in ('wrong_taxonomy', 'null_taxonomy', 'extra_authoring', 'weight'):
                with self.subTest(mutation=mutation):
                    generated.clear()
                    generated.update(copy.deepcopy(accepted))
                    document = json.loads(generated[shared.PROFILE])
                    row = next(row for row in document['records']
                               if row['production_definition']['production_key']
                               == 'oteryn:item.tibia.i21389')
                    if mutation == 'wrong_taxonomy':
                        row['authoring']['taxonomy'] = {'primary': 'Tools'}
                    elif mutation == 'null_taxonomy':
                        row['authoring']['taxonomy'] = None
                    elif mutation == 'extra_authoring':
                        row['authoring']['use_ability'] = 'unqualified'
                    else:
                        row['semantics']['physical']['value']['weight']['value'] += 1
                    generated[shared.PROFILE] = json.dumps(document).encode()
                    before = copy.deepcopy(generated)
                    with self.assertRaisesRegex(ValueError, 'LION_TRIAL_NATIVE_PROFILE_DRIFT'):
                        admit()
                    self.assertEqual(generated, before)

    def test_prior_taxonomy_requires_one_exact_canonical_target(self):
        import quest_callback_profiles as provider
        from qualification_inputs import appearance_input_scope

        qualified, generated, _, admit = self.quick_loot_fixture()
        taxonomy_path = ROOT / 'content/items/taxonomy/items.json'
        taxonomy = json.loads(taxonomy_path.read_bytes())
        read_bytes = Path.read_bytes
        with patch.object(provider, '_requested_inputs',
                          return_value=(set(qualified), set(), [])), patch.object(
                              sys, 'path', [str(ROOT / 'tools/content-schema/reward-claim-authoring'),
                                            *sys.path]), appearance_input_scope():
            for mutation in ('missing', 'duplicate', 'wrong_family', 'wrong_revision'):
                with self.subTest(mutation=mutation):
                    changed = copy.deepcopy(taxonomy)
                    row, = [row for row in changed['records']
                            if row['target']['key'] == 'oteryn:item.tibia.i21389']
                    if mutation == 'missing':
                        changed['records'].remove(row)
                    elif mutation == 'duplicate':
                        changed['records'].append(copy.deepcopy(row))
                    elif mutation == 'wrong_family':
                        row['target']['family'] = 'Creature'
                    else:
                        row['target']['revision'] = 'unqualified'
                    raw = json.dumps(changed).encode()

                    def mutated_read_bytes(path):
                        return raw if path == taxonomy_path else read_bytes(path)

                    before = copy.deepcopy(generated)
                    with patch.object(Path, 'read_bytes', mutated_read_bytes):
                        with self.assertRaisesRegex(ValueError, 'LION_TRIAL_ITEM_TAXONOMY'):
                            admit()
                    self.assertEqual(generated, before)

    def test_top_order_selection_rejects_unqualified_item_keys(self):
        keys = {f'oteryn:item.tibia.i{item}' for item in profiles.SOURCE_RECORDS}
        with self.assertRaisesRegex(ValueError, 'LION_TRIAL_ITEM_TOP_ORDER_COHORT'):
            profiles.records(ROOT, self.xml, self.definitions,
                             top_order_keys=keys | {'oteryn:item.tibia.i3031'})

    def test_house_top_order_survives_two_native_registration_cycles(self):
        import authored_static_item_constructors as authored
        import house_decoration_actions as house
        import quest_callback_profiles as provider
        import quick_loot_item_profiles as quick_loot
        from qualification_inputs import appearance_input_scope

        keys = {f'oteryn:item.tibia.i{item_id}' for item_id in profiles.SOURCE_RECORDS}
        selected = set(profiles.SOURCE_RECORDS) & shared.IDS
        self.assertEqual(selected, {5467, 21389, 21466})
        profile = shared.PROFILE
        manifest = 'content/spells.manifest.json'
        read_bytes = Path.read_bytes
        order = lambda row: row['production_definition']['production_key']
        canonical = lambda value: (json.dumps(value, sort_keys=True) + '\n').encode()

        def admit(generated):
            provider.admit(ROOT, generated, profile, manifest, order, canonical)

        def cycle(prior=None):
            def native_read_bytes(path):
                return prior if prior is not None and path == ROOT / profile else read_bytes(path)

            # Only the predecessor's native bytes change between real captures;
            # all canonical definitions, source pins and bindings stay actual.
            with patch.object(Path, 'read_bytes', native_read_bytes):
                captured = shared.capture(ROOT, item_ids=selected)
            generated = {
                profile: json.dumps({'schema': 'OTERYN_NATIVE_ITEM_PROFILES/v1',
                                     'records': captured['records']}).encode(),
                manifest: b'{"item_profiles":{"sha256":"pending"}}',
                authored.ADMISSIONS: (ROOT / authored.ADMISSIONS).read_bytes(),
            }
            quick_loot.admit(ROOT, generated, profile, manifest, order, canonical)
            admit(generated)
            return generated

        with patch.object(provider, '_requested_inputs',
                          return_value=(keys, set(), [])), patch.object(
                              sys, 'path', [str(ROOT / 'tools/content-schema/reward-claim-authoring'),
                                            *sys.path]), appearance_input_scope():
            overlay = json.loads((ROOT / house.PROFILES).read_bytes())
            house_appearance = {
                'reference': 'canary',
                'revision': '47dfd51f45280a59a1d3e50ba7edd573d7234446',
                'path': 'data/items/appearances.dat',
                'sha256': 'aa44a154f30c7ed59acc25f246286396e4043851ef0b54ef3cf3951e46d1ce50',
            }
            self.assertEqual([source for source in overlay['sources']
                              if source['reference'] == 'canary'], [house_appearance])
            self.assertEqual(overlay['appearance_sha256'], house_appearance['sha256'])
            self.assertNotEqual(overlay['appearance_sha256'], shared.CLIENT_SHA)
            # The two pinned files differ as a whole. Their exact four source
            # flags agree, so the accepted House projection is reproducible
            # from the independently verified appearance input used by Lion.
            orders = {item: next((value for flag, value in
                                  (('clip', 1), ('top', 3), ('bottom', 2))
                                  if self.appearances[item]['flags'].get('flags.' + flag, False)), 0)
                      for item in profiles.SOURCE_RECORDS}
            self.assertEqual(orders, {item: 0 for item in profiles.SOURCE_RECORDS})
            self.assertEqual(orders, {item: overlay['always_on_top'][str(item)]
                                      for item in profiles.SOURCE_RECORDS})

            # A published provider can already contain House's exact optional
            # pair. Prove each complete Lion row from its pinned source, then
            # supply the raw predecessor explicitly for the first cycle.
            predecessor = json.loads((ROOT / profile).read_bytes())
            prior_rows = {row['production_definition']['production_key']: row
                          for row in predecessor['records']}
            self.assertEqual(len(prior_rows), len(predecessor['records']))
            self.assertTrue(keys <= prior_rows.keys())
            taxonomy = json.loads((ROOT / 'content/items/taxonomy/items.json').read_bytes())
            replacements = {}
            for item_id in profiles.SOURCE_RECORDS:
                key = f'oteryn:item.tibia.i{item_id}'
                raw = profiles.qualify(*self.values(item_id))
                current = prior_rows[key]
                if 'taxonomy' in current['authoring']:
                    matching = [row for row in taxonomy['records']
                                if row['target'] == self.definitions[key]['identity']]
                    self.assertEqual(len(matching), 1)
                    raw['authoring']['taxonomy'] = copy.deepcopy(matching[0]['source_taxonomy'])
                attributes = current['attributes']
                top_fields = {'always_on_top', 'always_on_top_order'} & attributes.keys()
                self.assertIn(top_fields, (set(), {'always_on_top', 'always_on_top_order'}))
                expected = copy.deepcopy(raw)
                if top_fields:
                    self.assertIs(attributes['always_on_top'], orders[item_id] != 0)
                    self.assertIs(type(attributes['always_on_top_order']), int)
                    self.assertEqual(attributes['always_on_top_order'], orders[item_id])
                    expected['attributes'].update(always_on_top=orders[item_id] != 0,
                                                  always_on_top_order=orders[item_id])
                self.assertEqual(current, expected)
                replacements[key] = raw
            outside_before = {key: copy.deepcopy(row) for key, row in prior_rows.items()
                              if key not in keys}
            predecessor['records'] = [
                replacements.get(row['production_definition']['production_key'], row)
                for row in predecessor['records']]
            self.assertEqual({row['production_definition']['production_key']: row
                              for row in predecessor['records']
                              if row['production_definition']['production_key'] not in keys},
                             outside_before)
            first = cycle(json.dumps(predecessor).encode())
            original = {row['production_definition']['production_key']: row
                        for row in json.loads(first[profile])['records']}
            for key in keys:
                self.assertNotIn('always_on_top', original[key]['attributes'])
                self.assertNotIn('always_on_top_order', original[key]['attributes'])
            house.admit(ROOT, first, profile, manifest, order, canonical)
            after_house = {row['production_definition']['production_key']: row
                           for row in json.loads(first[profile])['records']}
            for key in keys:
                expected = copy.deepcopy(original[key])
                expected['attributes'].update(always_on_top=False, always_on_top_order=0)
                self.assertEqual(after_house[key], expected)

            second = cycle(first[profile])
            accepted = copy.deepcopy(second)
            actual = {row['production_definition']['production_key']: row
                      for row in json.loads(second[profile])['records']}
            for item in selected:
                key = f'oteryn:item.tibia.i{item}'
                self.assertEqual(actual[key], after_house[key])
            self.assertEqual(actual['oteryn:item.tibia.i3595'], original['oteryn:item.tibia.i3595'])
            house.admit(ROOT, second, profile, manifest, order, canonical)
            self.assertEqual(json.loads(second[profile]), json.loads(first[profile]))

            for mutation in ('wrong_top', 'wrong_order', 'missing_top', 'missing_order',
                             'null_top', 'extra_attribute'):
                with self.subTest(mutation=mutation):
                    changed = copy.deepcopy(accepted)
                    document = json.loads(changed[profile])
                    key = 'oteryn:item.tibia.i21389'
                    row = next(row for row in document['records']
                               if row['production_definition']['production_key'] == key)
                    if mutation == 'wrong_top':
                        row['attributes']['always_on_top'] = True
                    elif mutation == 'wrong_order':
                        row['attributes']['always_on_top_order'] = 1
                    elif mutation == 'missing_top':
                        row['attributes'].pop('always_on_top')
                    elif mutation == 'missing_order':
                        row['attributes'].pop('always_on_top_order')
                    elif mutation == 'null_top':
                        row['attributes']['always_on_top'] = None
                    else:
                        row['attributes']['unqualified_lion_attribute'] = True
                    changed[profile] = json.dumps(document).encode()
                    before = copy.deepcopy(changed)
                    with self.assertRaisesRegex(ValueError, 'LION_TRIAL_NATIVE_PROFILE_DRIFT') as error:
                        admit(changed)
                    self.assertEqual(changed, before)
                    lines = str(error.exception).splitlines(keepends=True)
                    self.assertEqual(lines[2], '+++ qualified/' + key + '\n')
                    expected = ''.join(line[1:] for line in lines[3:]
                                       if not line.startswith('@@') and line[0] in ' +')
                    self.assertEqual(json.loads(expected), after_house[key])

    def test_source_duration_and_custom_action_changes_are_refused(self):
        for item_id in profiles.SOURCE_RECORDS:
            for key, value in [('duration', '1'), ('script', 'custom')]:
                values = self.values(item_id)
                ET.SubElement(values[1], 'attribute', key=key, value=value)
                with self.assertRaisesRegex(ValueError, 'LION_TRIAL_ITEM_SOURCE'):
                    profiles.qualify(*values)

    def test_canonical_weight_and_binding_cannot_be_replaced(self):
        for item_id in profiles.SOURCE_RECORDS:
            values = self.values(item_id)
            values[-1]['semantics']['physical']['value']['weight']['value'] += 1
            with self.assertRaisesRegex(ValueError, 'CANONICAL_CONFLICT'):
                profiles.qualify(*values)
            values = self.values(item_id)
            values[4][str(item_id)]['target']['key'] = 'oteryn:item.tibia.i3031'
            with self.assertRaisesRegex(ValueError, 'LION_TRIAL_ITEM_BINDING'):
                profiles.qualify(*values)

    def test_fluid_container_and_immovable_appearance_are_refused(self):
        for item_id in profiles.SOURCE_RECORDS:
            values = self.values(item_id)
            values[2]['flags']['flags.liquidcontainer'] = True
            with self.assertRaisesRegex(ValueError, 'LION_TRIAL_ITEM_FLUID'):
                profiles.qualify(*values)
            values = self.values(item_id)
            values[3]['movable'] = False
            with self.assertRaisesRegex(ValueError, 'LION_TRIAL_ITEM_CUSTODY'):
                profiles.qualify(*values)


if __name__ == '__main__':
    unittest.main()
