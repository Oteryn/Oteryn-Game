"""Unrelated shard edits never authorize a change to a consumed cooking row."""
import json
from pathlib import Path
import tempfile
import unittest
from unittest.mock import patch
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


class CanonicalCookingCapture(unittest.TestCase):
    def fixture(self, root, *, selected_change=False, stale_pin=False):
        for path in (tool.PACKET, tool.RECIPE):
            (root / path).parent.mkdir(parents=True, exist_ok=True)
        previous = {'records': [
            {'definition': {'identity': {'key': 'oteryn:item.tibia.i1'}, 'weight': 3}},
            {'definition': {'identity': {'key': 'oteryn:item.tibia.i2'}, 'weight': 4}},
        ]}
        historical = json.dumps(previous).encode()
        previous['records'][1]['definition']['weight'] = 8
        if selected_change:
            previous['records'][0]['definition']['weight'] = 9
        current = json.dumps(previous).encode()
        (root / 'items.json').write_bytes(current)
        packet = {'canonical_inputs': [{
            'path': 'items.json',
            'sha256': tool.digest(historical if stale_pin else current),
        }]}
        (root / tool.PACKET).write_text(json.dumps(packet))
        (root / tool.RECIPE).write_text(json.dumps({'recipes': [{
            'ingredients': [{'item': 'oteryn:item.tibia.i1'}],
            'reward': {'item': 'oteryn:item.tibia.i1'},
        }]}))
        return historical

    def test_capture_preserves_historical_rows_across_unrelated_shard_changes(self):
        with tempfile.TemporaryDirectory() as tmp:
            root = Path(tmp)
            historical = self.fixture(root)
            with patch.object(tool.subprocess, 'check_output', return_value=historical) as git:
                witness = tool.capture(root)
            git.assert_called_once_with(['git', 'show', f'{tool.BASE}:items.json'], cwd=root)
            self.assertEqual(witness['shards'], {
                'items.json': tool.selected(historical, {'oteryn:item.tibia.i1'}),
            })
            self.assertEqual(witness['baseline'], tool.BASE)

    def test_fresh_shard_pin_cannot_authorize_changed_recipe_rows(self):
        with tempfile.TemporaryDirectory() as tmp:
            root = Path(tmp)
            historical = self.fixture(root, selected_change=True)
            with patch.object(tool.subprocess, 'check_output', return_value=historical):
                with self.assertRaisesRegex(ValueError, 'M42_CAPTURE_SELECTED_CANONICAL_DRIFT'):
                    tool.capture(root)

    def test_historical_shard_hash_cannot_replace_current_packet_pin(self):
        with tempfile.TemporaryDirectory() as tmp:
            root = Path(tmp)
            historical = self.fixture(root, stale_pin=True)
            with patch.object(tool.subprocess, 'check_output', return_value=historical):
                with self.assertRaisesRegex(ValueError, 'M42_CAPTURE_CURRENT_PIN'):
                    tool.capture(root)


if __name__ == '__main__':
    unittest.main()
