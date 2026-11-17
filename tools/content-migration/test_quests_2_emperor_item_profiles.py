"""The exact quest items cross the existing native Item import boundary."""
import copy
import json
import os
import unittest
from pathlib import Path

import quests_2_emperor_item_profiles as profiles


class EmperorItemProfiles(unittest.TestCase):
    @classmethod
    def setUpClass(cls):
        cls.root = Path(os.environ.get('OTERYN_EMPEROR_TEST_ROOT', profiles.ROOT))
        crystal = os.environ.get('OTERYN_EMPEROR_CRYSTAL')
        cls.packet = (profiles.capture(Path(crystal), cls.root) if crystal
                      else json.loads((cls.root / profiles.PATH).read_bytes()))

    def test_source_letterbag_is_portable_nonstackable_without_invented_capacity(self):
        row = next(row for row in self.packet['records'] if row['production_definition']['production_key'] == 'oteryn:item.tibia.i3217')
        self.assertEqual(row['production_definition']['production_key'], 'oteryn:item.tibia.i3217')
        self.assertEqual(row['admission'], {
            'materializable': True, 'stack_class': 'NonStackable',
            'legal_destinations': ['CharacterInventory', 'Ground'],
        })
        self.assertEqual(row['semantics']['container'], {'state': 'NOT_APPLICABLE'})
        self.assertEqual(row['semantics']['physical']['value']['weight'],
                         {'state': 'KNOWN', 'value': 50000})
        # The donor appearance has clothes.slot=3. Keep the accepted equipment
        # fact even though it is not a container; a backpack capacity is not inferred.
        self.assertEqual(row['semantics']['equipment']['value']['patterns']['value'][0]
                         ['primary_slot'], {'state': 'KNOWN', 'value': 'CONTAINER'})

    def test_dragon_fetish_has_exact_plain_custody_and_no_charges_or_lifetime(self):
        row = next(row for row in self.packet['records']
                   if row['production_definition']['production_key'] == 'oteryn:item.tibia.i3206')
        self.assertEqual(row['admission'], {'materializable': True,
            'stack_class': 'NonStackable', 'legal_destinations': ['CharacterInventory', 'Ground']})
        self.assertEqual(row['semantics']['physical']['value']['weight'],
                         {'state': 'KNOWN', 'value': 490})
        for group in ('container', 'charges', 'temporal'):
            self.assertEqual(row['semantics'][group], {'state': 'NOT_APPLICABLE'})

    def test_three_dragon_tears_use_actual_source_stack_limit(self):
        row = next(row for row in self.packet['records'] if row['production_definition']['production_key'] == 'oteryn:item.tibia.i19084')
        self.assertEqual(row['production_definition']['production_key'], 'oteryn:item.tibia.i19084')
        self.assertEqual(row['admission']['stack_class'], 'StackCapable')
        self.assertEqual(row['semantics']['stack'], {'state': 'KNOWN', 'value': {
            'stackable': {'state': 'KNOWN', 'value': True},
            'stack_max': {'state': 'KNOWN', 'value': 100},
        }})
        self.assertEqual(row['semantics']['physical']['value']['weight'],
                         {'state': 'KNOWN', 'value': 10})

    def test_admission_adds_only_selected_quest_items_and_pins_real_bytes(self):
        generated = {
            'profiles': profiles.encode({'schema': 'OTERYN_NATIVE_ITEM_PROFILES/v1', 'records': []}),
            'manifest': profiles.encode({'item_profiles': {'sha256': 'before'}}),
        }
        profiles.admit_packet(self.packet, self.root, generated, 'profiles', 'manifest',
                              lambda row: int(row['production_binding']['external_id']),
                              profiles.encode)
        actual = json.loads(generated['profiles'])
        self.assertEqual([r['production_binding']['external_id'] for r in actual['records']],
                         ['3206', '3217', '19084'])
        self.assertEqual(json.loads(generated['manifest'])['item_profiles']['sha256'],
                         profiles.digest(generated['profiles']))

    def test_existing_authority_conflict_fails_closed(self):
        existing = copy.deepcopy(next(row for row in self.packet['records'] if row['production_definition']['production_key'] == 'oteryn:item.tibia.i19084'))
        existing['semantics']['physical']['value']['weight']['value'] = 11
        generated = {
            'profiles': profiles.encode({'schema': 'OTERYN_NATIVE_ITEM_PROFILES/v1', 'records': [existing]}),
            'manifest': profiles.encode({'item_profiles': {'sha256': 'before'}}),
        }
        before = copy.deepcopy(generated)
        with self.assertRaisesRegex(ValueError, 'EXISTING_PROFILE_CONFLICT'):
            profiles.admit_packet(self.packet, self.root, generated, 'profiles', 'manifest',
                                  lambda row: int(row['production_binding']['external_id']),
                                  profiles.encode)
        self.assertEqual(generated, before)

    def test_wrong_donor_pin_or_extra_item_is_not_admitted(self):
        for field, value in (('source_revision', 'other'), ('item_ids', [3206, 3217, 19084, 18928])):
            changed = copy.deepcopy(self.packet)
            changed[field] = value
            with self.assertRaisesRegex(ValueError, 'PROFILE_PROVENANCE'):
                profiles.validate_packet(changed, self.root)

    def test_source_packet_cannot_replace_accepted_identity(self):
        changed = copy.deepcopy(self.packet)
        changed['records'][0]['production_definition']['production_key'] = 'oteryn:item.tibia.i2859'
        with self.assertRaisesRegex(ValueError, 'PROFILE_ITEM_SET'):
            profiles.validate_packet(changed, self.root)

    def test_callback_qualification_preserves_canonical_facts_and_needs_exact_native_record(self):
        definitions, _ = profiles.definitions(self.root)
        for row in self.packet['records']:
            key = row['production_definition']['production_key']
            original = copy.deepcopy(definitions[key])
            actual = profiles.callback_definition(self.packet, row, original, self.root)
            self.assertEqual(original, definitions[key])
            self.assertTrue(actual['materializable'])
            self.assertEqual(actual['stack_class'], row['admission']['stack_class'])
            self.assertEqual(actual['identity'], original['identity'])
            for group in original['semantics']:
                if group in ('physical', 'stack', 'container'):
                    self.assertFalse(profiles.accepted.known_conflict(
                        original['semantics'][group], actual['semantics'][group]))
                else:
                    self.assertEqual(actual['semantics'][group], original['semantics'][group])
            changed = copy.deepcopy(row)
            changed['attributes']['speed_bonus'] = 1
            with self.assertRaisesRegex(ValueError, 'CALLBACK_NATIVE_PROFILE'):
                profiles.callback_definition(self.packet, changed, original, self.root)

    def test_callback_qualification_cannot_use_changed_canonical_or_unrelated_item(self):
        definitions, _ = profiles.definitions(self.root)
        row = self.packet['records'][0]
        changed = copy.deepcopy(definitions[row['production_definition']['production_key']])
        changed['semantics']['physical']['value']['weight'] = {'state': 'KNOWN', 'value': 1}
        with self.assertRaisesRegex(ValueError, 'CALLBACK_CANONICAL'):
            profiles.callback_definition(self.packet, row, changed, self.root)
        unrelated = {'identity': {'key': 'oteryn:item.tibia.i18928'}, 'materializable': False}
        self.assertIs(profiles.callback_definition(self.packet, None, unrelated, self.root), unrelated)


if __name__ == '__main__':
    unittest.main()
