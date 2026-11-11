"""M07 uses actual pinned item facts; source and custody drift fail closed."""
import copy
import json
import unittest
from pathlib import Path
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
        cls.taxonomy = {row['target']['key']: row['source_taxonomy'] for row in
                        json.loads((ROOT / 'content/items/taxonomy/items.json').read_bytes())['records']}

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
            key = f'oteryn:item.tibia.i{item_id}'
            expected = profiles.qualify(*self.values(item_id))
            expected['authoring']['taxonomy'] = self.taxonomy[key]
            self.assertEqual(records[key], expected)

    def test_native_provider_preserves_quick_loot_taxonomy_once_and_is_repeatable(self):
        import authored_static_item_constructors as authored
        import quest_callback_profiles as provider
        import quests_2_emperor_item_profiles as emperor
        import quick_loot_item_profiles as quick_loot

        profile = 'content/items/definitions/spell-native-profiles.json'
        manifest = 'content/items/native-manifest.json'
        original = json.loads((ROOT / profile).read_bytes())
        rebuilt = {f'oteryn:item.tibia.i{i}' for i in
                   profiles.SOURCE_RECORDS.keys() | quick_loot.IDS | provider.CHEST_IDS | set(emperor.IDS)}
        original['records'] = [row for row in original['records']
                               if row['production_definition']['production_key'] not in rebuilt]
        generated = {profile: json.dumps(original).encode(),
                      manifest: b'{"item_profiles":{"sha256":"pending"}}',
                      authored.ADMISSIONS: (ROOT / authored.ADMISSIONS).read_bytes()}
        order = lambda row: row['production_definition']['production_key']
        encode = lambda value: (json.dumps(value, sort_keys=True) + '\n').encode()
        # Execute the real preceding owners: QENGINE creates these three profiles,
        # then Quick Loot attaches the canonical taxonomy to every present profile.
        packet = shared.capture(ROOT, item_ids={5467, 21389, 21466})
        shared.admit_records(ROOT, generated, profile, manifest, order, encode, packet['records'])
        quick_loot.admit(ROOT, generated, profile, manifest, order, encode)
        emperor.admit(ROOT, generated, profile, manifest, order, encode)
        preceding = {row['production_definition']['production_key']: row
                     for row in json.loads(generated[profile])['records']}
        self.assertNotIn('oteryn:item.tibia.i3595', preceding)
        for item_id in (5467, 21389, 21466):
            key = f'oteryn:item.tibia.i{item_id}'
            self.assertEqual(preceding[key]['authoring']['taxonomy'], self.taxonomy[key])
        def admit():
            provider.admit(ROOT, generated, profile, manifest, order, encode)
        admit()
        first = generated[profile]
        document = json.loads(first)
        for item_id in profiles.SOURCE_RECORDS:
            key = f'oteryn:item.tibia.i{item_id}'
            rows = [row for row in document['records']
                    if row['production_definition']['production_key'] == key]
            expected = profiles.qualify(*self.values(item_id))
            expected['authoring']['taxonomy'] = self.taxonomy[key]
            self.assertEqual(rows, [expected])
        self.assertEqual(json.loads(generated[manifest])['item_profiles']['sha256'],
                         owner.hashlib.sha256(first).hexdigest())
        admit()
        self.assertEqual(generated[profile], first)
        for change in ('taxonomy', 'weight', 'unexpected_authoring'):
            with self.subTest(change=change):
                changed = copy.deepcopy(document)
                flower = next(row for row in changed['records']
                              if row['production_definition']['production_key'] == 'oteryn:item.tibia.i21389')
                if change == 'taxonomy':
                    flower['authoring']['taxonomy'] = {'primary': 'Tools'}
                elif change == 'weight':
                    flower['semantics']['physical']['value']['weight']['value'] += 1
                else:
                    flower['authoring']['unqualified'] = True
                generated[profile] = json.dumps(changed).encode()
                with self.assertRaisesRegex(ValueError, 'LION_TRIAL_NATIVE_PROFILE_DRIFT'):
                    admit()

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
