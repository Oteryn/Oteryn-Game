"""Physical source operands required by Elemental Spheres item arrivals."""
import json
import unittest

import floor_assembly_profiles as profiles


class ElementalArrivalProfiles(unittest.TestCase):
    def test_m49_source_corpses_and_charged_world_objects_are_qualified(self):
        document = json.loads((profiles.ROOT / profiles.PATH).read_bytes())
        records = {row['production_definition']['production_key']: row
                   for row in document['records']}
        corpses = {8105, 8106, 8107, 8108, 8136, 8137, 8138, 8139, 8140, 8141}
        scenery = {7738, 7739, 7740, 7741, 7742, 7743, 7744, 7745, 7746, 7748}
        self.assertTrue((corpses | scenery).issubset(profiles.IDS))
        # Never replace the already qualified energy field or its damage owner.
        self.assertNotIn(2126, profiles.IDS)
        self.assertNotIn('oteryn:item.tibia.i2126', records)
        for item in corpses | scenery:
            row = records[f'oteryn:item.tibia.i{item}']
            self.assertEqual(row['admission']['materializable'], item in corpses)
            self.assertIn('Ground', row['admission']['legal_destinations'])
        for item, capacity in [(8139, 14), (8140, 14), (8141, 11), (8106, 15)]:
            row = records[f'oteryn:item.tibia.i{item}']
            self.assertEqual(row['semantics']['container']['value']['capacity']['value'], capacity)
            self.assertEqual(row['semantics']['temporal']['state'], 'KNOWN')


if __name__ == '__main__':
    unittest.main()
