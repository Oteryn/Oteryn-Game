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
        original['records'] = [before.get(row['production_definition']['production_key'], row)
                               for row in original['records']]
        generated = {profile: json.dumps(original).encode(),
                     manifest: b'{"item_profiles":{"sha256":"pending"}}'}
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
            values[4][str(item_id)]['target']['key'] = 'oteryn:item.tibia.i3595'
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
