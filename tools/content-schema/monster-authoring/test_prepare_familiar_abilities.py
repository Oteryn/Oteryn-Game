import copy
import json
import sys
import unittest
from pathlib import Path

import prepare_familiar_abilities as f

ROOT = Path(__file__).resolve().parent
sys.path.insert(0, str(ROOT.parents[1] / 'content-migration'))
import creature_admission_stage as admission


class FamiliarCompletionTests(unittest.TestCase):
    def setUp(self):
        directory = ROOT / 'samples/canary-47dfd51f-batch-2/knight_familiar'
        self.bundle = [json.loads((directory / name).read_text()) for name in
                       ('monster.json', 'dependencies.json', 'catalog.json', 'manifest.json')]
        self.facts = {'mana_cost': 1000, 'duration_ms': 900000}

    def complete(self, bundle=None, facts=None):
        return f.supplement(*(bundle or self.bundle), 'knight', facts or self.facts)

    def test_preserves_entire_monster_and_original_dependencies(self):
        before = copy.deepcopy(self.bundle)
        monster, deps, _, _ = self.complete()
        self.assertEqual(monster, before[0])
        self.assertEqual(self.bundle, before)
        for section, values in before[1].items():
            self.assertEqual(deps[section][:len(values)], values)

    def test_real_owned_summon_effect_and_exact_local_closure(self):
        monster, deps, catalog, _ = self.complete()
        self.assertEqual(catalog['definitions'], [])
        effect = deps['effects'][-1]
        self.assertEqual(effect['operation'], 'summon_creature')
        self.assertEqual(effect['summon']['creatures'], [f.cb.ref('Creature', monster['creature']['identity']['key'])])
        self.assertEqual(effect['summon']['only_below_summons'], 1)
        self.assertTrue(effect['summon']['owned'])
        self.assertEqual(effect['summon']['max_offset_tiles'], 0)
        self.assertEqual(deps['formulas'], self.bundle[1]['formulas'])

    def test_native_conversion_uses_summon_payload(self):
        monster, deps, _, _ = self.complete()
        stage = admission.Stage(admission.Mapper({}))
        stage.stage_dependencies(deps, 'knight_familiar')
        stage.stage_monster(monster, 'knight_familiar')
        key = stage.mapper.key('Ability', monster['creature']['summoning']['familiar']['summon_ability']['key'])
        profile = stage.profiles[('Ability', key)]
        operation = profile['data']['profile']['details']['effects'][0]['effect']['operation']
        self.assertEqual(operation['operation'], 'SummonCreature')
        self.assertEqual(operation['creatures'][0]['key'], 'oteryn:creature.knight_familiar')
        self.assertTrue(operation['owned'])
        identities = {(family, native_key, 'definition-r1') for family, native_key in stage.records}
        for value in list(stage.profiles.values()) + list(stage.records.values()):
            for reference in admission.exact_definition_refs(value):
                self.assertIn(reference, identities)

    def test_wrong_cost_or_duration_refused(self):
        for field in ('mana_cost', 'duration_ms'):
            facts = dict(self.facts, **{field: 1})
            with self.assertRaisesRegex(ValueError, 'cost/duration'):
                self.complete(facts=facts)

    def test_wrong_identity_refused(self):
        bundle = copy.deepcopy(self.bundle)
        bundle[0]['creature']['summoning']['familiar']['summon_ability']['revision'] = 'wrong'
        with self.assertRaisesRegex(ValueError, 'contract'):
            self.complete(bundle)

    def test_duplicate_or_missing_catalog_definition_refused(self):
        for duplicate in (False, True):
            bundle = copy.deepcopy(self.bundle)
            ref = bundle[2]['definitions'][0]
            bundle[2]['definitions'] = [ref, ref] if duplicate else []
            with self.assertRaisesRegex(ValueError, 'exactly one'):
                self.complete(bundle)

    def test_existing_local_ability_refused(self):
        bundle = copy.deepcopy(self.bundle)
        bundle[1]['abilities'].append({'identity': bundle[0]['creature']['summoning']['familiar']['summon_ability']})
        with self.assertRaisesRegex(ValueError, 'already locally'):
            self.complete(bundle)

    def test_runtime_gaps_explicit(self):
        self.assertTrue(any('timed familiar removal' in gap for gap in f.UNREPRESENTED))
        self.assertTrue(any('mana payment' in gap for gap in f.UNREPRESENTED))
        self.assertTrue(any('premium' in gap for gap in f.UNREPRESENTED))


if __name__ == '__main__':
    unittest.main()
