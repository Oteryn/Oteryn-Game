"""Exact candidate-graph closure regressions; no source identities are inferred from names."""
import unittest

import creature_admission_stage as stage


def identity(name, revision=stage.REVISION, family='Creature'):
    return family, 'oteryn:' + family.lower() + '.' + name, revision


def candidate(name, needed=(), produced=()):
    monster = {'creature': {'identity': {'key': 'canary:creature/' + name}}}
    return {}, monster, {}, {identity(name), *produced}, set(needed)


def close(candidates, encounters=None, waiting=None, covered_by=None, spells_need=None):
    return stage.resolve_reference_closure(candidates, encounters or {}, waiting or {}, covered_by or {},
                                           spells_need or {name: set() for name in candidates}, stage.Mapper({}))


class ExactClosureTests(unittest.TestCase):
    def test_nested_references_keep_revision_and_ignore_items(self):
        refs = [{'family': family, 'key': 'oteryn:' + family.lower(), 'revision': revision}
                for family, revision in [('Creature', 'r1'), ('Ability', 'r2'), ('Item', 'r3')]]
        self.assertEqual(list(stage.exact_definition_refs({'nested': refs})),
                         [('Creature', 'oteryn:creature', 'r1'), ('Ability', 'oteryn:ability', 'r2')])
        self.assertEqual(list(stage.definition_refs(refs)), [('Creature', 'oteryn:creature'), ('Ability', 'oteryn:ability')])

    def test_closed_chain_is_admitted_independent_of_order(self):
        candidates = {'a': candidate('a', [identity('b')]), 'b': candidate('b', [identity('c')]), 'c': candidate('c')}
        self.assertEqual(close(candidates), ({'a', 'b', 'c'}, set(), []))
        self.assertEqual(close(dict(reversed(list(candidates.items())))), close(candidates))

    def test_closed_cycle_is_not_rejected_by_topological_order(self):
        self.assertEqual(close({'a': candidate('a', [identity('b')]), 'b': candidate('b', [identity('a')])}),
                         ({'a', 'b'}, set(), []))

    def test_missing_revision_is_not_satisfied_by_same_key(self):
        admitted, _, missing = close({'a': candidate('a', [identity('b', 'r2')]), 'b': candidate('b')})
        self.assertEqual(admitted, {'b'})
        self.assertEqual(missing, [{'monster': 'a', 'references': ['oteryn:creature.b']}])

    def test_missing_reference_removes_all_dependents_but_preserves_other_group(self):
        admitted, _, missing = close({'a': candidate('a', [identity('b')]),
                                     'b': candidate('b', [identity('missing')]), 'c': candidate('c')})
        self.assertEqual(admitted, {'c'})
        self.assertEqual([row['monster'] for row in missing], ['a', 'b'])

    def test_external_records_are_not_invented(self):
        self.assertEqual(close({'a': candidate('a', [identity('missing')])})[0], set())

    def test_encounter_and_participant_cycle_is_a_closed_group(self):
        encounter = {'encounter': {'identity': {'key': 'canary:encounter/fight'}},
                     'creatures': ['canary:creature/a'], 'abilities': ['canary:ability/spell/hit']}
        values = {'a': candidate('a', produced=[identity('spell.hit', family='Ability')])}
        result = close(values, {'fight': encounter}, covered_by={'canary:creature/a': {'fight'}},
                       spells_need={'a': {('oteryn:encounter.fight', stage.REVISION)}})
        self.assertEqual(result, ({'a'}, {'fight'}, []))
        result = close(values, {'fight': encounter}, covered_by={'canary:creature/a': {'fight'}},
                       spells_need={'a': {('oteryn:encounter.fight', 'r2')}})
        self.assertEqual(result[:2], (set(), set()))

    def test_waiting_encounter_cannot_admit_its_creature(self):
        encounter = {'encounter': {'identity': {'key': 'canary:encounter/fight'}},
                     'creatures': ['canary:creature/a'], 'abilities': []}
        result = close({'a': candidate('a')}, {'fight': encounter}, {'fight': 'unlocated'},
                       {'canary:creature/a': {'fight'}})
        self.assertEqual(result[:2], (set(), set()))

    def test_empty_candidate_population_is_safe(self):
        self.assertEqual(close({}), (set(), set(), []))


if __name__ == '__main__':
    unittest.main()
