import unittest

from build_creature_races import CAKE_PATH, CAKE_REVISION, binding_revision, source_race


class CakeRaceTests(unittest.TestCase):
    def test_exact_successor_binding_retains_its_source_revision(self):
        identity = {'family': 'Creature', 'key': 'oteryn:creature.cake_golem',
                    'revision': 'definition-r1'}
        binding = {'identity_namespace': 'crystalserver/monster-file', 'target': identity,
                   'external_id': CAKE_PATH,
                   'source_revision': 'crystalserver-creature-1530:' + CAKE_REVISION}
        self.assertEqual(binding_revision(binding, identity, 'old-pin'), CAKE_REVISION)
        for forged in ({**binding, 'external_id': 'other.lua'},
                       {**binding, 'target': {**identity, 'revision': 'definition-r2'}},
                       {**binding, 'source_revision': 'unqualified:' + CAKE_REVISION}):
            with self.assertRaisesRegex(ValueError, 'unpinned source revision'):
                binding_revision(forged, identity, 'old-pin')
        with self.assertRaisesRegex(ValueError, 'unpinned source revision'):
            binding_revision(binding, {'key': 'oteryn:creature.other'}, 'old-pin')
        with self.assertRaisesRegex(ValueError, 'unpinned source revision'):
            binding_revision(binding, {**identity, 'revision': 'definition-r2'}, 'old-pin')

    def test_cake_source_venom_is_not_the_default_blood_race(self):
        self.assertEqual(source_race('monster.race = "venom"\n'), ('venom', 'explicit'))
        self.assertEqual(source_race('-- monster.race = "venom"\n'), ('blood', 'source_default'))


if __name__ == '__main__':
    unittest.main()
