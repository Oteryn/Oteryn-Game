#!/usr/bin/env python3
"""Reader inventory generation retains a strict canonical population pin."""
import ast
import importlib.util
import json
from pathlib import Path
import sys
import tempfile
import unittest

ROOT = Path(__file__).resolve().parents[2]
sys.path.insert(0, str(ROOT / 'tools/content-migration'))
TOOL = ROOT / 'tools/content-migration/world_project_v2_to_tree.py'
spec = importlib.util.spec_from_file_location('season_inventory', TOOL)
module = importlib.util.module_from_spec(spec)
spec.loader.exec_module(module)
TARGET = 'crates/gameplay/content/src/content/project/bestiary.rs'
VALIDATOR = 'tools/content-migration/validate_world_project_v2_to_tree.py'
GOLDEN = 'tools/content-migration/test_world_project_v2_to_tree.py'
CHARM = 'crates/gameplay/dispatch/src/gameplay_transport/charm_progression_tests.rs'
SHARD = 'content/creatures/definitions/creatures-00000-00002.json'

class InventoryTests(unittest.TestCase):
    def fixture(self, root):
        target = root / TARGET
        target.parent.mkdir(parents=True)
        target.write_text((ROOT / TARGET).read_text())
        validator = root / VALIDATOR
        validator.parent.mkdir(parents=True, exist_ok=True)
        validator.write_text((ROOT / VALIDATOR).read_text())
        (root / GOLDEN).write_text((ROOT / GOLDEN).read_text())
        charm = root / CHARM
        charm.parent.mkdir(parents=True, exist_ok=True)
        charm.write_text((ROOT / CHARM).read_text())
        records = [
            {'authoring': {'kind': 'Creature', 'profile': {'bestiary': {'kill_thresholds': [2, 3, 5]}}}},
            {'authoring': {'kind': 'Creature', 'profile': {'bestiary': None}}},
            {'authoring': {'kind': 'Creature', 'profile': {'bestiary': {'kill_thresholds': [10, 100, 250]}}}},
        ]
        shard = root / SHARD
        shard.parent.mkdir(parents=True)
        shard.write_text(json.dumps({'records': records}))
        index = root / 'content/creatures/definitions/index.json'
        index.write_text(json.dumps({'shards': [SHARD], 'record_count': 3}))
        return index, shard, target

    def test_refreshes_creature_and_bestiary_pins_and_goldens_together(self):
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            _, _, target = self.fixture(root)
            module.refresh_bestiary_inventory(root)
            self.assertIn('assert_eq!(bestiary.races.len(), 2);', (root / CHARM).read_text())
            text = target.read_text()
            self.assertIn('const CREATURE_COUNT: usize = 3;', text)
            self.assertIn('const BESTIARY_COUNT: usize = 2;', text)
            self.assertIn('assert_eq!(catalogue.bestiary.len(), 2);', text)
            self.assertIn('assert_eq!(limits().max_reference_records, 3);', text)
            self.assertIn('fn canonical_projection_retains_all_2_admitted_bestiary_blocks', text)
            validator_text = (root / VALIDATOR).read_text()
            self.assertIn('LEGACY_CREATURE_COUNT = 3', validator_text)
            self.assertIn("require(counts['Creature'] == LEGACY_CREATURE_COUNT, 'LEGACY_CREATURE_COUNT')", validator_text)
            def require(condition, code):
                if not condition:
                    raise RuntimeError(code)
            namespace = {'Any': object, 'require': require, 'CREATURE_FAMILY_NODES': {'Creature': ''}}
            nodes = ast.parse(validator_text).body
            pin = next(node for node in nodes if isinstance(node, ast.Assign)
                       and any(isinstance(target, ast.Name) and target.id == 'LEGACY_CREATURE_COUNT'
                               for target in node.targets))
            function = next(node for node in nodes
                            if isinstance(node, ast.FunctionDef) and node.name == 'creature_family_counts')
            exec(compile(ast.Module(body=[pin, function], type_ignores=[]), '<generated Creature validator>', 'exec'), namespace)
            reference = {'records': [{'identity': {'family': 'Creature'}} for _ in range(3)]}
            self.assertEqual(namespace['creature_family_counts'](reference), {'Creature': 3})
            reference['records'].pop()
            with self.assertRaisesRegex(RuntimeError, 'LEGACY_CREATURE_COUNT'):
                namespace['creature_family_counts'](reference)
            reference['records'].extend([{'identity': {'family': 'Creature'}} for _ in range(2)])
            with self.assertRaisesRegex(RuntimeError, 'LEGACY_CREATURE_COUNT'):
                namespace['creature_family_counts'](reference)
            golden_text = (root / GOLDEN).read_text()
            self.assertIn("assert family_counts['Creature'] == 3", golden_text)
            assertions = [node for node in ast.parse(golden_text).body
                          if isinstance(node, ast.Assert) and isinstance(node.test, ast.Compare)
                          and isinstance(node.test.left, ast.Subscript)
                          and isinstance(node.test.left.value, ast.Name)
                          and node.test.left.value.id in ('family_counts', 'lock')
                          and isinstance(node.test.left.slice, ast.Constant)
                          and node.test.left.slice.value in ('Creature', 'source_binding_counts')]
            self.assertEqual(len(assertions), 2)
            binding = next(node for node in assertions if node.test.left.value.id == 'lock')
            expected = ast.literal_eval(binding.test.comparators[0])
            self.assertEqual(expected['Creature'], 3)
            checks = compile(ast.Module(body=assertions, type_ignores=[]), '<generated record and binding goldens>', 'exec')
            for field in [None, 'record', 'binding']:
                values = {'family_counts': {'Creature': 3}, 'lock': {'source_binding_counts': expected.copy()}}
                if field == 'record':
                    values['family_counts']['Creature'] -= 1
                if field == 'binding':
                    values['lock']['source_binding_counts']['Creature'] -= 1
                if field is None:
                    exec(checks, values)
                else:
                    with self.assertRaises(AssertionError):
                        exec(checks, values)
            before = (target.read_bytes(), (root / VALIDATOR).read_bytes(), (root / GOLDEN).read_bytes())
            module.refresh_bestiary_inventory(root)
            self.assertEqual(before, (target.read_bytes(), (root / VALIDATOR).read_bytes(), (root / GOLDEN).read_bytes()))

    def test_malformed_authoring_or_profile_never_publishes_a_new_pin(self):
        for bad in [None, {'kind': 'Creature'}, {'kind': 'Creature', 'profile': []}, {'kind': 'Item', 'profile': {}}, {'kind': 'Creature', 'profile': {} }]:
            with tempfile.TemporaryDirectory() as directory:
                root = Path(directory)
                _, shard, target = self.fixture(root)
                document = json.loads(shard.read_text())
                if bad == {'kind': 'Creature', 'profile': {}}:
                    document['records'][0]['authoring'] = {'kind': 'Creature', 'profile': {'bestiary': 'invented'}}
                else:
                    document['records'][0]['authoring'] = bad
                shard.write_text(json.dumps(document))
                before = (target.read_bytes(), (root / VALIDATOR).read_bytes(), (root / GOLDEN).read_bytes())
                with self.assertRaisesRegex(RuntimeError, 'BESTIARY_(AUTHORING|PROFILE)_SHAPE'):
                    module.refresh_bestiary_inventory(root)
                self.assertEqual(before, (target.read_bytes(), (root / VALIDATOR).read_bytes(), (root / GOLDEN).read_bytes()))

    def test_wrong_population_or_duplicate_shard_is_refused(self):
        for mutation in ['count', 'duplicate']:
            with tempfile.TemporaryDirectory() as directory:
                root = Path(directory)
                index, _, target = self.fixture(root)
                document = json.loads(index.read_text())
                if mutation == 'count':
                    document['record_count'] += 1
                else:
                    document['shards'].append(SHARD)
                index.write_text(json.dumps(document))
                before = (target.read_bytes(), (root / VALIDATOR).read_bytes(), (root / GOLDEN).read_bytes())
                with self.assertRaisesRegex(RuntimeError, 'BESTIARY_SHARD_(COUNT|INVENTORY)'):
                    module.refresh_bestiary_inventory(root)
                self.assertEqual(before, (target.read_bytes(), (root / VALIDATOR).read_bytes(), (root / GOLDEN).read_bytes()))

if __name__ == '__main__':
    unittest.main()
