import copy
import json
import unittest

import validate_world_project_v2_to_tree as validator


class CakeCreaturePopulationTests(unittest.TestCase):
    @classmethod
    def setUpClass(cls):
        cls.reference = json.loads((validator.ROOT / 'content/world/definitions/reference.json').read_bytes())

    def test_actual_cake_source_addition_preserves_original_population(self):
        self.assertEqual(validator.creature_family_counts(self.reference)['Creature'], 1864)

    def test_same_count_cannot_substitute_an_unqualified_creature(self):
        reference = {'records': list(self.reference['records'])}
        index = next(i for i, row in enumerate(reference['records'])
                     if row['identity']['key'] == 'oteryn:creature.cake_golem')
        replacement = copy.deepcopy(reference['records'][index])
        replacement['identity']['key'] = 'oteryn:creature.unqualified_cake_substitute'
        reference['records'][index] = replacement
        with self.assertRaisesRegex(validator.ValidationError, 'CAKE_GOLEM_CREATURE_SOURCE'):
            validator.creature_family_counts(reference)

    def test_cake_addition_cannot_authorize_another_extra_creature(self):
        reference = {'records': list(self.reference['records'])}
        record = next(row for row in reference['records'] if row['identity']['key'] == 'oteryn:creature.cake_golem')
        extra = copy.deepcopy(record)
        extra['identity']['key'] = 'oteryn:creature.unqualified_extra'
        reference['records'].append(extra)
        with self.assertRaisesRegex(validator.ValidationError, 'LEGACY_CREATURE_COUNT'):
            validator.creature_family_counts(reference)


if __name__ == '__main__':
    unittest.main()
