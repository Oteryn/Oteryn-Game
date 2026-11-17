"""Newhaven item identity and compatibility regression tests."""
import copy
import json
from pathlib import Path
import unittest

import quest_finish_newhaven_items as newhaven


class NewhavenItemTests(unittest.TestCase):
    def test_exact_current_client_proves_all_selected_portable_names(self):
        newhaven.verify_client(Path(__file__).resolve().parents[2])

    def test_native_mount_preserves_existing_ranged_authoring(self):
        root = Path(__file__).resolve().parents[2]
        profiles = 'content/items/definitions/spell-native-profiles.json'
        manifest = 'content/spells.manifest.json'
        generated = {path: (root / path).read_bytes() for path in (profiles, manifest)}
        before = {row['production_definition']['production_key']: row for row in
                  json.loads(generated[profiles])['records']}
        newhaven.apply_native(root, generated, profiles, manifest,
                              lambda row: row['production_definition']['production_key'],
                              lambda value: (json.dumps(value) + '\n').encode())
        after = {row['production_definition']['production_key']: row for row in
                 json.loads(generated[profiles])['records']}
        for item in newhaven.SPEC:
            key = f'oteryn:item.tibia.i{item}'
            self.assertTrue(after[key]['admission']['materializable'])
            self.assertEqual(after[key]['production_binding']['target']['key'], key)
            equipment = after[key]['semantics']['equipment']
            if equipment['state'] == 'KNOWN':
                for pattern in equipment['value']['patterns']['value']:
                    vocations = pattern['vocations']
                    if vocations['state'] == 'KNOWN':
                        # ReferenceBaseVocation discriminants define loader order.
                        order = ['DRUID', 'KNIGHT', 'MONK', 'PALADIN', 'SORCERER', 'NONE']
                        self.assertEqual(vocations['value'], sorted(set(vocations['value']), key=order.index))
        key = 'oteryn:item.tibia.i3066'
        self.assertEqual(after[key]['authoring']['ranged'], before[key]['authoring']['ranged'])
        for item in newhaven.CORPSE_CHAIN:
            key = f'oteryn:item.tibia.i{item}'
            self.assertEqual(after[key]['admission'], {
                'materializable': True, 'stack_class': 'NonStackable',
                'legal_destinations': ['Ground'],
            })
            self.assertEqual(after[key]['production_binding']['target']['key'], key)

    def test_five_vocations_use_current_named_wiki_identities(self):
        self.assertEqual(newhaven.CORRUPTOR_ITEMS, {
            'knight': (3359, 3285, 3375), 'paladin': (3571, 51760),
            'sorcerer': (3074, 7991), 'druid': (3066, 7991),
            'monk': (50195, 51119),
        })

    def test_guaranteed_gear_has_complete_plain_equipment_requirements(self):
        for item in {item for items in newhaven.CORRUPTOR_ITEMS.values() for item in items}:
            with self.subTest(item=item):
                definition = {'identity': {'family': 'Item', 'key': f'oteryn:item.tibia.i{item}'}}
                self.assertTrue(newhaven.apply(definition))
                semantics = definition['semantics']
                self.assertEqual(semantics['container'], {'state': 'NOT_APPLICABLE'})
                pattern, = semantics['equipment']['value']['patterns']['value']
                self.assertEqual(pattern['level'], newhaven.known(6 if item in (3066, 3074) else 0))
                self.assertEqual(pattern['vocations'], newhaven.known(list(newhaven.SPEC[item][3])))

    def test_ordinary_mushroom_mount_preserves_canonical_stack_food_and_weight(self):
        root = Path(__file__).resolve().parents[2]
        key = 'oteryn:item.tibia.i3723'
        canonical = next(row['definition']
            for path in (root / 'content/items/definitions').glob('items-*.json')
            for row in json.loads(path.read_bytes())['records']
            if row['definition']['identity']['key'] == key)
        profiles = 'content/items/definitions/spell-native-profiles.json'
        manifest = 'content/spells.manifest.json'
        generated = {path: (root / path).read_bytes() for path in (profiles, manifest)}
        # Exercise a fresh mount; the existing-owner case below separately
        # requires byte-identical preservation of its richer source facts.
        document = json.loads(generated[profiles])
        document['records'] = []
        generated[profiles] = json.dumps(document).encode()
        newhaven.apply_native(root, generated, profiles, manifest,
            lambda row: row['production_definition']['production_key'],
            lambda value: (json.dumps(value) + '\n').encode())
        selected = [row for row in json.loads(generated[profiles])['records']
                    if row['production_definition']['production_key'] == key]
        self.assertEqual(len(selected), 1)
        self.assertEqual(selected[0]['semantics'], canonical['semantics'])
        self.assertEqual(selected[0]['production_binding']['target'], canonical['identity'])
        self.assertEqual(selected[0]['admission'], {'materializable': True,
            'stack_class': 'StackCapable', 'legal_destinations': ['CharacterInventory', 'Ground']})
        self.assertEqual(selected[0]['semantics']['stack']['value']['stack_max'], newhaven.known(100))
        self.assertEqual(selected[0]['semantics']['physical']['value']['weight'], newhaven.known(40))

    def test_complete_corruptor_chain_projects_source48_to_native30(self):
        self.assertEqual(newhaven.CORPSE_SOURCE_CAPACITY, 48)
        for item in (52113, 52114, 52115, 52116):
            definition = {'identity': {'family': 'Item', 'key': f'oteryn:item.tibia.i{item}'}}
            self.assertTrue(newhaven.apply(definition))
            sem = definition['semantics']
            self.assertEqual(sem['physical']['value']['movable'], newhaven.known(False))
            self.assertEqual(sem['physical']['value']['pickupable'], newhaven.known(False))
            self.assertTrue(definition['materializable'])
            if item != 52116:
                self.assertEqual(sem['container']['value']['capacity'], newhaven.known(30))
            else:
                self.assertEqual(sem['container'], {'state': 'NOT_APPLICABLE'})

    def test_staff_appearance_is_explicitly_a_separate_donor_namespace(self):
        self.assertEqual(newhaven.APPEARANCE_TRANSLATIONS[51166], 51119)
        staff = {'identity': {'family': 'Item', 'key': 'oteryn:item.tibia.i51119'}}
        newhaven.apply(staff)
        self.assertEqual(staff['identity']['key'], 'oteryn:item.tibia.i51119')
        self.assertTrue(staff['materializable'])
        self.assertEqual(staff['semantics']['equipment']['value']['patterns']['value'][0]
                         ['additional_reserved_slots']['value'], ['SHIELD'])

    def test_unrelated_historical_starter_item_is_preserved(self):
        original = {'identity': {'family': 'Item', 'key': 'oteryn:item.tibia.i50166'},
                    'materializable': False, 'historical': ['unchanged']}
        definition = copy.deepcopy(original)
        self.assertFalse(newhaven.apply(definition))
        self.assertEqual(definition, original)

    def test_wrong_named_identity_is_rejected_before_any_mutation(self):
        definition = {'identity': {'family': 'Item', 'key': 'oteryn:item.tibia.i3571'},
                      'semantics': {'presentation': newhaven.known({'name': newhaven.known('mammoth tusk')})}}
        original = copy.deepcopy(definition)
        with self.assertRaisesRegex(ValueError, 'NEWHAVEN_ITEM_NAME'):
            newhaven.apply(definition)
        self.assertEqual(definition, original)

    def test_apply_is_idempotent_and_preserves_other_semantic_groups(self):
        definition = {'identity': {'family': 'Item', 'key': 'oteryn:item.tibia.i3359'},
                      'semantics': {'protection': {'custom': 'preserve'}}}
        self.assertTrue(newhaven.apply(definition))
        original = copy.deepcopy(definition)
        self.assertTrue(newhaven.apply(definition))
        self.assertEqual(definition, original)
        self.assertEqual(definition['semantics']['protection'], {'custom': 'preserve'})
        self.assertEqual(definition['stack_class'], 'NonStackable')
        self.assertTrue(definition['materializable'])

    def test_existing_ordinary_loot_owner_is_preserved_and_conflicts_refuse_atomically(self):
        root = Path(__file__).resolve().parents[2]
        profiles = 'content/items/definitions/spell-native-profiles.json'
        manifest = 'content/spells.manifest.json'
        planned = {path: (root / path).read_bytes() for path in (profiles, manifest)}
        key = 'oteryn:item.tibia.i3723'
        before = next(row for row in json.loads(planned[profiles])['records']
                      if row['production_definition']['production_key'] == key)
        order = lambda row: row['production_definition']['production_key']
        canonical = lambda value: json.dumps(value).encode()
        newhaven.apply_native(root, planned, profiles, manifest, order, canonical)
        after = next(row for row in json.loads(planned[profiles])['records']
                     if row['production_definition']['production_key'] == key)
        self.assertEqual(after, before)
        corrupt = json.loads(planned[profiles])
        row = next(row for row in corrupt['records']
                   if row['production_definition']['production_key'] == key)
        row['semantics']['physical']['value']['weight']['value'] += 1
        planned[profiles] = canonical(corrupt)
        untouched = dict(planned)
        with self.assertRaisesRegex(ValueError, 'NEWHAVEN_NATIVE_LOOT_CONFLICT'):
            newhaven.apply_native(root, planned, profiles, manifest, order, canonical)
        self.assertEqual(planned, untouched)


if __name__ == '__main__':
    unittest.main()
