"""Real pinned DAT -> complete existing native index, never a default order."""
import json
from pathlib import Path
import unittest
import build_native_item_thing_orders as producer

class NativeThingOrders(unittest.TestCase):
    @classmethod
    def setUpClass(cls):
        cache, code = Path('/tmp/srx3-donors'), Path('/tmp/srx3-crystal-code')
        if not (cache / 'appearances.dat').is_file():
            raise unittest.SkipTest('pinned donor cache absent; no source proof')
        cls.blobs = producer.inputs(cache, code)
        cls.keys_raw = producer.DEFAULT_KEYS.read_bytes()
        cls.keys = json.loads(cls.keys_raw)['records']

    def test_actual_index_has_every_source_order_and_preserves_existing_refs(self):
        value = producer.build(self.blobs, self.keys_raw)
        self.assertEqual(len(value['records']), len(self.keys))
        pairs = dict(zip((int(key.split('.i')[-1]) for key, _ in self.keys), value['records']))
        self.assertEqual(pairs[33792], [33792, 1])
        for item in (33876, 33877, 34005, 33854, 34006, 34007):
            self.assertEqual(pairs[item], [item, 0])
        self.assertEqual({row[1] for row in value['records']}, {0, 1, 2, 3})

    def test_missing_source_appearance_is_refused_without_a_default(self):
        keys = [['oteryn:item.tibia.i65535', 'definition-r1']]
        with self.assertRaisesRegex(ValueError, 'missing source top-order appearance'):
            producer.build(self.blobs, producer.encode({'records': keys}))

    def test_changed_donor_or_definition_revision_is_refused(self):
        changed = dict(self.blobs)
        changed['src/items/items.cpp'] += b'\n'
        with self.assertRaisesRegex(ValueError, 'pinned Thing-order donor differs'):
            producer.build(changed, self.keys_raw)
        with self.assertRaisesRegex(ValueError, 'exact pinned source identity'):
            producer.build(self.blobs, producer.encode({'records': [[self.keys[0][0], 'other-r2']]}))
