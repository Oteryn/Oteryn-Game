"""Cake admission must keep every original attack and close exact source identities."""
import unittest
from pathlib import Path
from tempfile import TemporaryDirectory

import cake_creature_admission as cake


class CakeCreatureAdmission(unittest.TestCase):
    def test_substituted_donor_does_not_admit_a_source_identity(self):
        with TemporaryDirectory() as directory:
            root = Path(directory)
            source = root / cake.SOURCE
            source.parent.mkdir(parents=True)
            source.write_text('a different source blob')
            with self.assertRaisesRegex(ValueError, 'CAKE_DONOR_SOURCE_DRIFT'):
                cake.capture(root)

    def test_complete_source_bundle_has_three_exact_attacks_and_undefined_damage(self):
        bundle = cake.complete_bundle()
        attacks = bundle['monster.json']['behavior']['attacks']
        self.assertEqual([row['ability']['key'] for row in attacks],
                         [f'canary:ability/cake_golem/attack-{n}' for n in (1, 2, 3)])
        effect, = [row for row in bundle['dependencies.json']['effects']
                   if row['identity']['key'] == 'canary:effect/cake_golem/attack-2']
        self.assertEqual(effect['damage_type'], 'undefined')
        self.assertNotIn('mitigated_by', effect)
        self.assertEqual(attacks[1]['chance_percent'], 15)
        self.assertEqual(attacks[1]['interval_ms'], 2000)

    def test_normal_stage_keeps_real_source_binding_without_bridge_dependency(self):
        bundle = cake.complete_bundle()
        staged = cake.stage_bundle(bundle, {i: f'oteryn:item.tibia.i{i}' for i in (12141, 12143, 2886)})
        binding, = staged['source_identity_bindings']
        self.assertEqual(binding['target']['key'], 'oteryn:creature.cake_golem')
        self.assertEqual(binding['external_id'], cake.SOURCE)
        self.assertEqual(binding['disposition'], 'EXACT')
        creature, = [row for row in staged['authoring_profiles'] if row['target']['family'] == 'Creature']
        self.assertEqual(creature['data']['profile']['health'], 444)
        self.assertEqual(creature['data']['profile']['experience'], 100)
        self.assertNotIn('48227', str(staged))


if __name__ == '__main__':
    unittest.main()
