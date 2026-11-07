"""Dependency failures must never be hidden by structural admission or primitive support."""
import copy
import unittest

import verify_runtime_readiness as runtime


def ref(family, key, revision='definition-r1'):
    return {'family': family, 'key': f'oteryn:{key}', 'revision': revision}


def fixture():
    creature, behavior, ability, effect, formula = (
        ref('Creature', 'creature.test'), ref('Behavior', 'behavior.test'),
        ref('Ability', 'ability.test'), ref('Effect', 'effect.test'), ref('Formula', 'formula.test'))
    records = [
        {'kind': 'Creature', 'identity': creature, 'behavior': behavior},
        {'kind': 'Behavior', 'identity': behavior},
        {'kind': 'Ability', 'identity': ability, 'effects': [effect]},
        {'kind': 'Effect', 'identity': effect, 'effect_family': 'Damage', 'formula': formula},
        {'kind': 'Formula', 'identity': formula}]
    profiles = [
        {'target': creature, 'data': {'kind': 'Creature', 'profile': {'abilities': [ability]}}},
        {'target': behavior, 'data': {'kind': 'Behavior', 'profile': {'attacks': [
            {'ability': ability, 'interval_ms': 1000, 'chance_ppm': 1000000}]}}},
        {'target': ability, 'data': {'kind': 'Ability', 'profile': {'details': {
            'kind': 'Melee', 'effects': [{'kind': 'Executable', 'effect': effect}]}}}},
        {'target': effect, 'data': {'kind': 'Effect', 'profile': {'damage_type': 'physical'}}},
        {'target': formula, 'data': {'kind': 'Formula', 'profile': {
            'formula': 'Range', 'minimum': 0, 'maximum': 10}}}]
    return {'records': records, 'authoring_profiles': profiles}


class RuntimeReadinessTest(unittest.TestCase):
    def test_complete_graph_and_executable_label_do_not_qualify_gameplay(self):
        report = runtime.inventory(fixture())
        self.assertEqual(report['counts']['creatures_with_dependency_problems'], 0)
        self.assertEqual(report['counts']['runtime_qualified_creatures'], 0)
        self.assertFalse(report['runtime_qualified'])
        self.assertFalse(report['creatures'][0]['runtime_qualified'])
        self.assertEqual(report['abilities'][0]['features'], ['effect.Damage', 'formula.Range'])

    def test_missing_formula_payload_is_not_implicitly_executable(self):
        stage = fixture()
        stage['authoring_profiles'].pop()
        report = runtime.inventory(stage)
        self.assertEqual(report['counts']['creatures_with_dependency_problems'], 1)
        self.assertIn('MISSING_AUTHORING_PROFILE',
                      {p['kind'] for p in report['creatures'][0]['problems']})

    def test_revision_mismatch_is_missing_even_when_key_matches(self):
        stage = fixture()
        stage['records'][3]['formula'] = ref('Formula', 'formula.test', 'definition-r2')
        report = runtime.inventory(stage)
        self.assertEqual(report['counts']['creatures_with_dependency_problems'], 1)
        self.assertTrue(any(p['kind'] == 'MISSING_REFERENCE'
                            for p in report['creatures'][0]['problems']))

    def test_schedule_omission_is_reported_and_missing_ability_traced(self):
        stage = fixture()
        stage['authoring_profiles'][1]['data']['profile']['attacks'][0]['ability'] = ref('Ability', 'lost')
        report = runtime.inventory(stage)
        kinds = {p['kind'] for p in report['creatures'][0]['problems']}
        self.assertIn('SCHEDULE_NOT_IN_CREATURE_ABILITIES', kinds)
        self.assertIn('MISSING_DEFINITION', kinds)

    def test_inline_parameters_and_transitive_formula_are_inventoried(self):
        stage = fixture()
        inline = {'key': 'slow', 'operation': {'operation': 'Condition', 'condition': {
            'condition_type': 'paralyze', 'buff_spell': False,
            'speed_formula': ref('Formula', 'missing_speed')}}}
        stage['authoring_profiles'][2]['data']['profile']['details']['effects'].append(
            {'kind': 'Inline', 'effect': inline})
        report = runtime.inventory(stage)
        self.assertIn('condition.buff_spell', report['abilities'][0]['features'])
        self.assertIn('condition.speed_formula', report['abilities'][0]['features'])
        self.assertEqual(report['counts']['creatures_with_dependency_problems'], 1)
        self.assertFalse(report['runtime_qualified'])

    def test_variant_cycle_is_finite_and_fails(self):
        stage = fixture()
        stage['authoring_profiles'][2]['data']['profile']['details']['variants'] = [ref('Ability', 'ability.test')]
        report = runtime.inventory(stage)
        self.assertIn('ABILITY_REFERENCE_CYCLE', {p['kind'] for p in report['creatures'][0]['problems']})

    def test_variant_children_are_included_in_reachable_census(self):
        stage = fixture()
        variant = ref('Ability', 'ability.variant')
        stage['authoring_profiles'][2]['data']['profile']['details']['variants'] = [variant]
        stage['records'].append({'kind': 'Ability', 'identity': variant, 'effects': []})
        stage['authoring_profiles'].append({'target': variant, 'data': {
            'kind': 'Ability', 'profile': {'details': {'kind': 'Spell', 'effects': []}}}})
        report = runtime.inventory(stage)
        self.assertEqual(report['counts']['unique_direct_creature_abilities'], 1)
        self.assertEqual(report['counts']['unique_reachable_abilities'], 2)

    def test_duplicate_identity_is_rejected(self):
        stage = fixture()
        stage['records'].append(copy.deepcopy(stage['records'][0]))
        with self.assertRaisesRegex(ValueError, 'duplicate'):
            runtime.inventory(stage)

    def test_external_item_is_required_and_conflicts_rejected(self):
        stage = fixture()
        item = ref('Item', 'item.test')
        stage['authoring_profiles'][2]['data']['profile']['details']['effects'].append(
            {'kind': 'Inline', 'effect': {'key': 'field', 'operation': {
                'operation': 'CreateItem', 'item': item}}})
        self.assertEqual(runtime.inventory(stage)['counts']['creatures_with_dependency_problems'], 1)
        self.assertEqual(runtime.inventory(stage, [{'kind': 'Item', 'identity': item}])[
            'counts']['creatures_with_dependency_problems'], 0)
        wrong = copy.deepcopy(stage['records'][0])
        wrong['behavior'] = ref('Behavior', 'other')
        with self.assertRaisesRegex(ValueError, 'conflicting'):
            runtime.inventory(stage, [wrong])

    def test_empty_or_non_creature_input_cannot_qualify(self):
        for stage in ({'records': [], 'authoring_profiles': []},
                      {'records': fixture()['records'][1:],
                       'authoring_profiles': fixture()['authoring_profiles'][1:]}):
            with self.assertRaises(ValueError):
                runtime.inventory(stage)


if __name__ == '__main__':
    unittest.main()
