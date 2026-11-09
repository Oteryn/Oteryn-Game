import unittest

import qm50_ground
import world_objects


class ElevatorGroundTests(unittest.TestCase):
    def test_qm50_source_ground_retains_item_identity_and_ordinary_step_facts(self):
        row = qm50_ground.record()
        self.assertEqual(world_objects.validate_record(row), [])
        self.assertEqual(row['identity']['key'], 'oteryn:terrain.tibia.i17970')
        self.assertEqual(row['provenance']['item_pointer']['key'], 'oteryn:item.tibia.i17970')
        self.assertEqual(row['kind'], {'state': 'KNOWN', 'value': 'ground'})
        self.assertEqual(row['walkable'], {'state': 'KNOWN', 'value': True})
        self.assertEqual(row['ground_speed'], {'state': 'KNOWN', 'value': 110})

    def test_qm50_generated_ground_is_committed_and_current(self):
        for path, raw in qm50_ground.outputs().items():
            self.assertEqual((qm50_ground.ROOT / path).read_bytes(), raw, path)

    def test_qm50_ground_collision_cannot_be_changed(self):
        row = qm50_ground.record()
        row['ground_speed']['value'] = 150
        self.assertIn('M50 ground differs from its qualified source', world_objects.validate_record(row))


if __name__ == '__main__':
    unittest.main()
