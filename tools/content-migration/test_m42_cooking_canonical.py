"""Unrelated shard edits never authorize a change to a consumed cooking row."""
import json
from pathlib import Path
import tempfile
import unittest
import m42_cooking_canonical as tool


class CanonicalCooking(unittest.TestCase):
    def test_consumed_row_and_witness_inputs_remain_exact_without_git_history(self):
        with tempfile.TemporaryDirectory() as tmp:
            root = Path(tmp)
            for path in (tool.PACKET, tool.RECIPE, tool.WITNESS, 'items.json'):
                (root / path).parent.mkdir(parents=True, exist_ok=True)
            (root / tool.PACKET).write_bytes(b'{}')
            (root / tool.RECIPE).write_bytes(b'{"recipes":[]}')
            rows = {'records': [{'definition': {'identity': {'key': 'oteryn:item.tibia.i1'}, 'weight': 3}},
                                {'definition': {'identity': {'key': 'oteryn:item.tibia.i2'}, 'weight': 4}}]}
            raw = json.dumps(rows).encode(); (root / 'items.json').write_bytes(raw)
            keys = {'oteryn:item.tibia.i1'}
            witness = dict(schema='OTERYN_M42_COOKING_CANONICAL/v1',
                           source_packet_sha256=tool.digest((root/tool.PACKET).read_bytes()),
                           recipe_sha256=tool.digest((root/tool.RECIPE).read_bytes()),
                           shards={'items.json': tool.selected(raw, keys)})
            (root / tool.WITNESS).write_text(json.dumps(witness))
            descriptor = {'path': 'items.json'}
            tool.verify(root, descriptor, keys)
            rows['records'][1]['definition']['weight'] = 8
            (root / 'items.json').write_text(json.dumps(rows))
            tool.verify(root, descriptor, keys)
            rows['records'][0]['definition']['weight'] = 9
            (root / 'items.json').write_text(json.dumps(rows))
            with self.assertRaisesRegex(ValueError, 'CANONICAL_DRIFT'):
                tool.verify(root, descriptor, keys)
            (root / 'items.json').write_bytes(raw)
            (root / tool.PACKET).write_bytes(b'{"changed":true}')
            with self.assertRaisesRegex(ValueError, 'WITNESS_INPUT_DRIFT'):
                tool.verify(root, descriptor, keys)


if __name__ == '__main__':
    unittest.main()
