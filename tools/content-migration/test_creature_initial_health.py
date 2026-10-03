"""Monster D4: preserve a positive spawn-health split without inventing helper HP."""
import json
import unittest

import creature_admission_stage as stage


class InitialHealthPreservation(unittest.TestCase):
    def setUp(self):
        sample = stage.ROOT / 'tools/content-schema/monster-authoring/samples/canary-47dfd51f/dragon/monster.json'
        self.monster = json.loads(sample.read_text(encoding='utf-8'))
        creature = self.monster['creature']
        item_ids = {int(creature['corpse_item']['key'].rsplit('/', 1)[1]),
                    int(creature['death_residue']['item']['key'].rsplit('/', 1)[1])}
        self.admission = stage.Stage(stage.Mapper({item: f'oteryn:item.tibia.i{item}' for item in item_ids}))

    def test_split_preserves_maximum_and_initial_independently(self):
        for initial in (1, 999):
            with self.subTest(initial=initial):
                self.monster['creature']['stats'].update(max_health=1000, initial_health=initial)
                result = self.admission.creature_profile(self.monster)
                self.assertEqual(1000, result['health'])
                self.assertEqual(initial, result['initial_health'])

    def test_equal_health_keeps_legacy_profile_shape(self):
        self.monster['creature']['stats'].update(max_health=1000, initial_health=1000)
        result = self.admission.creature_profile(self.monster)
        self.assertEqual(1000, result['health'])
        self.assertNotIn('initial_health', result)

    def test_zero_helpers_bad_bounds_and_noninteger_values_fail_closed(self):
        for maximum, initial in ((1000, 0), (0, 0), (1000, 1001), (1000, -1),
                                 (1000, True), (True, 1), (1000, 1.5),
                                 (1000, '1'), (2**64, 1)):
            with self.subTest(maximum=maximum, initial=initial):
                self.monster['creature']['stats'].update(max_health=maximum, initial_health=initial)
                with self.assertRaises(stage.StageError):
                    self.admission.creature_profile(self.monster)


if __name__ == '__main__':
    unittest.main()
