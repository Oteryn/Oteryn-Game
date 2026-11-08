"""The live USE target must survive non-production compilation as exact static geometry."""
import copy
import json
import unittest
import cook_trashcan
import world_objects

class CookTrashcanTests(unittest.TestCase):
    @classmethod
    def setUpClass(cls):
        cls.files = cook_trashcan.outputs()
        cls.record = json.loads(cls.files[cook_trashcan.SHARD])['records'][0]
    def test_exact_nonportable_source_geometry(self):
        self.assertEqual(world_objects.validate_record(self.record), [])
        self.assertEqual(self.record['identity']['key'], cook_trashcan.KEY)
        self.assertEqual(self.record['kind'], dict(state='KNOWN', value='common'))
        self.assertEqual(self.record['walkable'], dict(state='KNOWN', value=True))
        row, = [r for r in json.loads(self.files['content/world/placements/index.json'])['palette']
                if r['source_item_id'] == cook_trashcan.IDENTIFIER]
        self.assertFalse(row['provisional'])
        self.assertEqual(row['key'], cook_trashcan.KEY)
        self.assertNotIn('item_pointer', self.record['provenance'])
    def test_collision_tamper_refuses(self):
        record = copy.deepcopy(self.record)
        record['walkable']['value'] = False
        self.assertEqual(world_objects.validate_record(record),
            ['appearance-only Terrain differs from its qualified official source'])
    def test_generated_files_are_current(self):
        for name, raw in self.files.items():
            self.assertEqual((cook_trashcan.ROOT / name).read_bytes(), raw, name)

if __name__ == '__main__':
    unittest.main()
