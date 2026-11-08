import unittest

import questpc3_exit_profiles as profiles


class ExitProfiles(unittest.TestCase):
    def test_exact_source_gems_are_ground_only_and_nonblocking(self):
        records = profiles.capture(profiles.owner.ROOT)
        self.assertEqual({r['production_definition']['production_key'] for r in records},
                         {f'oteryn:item.tibia.i{i}' for i in profiles.IDS})
        for row in records:
            self.assertEqual(row['admission']['legal_destinations'], ['Ground'])
            self.assertIs(row['attributes']['blocks_movement'], False)
            self.assertIs(row['attributes']['blocks_projectile'], False)
            self.assertIs(row['attributes']['immovable_block_solid'], False)
            self.assertEqual(row['attributes']['speed_bonus'], 0)


if __name__ == '__main__':
    unittest.main()
