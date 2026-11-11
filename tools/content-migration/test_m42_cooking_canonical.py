"""Unrelated shard edits never authorize a change to a consumed cooking row."""
import copy
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


class CanonicalCookingRefresh(unittest.TestCase):
    def fixture(self, root):
        self.root = root
        self.history = {}
        rows = {'records': [
            {'definition': {'identity': {'key': f'oteryn:item.tibia.i{i}'}, 'weight': i}}
            for i in (1, 2, 3)
        ]}
        historical = self.encode(rows)
        self.history[f'{tool.BASE}:items.json'] = historical
        self.history[f'{tool.PACKET_BASE}:items.json'] = historical
        self.write('items.json', historical)
        recipe = self.encode({'recipes': [{
            'ingredients': [{'item': 'oteryn:item.tibia.i1'}],
            'reward': {'item': 'oteryn:item.tibia.i1'},
        }]})
        self.write(tool.RECIPE, recipe)
        self.write(tool.AUTHORED, recipe)
        self.write('bindings.json', b'accepted bindings')
        admission_inputs = []
        for path in tool.REFINER_INPUTS:
            raw = self.encode({'accepted': path})
            self.write(path, raw)
            admission_inputs.append({'path': path, 'sha256': tool.digest(raw)})
        packet = {
            'classification': [{'key': f'oteryn:item.tibia.i{i}', 'holds': []} for i in (1, 2)],
            'records': [{'source': 'immutable accepted source'}],
            'canonical_inputs': [{'path': 'items.json', 'sha256': tool.digest(historical)}],
            'authoring_inputs': [{'path': tool.AUTHORED, 'sha256': tool.digest(recipe)}],
            'binding_input': {'path': 'bindings.json', 'sha256': tool.digest(b'accepted bindings')},
            'accepted_inputs': admission_inputs,
        }
        accepted = self.encode(packet)
        self.history[f'{tool.PACKET_BASE}:{tool.PACKET}'] = accepted
        self.write(tool.PACKET, accepted)
        self.write(tool.WITNESS, b'old witness')
        # A different owner changes only the row outside the accepted packet.
        rows['records'][2]['definition']['weight'] = 30
        self.write('items.json', self.encode(rows))
        return packet

    @staticmethod
    def encode(value):
        return (json.dumps(value, indent=2, ensure_ascii=False) + '\n').encode()

    def write(self, path, raw):
        target = self.root / path
        target.parent.mkdir(parents=True, exist_ok=True)
        target.write_bytes(raw)

    def git(self, args, **kwargs):
        return self.history[args[-1]]

    def snapshot(self):
        return {str(path.relative_to(self.root)): path.read_bytes()
                for path in self.root.rglob('*') if path.is_file()}

    def refused_without_writes(self, expected):
        before = self.snapshot()
        with patch.object(tool.subprocess, 'check_output', side_effect=self.git):
            with self.assertRaisesRegex(ValueError, expected):
                tool.refresh(self.root)
        self.assertEqual(self.snapshot(), before)

    def test_refresh_qualifies_unrelated_shard_changes_and_is_idempotent(self):
        with tempfile.TemporaryDirectory() as tmp:
            self.fixture(Path(tmp))
            with patch.object(tool.subprocess, 'check_output', side_effect=self.git):
                tool.refresh(self.root)
                first = self.snapshot()
                tool.refresh(self.root)
            self.assertEqual(self.snapshot(), first)
            packet = json.loads((self.root / tool.PACKET).read_bytes())
            self.assertEqual(packet['canonical_inputs'][0]['sha256'],
                             tool.digest((self.root / 'items.json').read_bytes()))
            tool.verify(self.root, packet['canonical_inputs'][0], {'oteryn:item.tibia.i1'})
            self.assertEqual(json.loads((self.root / tool.WITNESS).read_bytes())['baseline'], tool.BASE)

    def test_packet_seal_rejects_changed_classification_holds_and_source_before_writes(self):
        for mutation in ('classification', 'holds', 'source', 'descriptor_membership', 'raw_format'):
            with self.subTest(mutation=mutation), tempfile.TemporaryDirectory() as tmp:
                packet = self.fixture(Path(tmp))
                if mutation == 'classification':
                    packet['classification'].append({'key': 'oteryn:item.tibia.i3', 'holds': []})
                elif mutation == 'holds':
                    packet['classification'][0]['holds'].append('CHANGED_HOLD')
                elif mutation == 'source':
                    packet['records'][0]['source'] = 'different source'
                elif mutation == 'descriptor_membership':
                    packet['canonical_inputs'].append(copy.deepcopy(packet['canonical_inputs'][0]))
                raw = self.encode(packet)
                self.write(tool.PACKET, raw + b' ' if mutation == 'raw_format' else raw)
                self.refused_without_writes('M42_CAPTURE_PACKET_SEAL')

    def test_current_matching_pin_cannot_hide_a_non_cookbook_selected_row_change(self):
        with tempfile.TemporaryDirectory() as tmp:
            packet = self.fixture(Path(tmp))
            rows = json.loads((self.root / 'items.json').read_bytes())
            rows['records'][1]['definition']['weight'] = 200
            raw = self.encode(rows)
            self.write('items.json', raw)
            packet['canonical_inputs'][0]['sha256'] = tool.digest(raw)
            self.write(tool.PACKET, self.encode(packet))
            self.refused_without_writes('GRIZZLY_SHOP_REPIN_SELECTED_DEFINITION_CHANGED')

    def test_historical_cookbook_row_is_checked_before_any_refresh_write(self):
        with tempfile.TemporaryDirectory() as tmp:
            self.fixture(Path(tmp))
            previous = json.loads(self.history[f'{tool.BASE}:items.json'])
            previous['records'][0]['definition']['weight'] = 100
            self.history[f'{tool.BASE}:items.json'] = self.encode(previous)
            self.refused_without_writes('M42_CAPTURE_SELECTED_CANONICAL_DRIFT')

    def test_recipe_binding_and_refiner_inputs_are_pinned_before_writes(self):
        for changed in (tool.RECIPE, tool.AUTHORED, 'bindings.json', *tool.REFINER_INPUTS):
            with self.subTest(path=changed), tempfile.TemporaryDirectory() as tmp:
                self.fixture(Path(tmp))
                self.write(changed, (self.root / changed).read_bytes() + b' ')
                self.refused_without_writes('M42_CAPTURE_(RECIPE|BINDING|REFINER_INPUT)')

    def test_no_write_pin_plan_and_existing_refresh_have_identical_guarded_output(self):
        from grizzly_shop_profiles import plan_canonical_input_pins, refresh_canonical_input_pins
        with tempfile.TemporaryDirectory() as tmp:
            self.fixture(Path(tmp))
            before = self.snapshot()
            with patch.object(tool.subprocess, 'check_output', side_effect=self.git):
                planned = plan_canonical_input_pins(
                    self.root, tool.PACKET_BASE, packet_path=tool.PACKET,
                    keys={'oteryn:item.tibia.i1', 'oteryn:item.tibia.i2'})
                self.assertEqual(self.snapshot(), before)
                self.assertIsNone(refresh_canonical_input_pins(
                    self.root, tool.PACKET_BASE, packet_path=tool.PACKET,
                    keys={'oteryn:item.tibia.i1', 'oteryn:item.tibia.i2'}))
            self.assertEqual((self.root / tool.PACKET).read_bytes(), planned)


if __name__ == '__main__':
    unittest.main()
