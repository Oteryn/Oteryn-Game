"""Ordinary loot follows exact active native custody without changing drop tables."""
import copy
import json
import unittest

import quest_loot_profiles as loot
import build_full_spell_artifact as artifact


class NativeQuestLootFacts(unittest.TestCase):
    @classmethod
    def setUpClass(cls):
        cls.creatures = json.loads((loot.ROOT / loot.corpse.CREATURES).read_bytes())
        cls.tables = {}
        for path in sorted(artifact.LOOT_DEFINITIONS.glob('loot-*.json')):
            for row in json.loads(path.read_bytes())['records']:
                cls.tables[row['definition']['identity']['key']] = row['definition']
        cls.facts = artifact.item_facts()
        cls.profiles = json.loads((loot.ROOT / loot.PROFILE).read_bytes())
        cls.native = {row['production_definition']['production_key']: row for row in cls.profiles['records']}
        cls.bindings = {row['target']['key']: row for row in json.loads(
            (loot.ROOT / 'imports/crystalserver/bindings/items.json').read_bytes())['bindings']}

    def test_actual_animal_loot_bridge_preserves_all_entries_and_source_quantities(self):
        required = loot.required_items(loot.ROOT, self.creatures, self.tables)
        expected = {f'oteryn:item.tibia.i{number}' for number in
                    [7398, 3413, 3410, 9657, 9689, 3336, 3355, 3277]}
        animal_expected = expected.copy()
        expected |= {f'oteryn:item.tibia.i{number}' for number in
                     [281, 3006, 3036, 3037, 3038, 5809, 5892, 5904, 7414, 7427, 23529, 23531, 23533, 23542, 23543, 23544, 25759, 30192, 30195, 30275, 30276, 30277, 30278, 30279, 30280, 30281, 30282, 30283, 30284, 30285, 30318, 30319, 30320, 30321, 30322]}
        # Main now admits these seven canonical Items before native projection.
        # Model the pre-projection admission view this bridge test exercises,
        # retaining every source quantity and all unrelated canonical facts.
        baseline = copy.deepcopy(self.facts)
        already_admitted = {f'oteryn:item.tibia.i{number}' for number in
                            [281, 3036, 3037, 3038, 3277, 3355, 3413]}
        self.assertTrue(all(baseline[key]['materializable'] for key in already_admitted))
        for key in already_admitted:
            baseline[key]['materializable'] = False
        missing = {key for key in required if not baseline[key]['materializable']}
        self.assertEqual(missing, expected)
        facts, tables = copy.deepcopy(baseline), copy.deepcopy(self.tables)
        loot.apply_loot_facts(facts, loot.ROOT, self.creatures, tables, self.profiles)
        self.assertTrue(all(facts[key]['materializable'] for key in required))
        self.assertEqual(tables, self.tables)
        changed = {key for key in facts if facts[key] != baseline[key]}
        self.assertEqual(changed, expected)
        for key in animal_expected:
            expected_fact = copy.deepcopy(baseline[key])
            expected_fact['materializable'] = True
            self.assertEqual(facts[key], expected_fact)
        before = copy.deepcopy(facts)
        loot.apply_loot_facts(facts, loot.ROOT, self.creatures, tables, self.profiles)
        self.assertEqual(facts, before)

    def test_full_typed_identity_binding_and_native_qualification_are_mandatory(self):
        key = 'oteryn:item.tibia.i7398'
        for mutation in ['revision', 'binding', 'admission', 'physical', 'stack', 'container', 'charges']:
            fact, native, binding = copy.deepcopy(self.facts[key]), copy.deepcopy(self.native[key]), copy.deepcopy(self.bindings[key])
            if mutation == 'revision':
                native['production_definition']['revision_ref'] = 'definition-r2'
            elif mutation == 'binding':
                binding['disposition'] = 'HYPOTHESIS'
            elif mutation == 'admission':
                native['admission']['materializable'] = False
            elif mutation == 'physical':
                native['semantics']['physical']['value']['pickupable']['value'] = False
            elif mutation == 'stack':
                native['semantics']['stack']['value']['stackable']['value'] = True
            elif mutation == 'container':
                native['semantics']['container'] = {'state': 'UNKNOWN'}
            else:
                native['semantics']['charges'] = {'state': 'UNKNOWN'}
            with self.subTest(mutation=mutation), self.assertRaises(ValueError):
                loot.project(fact, native, binding)

    def test_missing_native_admission_refuses_without_partial_cohort_mutation(self):
        profiles = copy.deepcopy(self.profiles)
        profiles['records'] = [row for row in profiles['records']
                               if row['production_definition']['production_key'] != 'oteryn:item.tibia.i9689']
        facts = copy.deepcopy(self.facts)
        with self.assertRaisesRegex(ValueError, 'NATIVE_ADMISSION_MISSING'):
            loot.apply_loot_facts(facts, loot.ROOT, self.creatures, self.tables, profiles)
        self.assertEqual(facts, self.facts)

    def test_source_table_identity_substitution_is_refused(self):
        tables = copy.deepcopy(self.tables)
        tables['oteryn:loot.creature.cyclops']['identity']['revision'] = 'definition-r2'
        with self.assertRaisesRegex(ValueError, 'EXACT_TABLE_BINDING'):
            loot.required_items(loot.ROOT, self.creatures, tables)

    def test_charge_projection_requires_the_exact_qualified_full_definition(self):
        key = 'oteryn:item.tibia.i3054'
        fact = copy.deepcopy(self.facts[key])
        projected = loot.project(fact, self.native[key], self.bindings[key])
        self.assertEqual(projected['charges'], 200)
        fact['charges'] = 199
        with self.assertRaisesRegex(ValueError, 'ACCEPTED_CHARGE_CONFLICT'):
            loot.project(fact, self.native[key], self.bindings[key])

    def test_ordinary_source_charge_absence_cannot_replace_an_existing_charge_fact(self):
        key = 'oteryn:item.tibia.i30318'
        fact = copy.deepcopy(self.facts[key])
        with self.assertRaisesRegex(ValueError, 'CHARGE_POLICY_CONFLICT'):
            loot.project(fact, self.native[key], self.bindings[key])
        projected = loot.project(fact, self.native[key], self.bindings[key], source_charges_absent=True)
        self.assertTrue(projected['materializable'])
        self.assertNotIn('charges', projected)
        fact['charges'] = 1
        with self.assertRaisesRegex(ValueError, 'CHARGE_POLICY_CONFLICT'):
            loot.project(fact, self.native[key], self.bindings[key], source_charges_absent=True)


if __name__ == '__main__':
    unittest.main()
