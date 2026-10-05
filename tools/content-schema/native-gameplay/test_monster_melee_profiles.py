#!/usr/bin/env python3
"""Dependency selection tests: advanced/ambiguous donor data stays disabled."""
import copy
import unittest
from build_monster_melee_profiles import select_melee, melee_skill_range, merge_disabled_fallback


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


class DisabledFallbackMerge(unittest.TestCase):
    def inputs(self):
        refs = [{'family': 'Creature', 'key': name, 'revision': 'r1'} for name in ['rat', 'stag']]
        primary = {'records': [{'profile': {'target': ref, 'data': {'health': 20}},
                               **({'monster_melee': {'source': 'canary', 'maximum': 8}} if i == 0 else {})}
                              for i, ref in enumerate(refs)]}
        report = {'records': [{'creature': ref, 'status': 'enabled_approximate_melee' if i == 0
                               else 'disabled_no_unique_source'} for i, ref in enumerate(refs)]}
        alternate = copy.deepcopy(primary)
        for row in alternate['records']:
            row['monster_melee'] = {'source': 'crystal', 'maximum': 3}
        observations = {'pack': 'crystal/data-global/monster', 'source_index_sha256': 'frozen',
                        'records': [{'creature': ref, 'status': 'enabled_approximate_melee'} for ref in refs]}
        return primary, report, alternate, observations

    def test_disabled_only_is_filled_and_primary_provenance_preserved(self):
        args = self.inputs()
        before = copy.deepcopy(args)
        result, report = merge_disabled_fallback(*args)
        self.assertEqual(args, before)
        self.assertEqual(result['records'][0], args[0]['records'][0])
        self.assertEqual(result['records'][1]['monster_melee']['source'], 'crystal')
        self.assertEqual(report['records'][1]['primary_disabled_observation'], args[1]['records'][1])
        self.assertEqual((report['enabled'], report['disabled'], report['fallback']['recovered_count']), (2, 0, 1))

    def test_failed_fallback_stays_disabled_with_both_receipts(self):
        args = self.inputs()
        args[2]['records'][1].pop('monster_melee')
        args[3]['records'][1]['status'] = 'disabled_source_health_conflict'
        result, report = merge_disabled_fallback(*args)
        self.assertNotIn('monster_melee', result['records'][1])
        self.assertEqual(report['records'][1]['status'], 'disabled_no_unique_source')
        self.assertEqual(report['records'][1]['fallback_disabled_observation']['status'], 'disabled_source_health_conflict')

    def test_changed_identity_health_and_missing_closure_refuse(self):
        for mutation in [lambda a: a[2]['records'][1]['profile']['data'].update(health=200),
                         lambda a: a[2]['records'][1]['profile']['target'].update(revision='r2'),
                         lambda a: a[2]['records'][1].pop('monster_melee'),
                         lambda a: a[3]['records'].pop(),
                         lambda a: a[2]['records'].append(copy.deepcopy(a[2]['records'][0])),
                         lambda a: a[1]['records'].append(copy.deepcopy(a[1]['records'][0])),
                         lambda a: a[3]['records'].append(copy.deepcopy(a[3]['records'][0]))]:
            with self.subTest(mutation=mutation):
                args = self.inputs()
                mutation(args)
                with self.assertRaises(ValueError):
                    merge_disabled_fallback(*args)


if __name__ == '__main__':
    unittest.main()
