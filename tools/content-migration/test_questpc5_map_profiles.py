"""The tower barrel retains source collision facts and cannot enter inventory."""
import unittest
import questpc5_map_profiles as profiles


class Questpc5MapProfiles(unittest.TestCase):
    def test_exact_source_barrel_is_walkable_ground_only_container(self):
        row = profiles.capture()
        self.assertEqual(row['production_definition']['production_key'], 'oteryn:item.tibia.i2523')
        self.assertEqual(row['admission'], dict(materializable=True, stack_class='NonStackable', legal_destinations=['Ground']))
        self.assertFalse(row['attributes']['blocks_movement'])
        self.assertFalse(row['attributes']['blocks_projectile'])
        self.assertTrue(row['attributes']['has_height'])
        self.assertEqual(row['semantics']['physical']['value']['pickupable'], dict(state='KNOWN',value=False))
        self.assertEqual(row['semantics']['container']['value']['capacity'], dict(state='KNOWN',value=25))


if __name__ == '__main__':
    unittest.main()
