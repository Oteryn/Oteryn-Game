"""Exact pinned corpse data and atomic admission through the existing Item owner."""
import copy
import json
from pathlib import Path
import tempfile
import unittest
from unittest import mock

import encounter_corpse_items as encounter


def definitions():
    keys = {f'oteryn:item.tibia.i{i}' for i in encounter.source_policies()}
    result = []
    for path in (encounter.ROOT / 'content/items/definitions').glob('items-*.json'):
        for record in json.loads(path.read_bytes())['records']:
            definition = record['definition']
            if definition['identity']['key'] in keys | {'oteryn:item.tibia.i39530'}:
                result.append(definition)
    return result


class EncounterCorpseItemsTests(unittest.TestCase):
    def test_source_capacity_and_complete_decay_chains_are_preserved(self):
        policies = encounter.source_policies()
        self.assertEqual(set(policies), {4025, 4026, 4027, 4097, 4098, 4099, 5973, 5995, 33864, 33865, 33866, 33867, 44012, 44013, 44014})
        self.assertEqual(policies[5973], (12, 10000, 4025))
        self.assertEqual(policies[4025], (10, 300000, 4026))
        self.assertEqual(policies[4026], (10, 300000, 4027))
        self.assertEqual(policies[4027], (None, 60000, None))
        self.assertEqual(policies[5995], (15, 10000, 4097))
        self.assertEqual(policies[4097], (24, 300000, 4098))
        self.assertEqual(policies[4098], (24, 300000, 4099))
        self.assertEqual(policies[4099], (None, 60000, None))
        self.assertEqual(policies[33867], (48, 10000, 33866))
        self.assertEqual(policies[33864], (None, 600000, None))
        self.assertEqual(policies[44012], (30, 60000, 44013))
        self.assertEqual(policies[44014], (None, 60000, None))
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            path = root / encounter.XML
            path.parent.mkdir(parents=True)
            path.write_bytes(b'tampered source')
            with self.assertRaisesRegex(ValueError, 'selected pin'):
                encounter.source_policies(root)

    def test_admission_preserves_unowned_items_and_unknown_semantic_groups(self):
        items = definitions()
        primal = copy.deepcopy(next(d for d in items if d['identity']['key'].endswith('i39530')))
        encounter.apply_admissions(items)
        for item in items:
            if item['identity']['key'].endswith('i39530'):
                self.assertEqual(item, primal)
                continue
            self.assertTrue(item['materializable'])
            self.assertEqual(item['stack_class'], 'NonStackable')
            self.assertEqual(item['semantics']['physical']['value']['pickupable'],
                             {'state': 'KNOWN', 'value': False})
            for group in ('equipment', 'weapon', 'imbuement', 'use_transform', 'fluid'):
                self.assertEqual(item['semantics'][group], {'state': 'UNKNOWN'})
        first = copy.deepcopy(items)
        encounter.apply_admissions(items)
        self.assertEqual(items, first)

    def test_invalid_independent_worldobject_proof_cannot_partially_admit_items(self):
        policies = encounter.source_policies()
        items = definitions()
        before = copy.deepcopy(items)
        keys = {f'oteryn:item.tibia.i{i}' for i in policies}
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            folder = root / 'content/world/objects'
            folder.mkdir(parents=True)
            for name in encounter.OBJECT_SHARDS:
                original = json.loads((encounter.ROOT / 'content/world/objects' / name).read_bytes())
                rows = [r for r in original['records'] if
                        r.get('provenance', {}).get('item_pointer', {}).get('key') in keys]
                for row in rows:
                    if row['provenance']['item_pointer']['key'].endswith('i44014'):
                        row['movable'] = {'state': 'UNKNOWN'}
                (folder / name).write_text(json.dumps({'records': rows}))
            with mock.patch.object(encounter, 'source_policies', return_value=policies):
                with self.assertRaisesRegex(ValueError, 'WorldObject corpse binding'):
                    encounter.apply_admissions(items, root)
        self.assertEqual(items, before)


if __name__ == '__main__':
    unittest.main()
