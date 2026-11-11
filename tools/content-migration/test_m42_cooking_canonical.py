"""Unrelated shard edits never authorize a change to a consumed cooking row."""
import ast
import copy
import json
from pathlib import Path
import tempfile
import types
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
        keys = [f'oteryn:item.tibia.i{number}' for number in range(1, 73)]
        rows = [{'definition': {'identity': {'key': key}, 'weight': number},
                 'source': 'accepted-wrapper'} for number, key in enumerate(keys)]
        before = {
            'items-a.json': json.dumps({'records': rows[:36] + [
                {'definition': {'identity': {'key': 'other'}, 'weight': 4}},
            ]}).encode(),
            'items-b.json': json.dumps({'records': rows[36:]}).encode(),
        }
        recipe = json.dumps({'recipes': [{
            'ingredients': [{'item': key} for key in keys], 'reward': {'item': keys[0]},
        }]}).encode()
        packet = {
            'schema': 'OTERYN_NATIVE_QUEST_BATCH3_ITEM_PROFILES/v1',
            'source_revision': 'accepted-source',
            'authoring_inputs': [{'path': 'accepted-authoring', 'sha256': 'a' * 64}],
            'canonical_inputs': [{'path': path, 'sha256': tool.digest(raw)}
                                 for path, raw in before.items()],
            'records': [{'production_definition': {'production_key': key}}
                        for key in keys[:66]],
            'classification': [{'key': key, 'classification': 'HELD' if index >= 66
                                else 'EXISTING_NATIVE', 'holds': []}
                               for index, key in enumerate(keys)],
        }
        before[tool.PACKET] = json.dumps(packet).encode()
        before[tool.RECIPE] = recipe
        for path, raw in before.items():
            target = root / path
            target.parent.mkdir(parents=True, exist_ok=True)
            target.write_bytes(raw)
        source = root / 'tools/content-schema/quest-authoring/batch3_a/native/hot_cuisine.json'
        source.parent.mkdir(parents=True, exist_ok=True)
        source.write_bytes(recipe)
        (root / tool.WITNESS).write_text(json.dumps({
            'schema': 'OTERYN_M42_COOKING_CANONICAL/v1',
            'source_packet_sha256': '0' * 64, 'recipe_sha256': tool.digest(recipe),
            'shards': {},
        }))
        current = json.loads(before['items-a.json'])
        current['records'][-1]['definition']['weight'] = 8
        (root / 'items-a.json').write_text(json.dumps(current))

        def git(arguments, **kwargs):
            self.assertEqual(arguments[-2], 'show')
            prefix, path = arguments[-1].split(':', 1)
            self.assertEqual(prefix, tool.BASE)
            return before[path]
        return packet, set(keys), git

    def snapshot(self, root):
        return {path: (root / path).read_bytes() for path in (tool.PACKET, tool.WITNESS)}

    def test_converter_refreshes_before_first_prey_consumer_and_replays(self):
        with tempfile.TemporaryDirectory() as tmp:
            root = Path(tmp)
            packet, keys, git = self.fixture(root)
            converter = Path(__file__).with_name('world_project_v2_to_tree.py')
            tree = ast.parse(converter.read_text())
            main = next(node for node in tree.body
                        if isinstance(node, ast.FunctionDef) and node.name == 'main')
            def call_name(node):
                return ast.unparse(node.value.func) if isinstance(node, ast.Expr) and isinstance(node.value, ast.Call) else None
            consumer = next(index for index, node in enumerate(main.body)
                            if call_name(node) == 'prey_difficulty.regenerate')
            boundary = max(index for index, node in enumerate(main.body[:consumer])
                           if call_name(node) == 'subprocess.run')
            prefix = ast.Module(body=main.body[boundary + 1:consumer + 1], type_ignores=[])
            consumed = []
            def prey(selected_root):
                self.assertEqual(selected_root, root)
                for descriptor in json.loads((root / tool.PACKET).read_bytes())['canonical_inputs']:
                    tool.verify(root, descriptor, keys)
                consumed.append(True)
            with patch.object(tool.subprocess, 'check_output', side_effect=git), patch.dict(
                    'sys.modules', prey_difficulty=types.SimpleNamespace(regenerate=prey)):
                exec(compile(prefix, str(converter), 'exec'), {'ROOT': root})
                self.assertEqual(consumed, [True])
                first = self.snapshot(root)
                tool.refresh(root)
                self.assertEqual(self.snapshot(root), first)
            refreshed = json.loads(first[tool.PACKET])
            normalized = copy.deepcopy(refreshed)
            normalized['canonical_inputs'] = packet['canonical_inputs']
            self.assertEqual(normalized, packet)
            self.assertEqual(refreshed['canonical_inputs'][0]['sha256'],
                             tool.digest((root / 'items-a.json').read_bytes()))
            witness = json.loads(first[tool.WITNESS])
            self.assertEqual(witness['baseline'], tool.BASE)
            self.assertEqual(witness['source_packet_sha256'], tool.digest(first[tool.PACKET]))
            self.assertEqual(set().union(*(set(rows) for rows in witness['shards'].values())), keys)
            self.assertEqual(len(refreshed['records']), 66)
            self.assertEqual(sum(row['classification'] == 'HELD'
                                 for row in refreshed['classification']), 6)

    def test_selected_wrapper_changes_and_missing_or_duplicate_rows_never_write(self):
        for change in ('definition', 'wrapper', 'missing', 'duplicate', 'fresh-selected-pin'):
            with self.subTest(change=change), tempfile.TemporaryDirectory() as tmp:
                root = Path(tmp)
                _, _, git = self.fixture(root)
                path = root / 'items-b.json'
                current = json.loads(path.read_bytes())
                if change in ('definition', 'fresh-selected-pin'):
                    current['records'][0]['definition']['weight'] += 1
                elif change == 'wrapper':
                    current['records'][0]['source'] = 'unapproved'
                elif change == 'missing':
                    current['records'].pop()
                else:
                    current['records'].append(copy.deepcopy(current['records'][0]))
                path.write_text(json.dumps(current))
                if change == 'fresh-selected-pin':
                    packet = json.loads((root / tool.PACKET).read_bytes())
                    packet['canonical_inputs'][1]['sha256'] = tool.digest(path.read_bytes())
                    (root / tool.PACKET).write_text(json.dumps(packet))
                before = self.snapshot(root)
                with patch.object(tool.subprocess, 'check_output', side_effect=git):
                    with self.assertRaisesRegex(ValueError, 'SELECTED_CANONICAL_DRIFT'):
                        tool.refresh(root)
                self.assertEqual(self.snapshot(root), before)

    def test_packet_recipe_path_and_pin_provenance_fail_before_any_write(self):
        for change in ('source', 'profile', 'held', 'authoring', 'path', 'duplicate-path',
                       'missing-path', 'pin', 'recipe', 'recipe-source'):
            with self.subTest(change=change), tempfile.TemporaryDirectory() as tmp:
                root = Path(tmp)
                packet, _, git = self.fixture(root)
                if change == 'source':
                    packet['source_revision'] = 'unapproved'
                elif change == 'profile':
                    packet['records'][0]['new_authority'] = True
                elif change == 'held':
                    packet['classification'][-1]['classification'] = 'EXISTING_NATIVE'
                elif change == 'authoring':
                    packet['authoring_inputs'].append({'path': 'extra', 'sha256': 'b' * 64})
                elif change == 'path':
                    packet['canonical_inputs'][0]['path'] = '../outside.json'
                elif change == 'duplicate-path':
                    packet['canonical_inputs'].append(copy.deepcopy(packet['canonical_inputs'][0]))
                elif change == 'missing-path':
                    packet['canonical_inputs'].pop()
                elif change == 'pin':
                    packet['canonical_inputs'][0]['sha256'] = 'f' * 64
                elif change == 'recipe':
                    (root / tool.RECIPE).write_text('{"recipes":[]}')
                else:
                    (root / tool.RECIPE_SOURCE).write_text('{"recipes":[]}')
                (root / tool.PACKET).write_text(json.dumps(packet))
                before = self.snapshot(root)
                with patch.object(tool.subprocess, 'check_output', side_effect=git):
                    with self.assertRaisesRegex(ValueError, 'M42_REFRESH_'):
                        tool.refresh(root)
                self.assertEqual(self.snapshot(root), before)


    def test_incomplete_immutable_row_closure_never_writes(self):
        with tempfile.TemporaryDirectory() as tmp:
            root = Path(tmp)
            packet, _, git = self.fixture(root)
            path = 'items-b.json'
            rows = json.loads(git(['git', 'show', f'{tool.BASE}:{path}']))
            rows['records'].pop()
            raw = json.dumps(rows).encode()
            packet['canonical_inputs'][1]['sha256'] = tool.digest(raw)
            packet_raw = json.dumps(packet).encode()
            (root / tool.PACKET).write_bytes(packet_raw)
            (root / path).write_bytes(raw)
            def incomplete(arguments, **kwargs):
                selected_path = arguments[-1].split(':', 1)[1]
                return {tool.PACKET: packet_raw, path: raw}.get(selected_path) or git(arguments, **kwargs)
            before = self.snapshot(root)
            with patch.object(tool.subprocess, 'check_output', side_effect=incomplete):
                with self.assertRaisesRegex(ValueError, 'M42_CAPTURE_COOKBOOK_CLOSURE'):
                    tool.refresh(root)
            self.assertEqual(self.snapshot(root), before)


if __name__ == '__main__':
    unittest.main()
