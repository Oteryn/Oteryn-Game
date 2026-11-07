"""The source buttress cannot broaden custody or overwrite known physics."""
import copy
import json
import pathlib
import tempfile
import unittest
import hugo_route_item_profile as subject
class Admission(unittest.TestCase):
    def setUp(self):
        self.directory = tempfile.TemporaryDirectory()
        self.root = pathlib.Path(self.directory.name)
        source = self.root/subject.PROFILE
        source.parent.mkdir(parents=True)
        source.write_bytes((subject.ROOT/subject.PROFILE).read_bytes())
        self.records = json.loads(source.read_bytes())['records']
        self.record = self.records[0]
    def tearDown(self):
        self.directory.cleanup()
    def admit(self, records):
        generated = {'p':json.dumps({'records':records}).encode(), 'm':b'{"item_profiles":{}}'}
        subject.admit(self.root,generated,'p','m',lambda r:r['production_definition']['production_key'],lambda v:json.dumps(v).encode())
        return json.loads(generated['p'])['records']
    def test_exact_source_nonblocking_map_only_and_idempotent(self):
        admitted = self.admit([])
        self.assertEqual(admitted,sorted(self.records,key=lambda r:r['production_definition']['production_key']))
        self.assertEqual({int(r['production_binding']['external_id']) for r in admitted},{1125,1634,1948})
        self.assertEqual(self.admit(admitted),admitted)
        self.assertFalse(admitted[0]['admission']['materializable'])
        self.assertEqual(admitted[0]['admission']['legal_destinations'],['Ground'])
    def test_existing_inventory_authority_is_preserved_by_refusal(self):
        existing = copy.deepcopy(self.record)
        existing['admission']['materializable'] = True
        with self.assertRaisesRegex(ValueError,'EXISTING_AUTHORITY_OR_PHYSICS_CONFLICT'):
            self.admit([existing])
    def test_existing_collision_cannot_be_replaced(self):
        existing = copy.deepcopy(self.record)
        existing['attributes']['blocks_movement'] = True
        with self.assertRaisesRegex(ValueError,'EXISTING_AUTHORITY_OR_PHYSICS_CONFLICT'):
            self.admit([existing])
if __name__ == '__main__':
    unittest.main()
