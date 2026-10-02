#!/usr/bin/env python3
"""Dependency selection tests: advanced/ambiguous donor data stays disabled."""
import copy
import unittest
from build_monster_melee_profiles import select_melee, melee_skill_range


def fixture():
    ability_ref = {'key': 'canary/rat/attack-1', 'revision': 'donor'}
    effect_ref = {'key': 'effect/rat/bite', 'revision': 'donor'}
    formula_ref = {'key': 'formula/rat/bite', 'revision': 'donor'}
    monster = {'behavior': {'attacks': [
        {'ability': ability_ref, 'interval_ms': 2000, 'chance_percent': 75}]}}
    dependencies = {
        'abilities': [{'identity': ability_ref, 'kind': 'melee', 'range_tiles': 1, 'effects': [effect_ref]}],
        'effects': [{'identity': effect_ref, 'operation': 'damage', 'damage_type': 'physical', 'formula': formula_ref}],
        'formulas': [{'identity': formula_ref, 'kind': 'range', 'magnitude': {'minimum': 0, 'maximum': 8}}],
    }
    behavior = {'attacks': [{'ability': {'key': 'creature.rat.attack-1', 'revision': 'active'},
                            'interval_ms': 2000, 'chance_ppm': 750000}]}
    return monster, dependencies, behavior


class MeleeClosureSelection(unittest.TestCase):
    def test_active_reference_and_donor_reference_are_preserved(self):
        inputs = fixture()
        before = copy.deepcopy(inputs)
        selected = select_melee(*inputs)
        self.assertIsNotNone(selected)
        self.assertEqual(selected[1]['ability']['revision'], 'active')
        self.assertEqual(selected[2]['ability']['revision'], 'donor')
        self.assertEqual(inputs, before)

    def test_advanced_or_incomplete_closures_are_disabled(self):
        def extra_effect(d):
            d['abilities'][0]['effects'].append({'key': 'condition', 'revision': 'donor'})
        for mutate in [extra_effect,
                       lambda d: d['formulas'][0].update(kind='skill'),
                       lambda d: d['effects'][0].update(damage_type='fire'),
                       lambda d: d['abilities'][0].update(range_tiles=3),
                       lambda d: d['formulas'].clear(),
                       lambda d: d['formulas'][0]['magnitude'].update(maximum=0)]:
            with self.subTest(mutate=mutate):
                monster, dependencies, behavior = fixture()
                mutate(dependencies)
                self.assertIsNone(select_melee(monster, dependencies, behavior))

    def test_schedule_slot_and_source_interval_must_match(self):
        for mutate in [lambda b: b['attacks'][0].update(interval_ms=1000),
                       lambda b: b['attacks'][0].update(chance_ppm=1000000),
                       lambda b: b['attacks'][0]['ability'].update(key='creature.rat.attack-2'),
                       lambda b: b['attacks'].clear()]:
            with self.subTest(mutate=mutate):
                monster, dependencies, behavior = fixture()
                mutate(behavior)
                self.assertIsNone(select_melee(monster, dependencies, behavior))

    def test_melee_skill_endpoints_match_source_binary64_ceil_and_refuse_bad_inputs(self):
        self.assertEqual(melee_skill_range({'melee': {'attack': 70, 'skill': 66}}), (0, 266))
        self.assertEqual(melee_skill_range({'melee': {'attack': 61, 'skill': 30}}), (0, 123))
        self.assertEqual(melee_skill_range({'melee': {'attack': 1, 'skill': 1}}), (0, 1))
        for value in [0, -1, True, 1.5, float('inf'), 0x80000000]:
            with self.subTest(value=value):
                self.assertIsNone(melee_skill_range({'melee': {'attack': value, 'skill': 30}}))
                self.assertIsNone(melee_skill_range({'melee': {'attack': 61, 'skill': value}}))
        self.assertIsNone(melee_skill_range({'melee': {'attack': 0x7fffffff, 'skill': 0x7fffffff}}))

    def test_source_skill_formula_is_preserved_and_requires_explicit_qualified_lane(self):
        monster, dependencies, behavior = fixture()
        dependencies['formulas'][0] = {'identity': dependencies['formulas'][0]['identity'],
                                      'kind': 'melee_attack_skill', 'melee': {'attack': 61, 'skill': 30}}
        original = copy.deepcopy(dependencies)
        self.assertIsNone(select_melee(monster, dependencies, behavior))
        selected = select_melee(monster, dependencies, behavior, allow_skill=True)
        self.assertEqual(selected[6:8], (0, 123))
        self.assertEqual(selected[5]['kind'], 'melee_attack_skill')
        self.assertEqual(dependencies, original)

    def test_secondary_conditions_are_retained_disabled_and_unknown_effects_refuse(self):
        monster, dependencies, behavior = fixture()
        ref = {'key': 'effect/rat/poison', 'revision': 'donor'}
        condition = {'identity': ref, 'operation': 'condition', 'condition': {'type': 'poison'}}
        dependencies['effects'].append(condition)
        dependencies['abilities'][0]['effects'].append(ref)
        self.assertIsNone(select_melee(monster, dependencies, behavior))
        selected = select_melee(monster, dependencies, behavior, allow_secondary_conditions=True)
        self.assertEqual(selected[8], [condition])
        self.assertEqual(selected[3]['effects'], dependencies['abilities'][0]['effects'])
        for change in [{'operation': 'custom_callback'}, {'condition': {'type': 'unknown'}}]:
            invalid = copy.deepcopy(dependencies)
            invalid['effects'][1].update(change)
            self.assertIsNone(select_melee(monster, invalid, behavior, allow_secondary_conditions=True))


if __name__ == '__main__':
    unittest.main()
