"""Regression coverage for provisioning refusal and complete scenario mapping."""
import copy
import hashlib
import json
from pathlib import Path
import tempfile
import unittest

from build_scenarios import build, read_json
from prepare_test_manifest import prepare, REQUIRED_V5_PINS

ROOT = Path(__file__).resolve().parents[3]
ARTIFACT = ROOT / 'docs/reference/spells/r21-local-candidate/active-artifact'


class ProvisioningTests(unittest.TestCase):
    def setUp(self):
        self.temp = tempfile.TemporaryDirectory()
        self.addCleanup(self.temp.cleanup)
        self.root = Path(self.temp.name)
        data = b'{}'
        (self.root / 'input.json').write_bytes(data)
        pin = {'path': 'input.json', 'sha256': hashlib.sha256(data).hexdigest()}
        self.manifest = {'schema': 'OTERYN_NATIVE_GAMEPLAY_MANIFEST/v5'}
        self.manifest.update({key: dict(pin) for key in REQUIRED_V5_PINS})
        self.manifest['build_training']['content_revision'] = 'test-1'

    def run_prepare(self, policy='strict'):
        path = self.root / 'manifest.json'
        path.write_text(json.dumps(self.manifest))
        return prepare(path, self.root / 'output', policy, None)

    def assert_refused(self):
        with self.assertRaises(ValueError):
            self.run_prepare()
        self.assertFalse((self.root / 'output').exists())

    def test_missing_required_providers_refused_before_output(self):
        for key in REQUIRED_V5_PINS:
            with self.subTest(key=key):
                value = self.manifest.pop(key)
                self.assert_refused()
                self.manifest[key] = value

    def test_bad_path_types_empty_and_traversal(self):
        for value in (None, 7, [], '', '../input.json', '/tmp/input.json'):
            with self.subTest(path=value):
                self.manifest['catalog']['path'] = value
                self.assert_refused()

    def test_pin_symlink_escape_refused(self):
        with tempfile.TemporaryDirectory() as outside:
            target = Path(outside) / 'input.json'
            target.write_bytes(b'{}')
            (self.root / 'escape.json').symlink_to(target)
            self.manifest['catalog']['path'] = 'escape.json'
            self.assert_refused()

    def test_digest_and_training_revision_refused(self):
        self.manifest['catalog']['sha256'] = '0' * 64
        self.assert_refused()
        self.manifest['catalog']['sha256'] = hashlib.sha256(b'{}').hexdigest()
        for value in (None, '', 'é', 1, 'a' * 129):
            self.manifest['build_training']['content_revision'] = value
            self.assert_refused()

    def test_unknown_policy_refused(self):
        with self.assertRaises(ValueError):
            self.run_prepare('typo')
        self.assertFalse((self.root / 'output').exists())

    def test_unknown_manifest_and_pin_fields_refused(self):
        self.manifest['extra'] = True
        self.assert_refused()
        del self.manifest['extra']
        self.manifest['catalog']['extra'] = True
        self.assert_refused()

    def test_oversized_payload_refused_before_output(self):
        (self.root / 'input.json').write_bytes(b' ' * (8 * 1024 * 1024 + 1))
        self.assert_refused()

    def test_valid_pins_and_exclusive_output(self):
        result = self.run_prepare()
        self.assertFalse(result['runtime_activation'])
        with self.assertRaises(FileExistsError):
            self.run_prepare()


class ScenarioTests(unittest.TestCase):
    @classmethod
    def setUpClass(cls):
        cls.catalog, cls.digest = read_json(ARTIFACT / 'catalog.json')
        cls.selection, _ = read_json(ARTIFACT / 'source-selection.json')

    def test_full_catalog_aliases_and_index_order(self):
        result = build(self.catalog, self.selection, self.digest)
        self.assertEqual(result['summary']['definitions'], 246)
        self.assertEqual(result['summary']['selected'], 241)
        rows = result['scenarios']
        self.assertEqual([x['canonical_book_index'] for x in rows], list(range(1, 247)))
        self.assertEqual([x['key'] for x in rows], sorted(x['key'] for x in rows))
        self.assertTrue(all(x['execution_evidence'] is None for x in rows))
        aliases = {x['key'] for x in rows if not x['selected']}
        self.assertIn('candidate:spell/practise_magic_missile', aliases)
        self.assertTrue(all(not x['positive']['execute'] for x in rows if not x['selected']))

    def test_premium_house_target_and_cost_cases(self):
        rows = {x['key']: x for x in build(self.catalog, self.selection, self.digest)['scenarios']}
        energy = rows['candidate:spell/energy_strike']
        self.assertIn({'condition': 'mana_below_required_cost', 'expected': 'NotEnoughMana'}, energy['refusals'])
        self.assertIn('current_facing_and_occupied_effect_footprint', energy['positive']['fixtures'])
        house = next(x for x in rows.values() if x['native_behavior'] == 'house_access')
        self.assertIn('physical_world_house_interior_owner_not_composed', house['runtime_owner_requirements'])
        fiend = rows['candidate:spell/find_fiend']
        self.assertEqual(fiend['planning_status'], 'requires_runtime_owner_integration')
        self.assertIn('physical_cast_dispatcher_missing_for_nearest_fiendish', fiend['runtime_owner_requirements'])

    def test_missing_duplicate_and_unselected_alias_inputs_refused(self):
        for mutation in ('missing', 'duplicate', 'selection'):
            c, s = copy.deepcopy(self.catalog), copy.deepcopy(self.selection)
            if mutation == 'missing':
                c['bundles'].pop()
            elif mutation == 'duplicate':
                c['bundles'][1] = c['bundles'][0]
            else:
                s['selections'].pop()
            with self.subTest(mutation=mutation), self.assertRaises(ValueError):
                build(c, s, self.digest)

    def test_wrong_source_pin_refused(self):
        with self.assertRaises(ValueError):
            build(self.catalog, self.selection, '0' * 64)


if __name__ == '__main__':
    unittest.main()
