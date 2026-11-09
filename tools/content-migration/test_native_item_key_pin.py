"""The registered native provider must not pin a stale canonical Item key slice."""
import json
import tempfile
import unittest
from pathlib import Path
import register_spell_families as registered

class NativeItemKeyPinTests(unittest.TestCase):
    def test_registered_provider_refuses_missing_real_reference_keys(self):
        with tempfile.TemporaryDirectory() as temp:
            root=Path(temp)
            source=Path(__file__).resolve().parents[2]
            rows=[]
            for name in json.loads((source/'content/items/index.json').read_bytes())['shards']:
                for row in json.loads((source/name).read_bytes())['records']:
                    if row['definition']['identity']['key'] in ('oteryn:item.tibia.i43666','oteryn:item.tibia.i43860'):rows.append(row)
            self.assertEqual(len(rows),2)
            shard='content/items/definitions/items-00000-00001.json'
            (root/shard).parent.mkdir(parents=True)
            (root/shard).write_text(json.dumps({'family':'Item','records':rows}))
            (root/'content/items/index.json').write_text(json.dumps({'record_count':2,'shards':[shard]}))
            for name in registered.REPOSITORY_PINS.values():
                (root/name).parent.mkdir(parents=True,exist_ok=True)
                (root/name).write_text('{}')
            stale={'schema':'OTERYN_NATIVE_ITEM_KEYS/v1','records':[['oteryn:item.tibia.i43666','definition-r1']]}
            (root/registered.REPOSITORY_PINS['item_keys']).write_text(json.dumps(stale))
            generated={registered.NATIVE_MANIFEST:b'{}'}
            with self.assertRaisesRegex(ValueError,'NATIVE_ITEM_KEYS_STALE'):
                registered.pin_repository_inputs(root,generated)
