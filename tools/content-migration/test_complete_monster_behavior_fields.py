import copy
import json
import tempfile
import unittest
from pathlib import Path
from jsonschema import Draft202012Validator

import complete_monster_behavior_fields as completion

POP = Path('/workspace/monster-final-output/population')
AUDIT = Path('/workspace/monster-field-audit-20261002/mechanics')
REPO = Path(__file__).resolve().parents[2]


class BehaviorFieldCompletionTests(unittest.TestCase):
    @classmethod
    def setUpClass(cls):
        cls.temp = tempfile.TemporaryDirectory()
        cls.before = completion.sha(POP / 'population-index.json')
        cls.packet = completion.prepare(POP, AUDIT, Path(cls.temp.name))

    @classmethod
    def tearDownClass(cls):
        cls.temp.cleanup()

    def test_boolean_patch_guards_and_exact_wiki_values(self):
        cells = [p for p in self.packet['patches'] if p['source']['kind'] == 'mediawiki']
        self.assertEqual(len(cells), 202)
        self.assertEqual(len(self.packet['excluded_cells']), 13)
        self.assertFalse({p['monster'] for p in cells} & completion.SHARED_PAGE_VARIANTS)
        self.assertEqual(len({(p['monster'], p['file'], p['pointer']) for p in cells}), len(cells))
        for p in cells:
            self.assertIs(p['expected_present'], True)
            self.assertIsInstance(p['expected_value'], bool)
            self.assertIsInstance(p['value'], bool)
            self.assertIsNot(p['value'], p['expected_value'])
            self.assertGreater(p['source']['revision_id'], 0)
            self.assertEqual(len(p['source']['content_sha256']), 64)
        acid = next(p for p in cells if p['monster'] == 'acid_blob')
        self.assertIs(acid['value'], False)
        self.assertEqual(self.before, completion.sha(POP / 'population-index.json'))

    def test_proxy_is_finite_typed_and_not_labeled_global(self):
        patches = [p for p in self.packet['patches'] if p['source']['kind'] == 'owner_accepted_non_global_proxy']
        schema = completion.read(REPO / 'tools/content-schema/monster-authoring/monster.schema.json')
        for field, definition in [('abilities', 'ability'), ('effects', 'effect'), ('formulas', 'formula')]:
            p = next(p for p in patches if p['pointer'] == '/' + field + '/-')
            Draft202012Validator({'$ref': '#/$defs/' + definition, '$defs': schema['$defs']}).validate(p['value'])
            self.assertIs(p['source']['global_parity'], False)
            self.assertFalse(p['expected_present'])
        formula = next(p['value'] for p in patches if p['pointer'] == '/formulas/-')
        self.assertEqual(formula['magnitude'], {'minimum': 430, 'maximum': 550})
        schedule = next(p['value'] for p in patches if p['pointer'] == '/behavior/attacks/-')
        self.assertEqual((schedule['interval_ms'], schedule['chance_percent']), (2000, 20))

    def test_new_proxy_reference_closure_and_unique_catalog_declarations(self):
        patches = [p for p in self.packet['patches'] if p['source']['kind'] == 'owner_accepted_non_global_proxy']
        defs = {p['value']['identity']['key']: p['value'] for p in patches if p['file'] == 'dependencies.json'}
        self.assertEqual(len(defs), 3)
        catalog = [p['value'] for p in patches if p['file'] == 'catalog.json']
        self.assertFalse(catalog)  # Local definitions must not be duplicated in the external catalog.
        for p in patches:
            def refs(x):
                if isinstance(x, dict):
                    if x.get('family') in {'Ability', 'Effect', 'Formula'}:
                        self.assertIn(x['key'], defs)
                        self.assertEqual(x['revision'], defs[x['key']]['identity']['revision'])
                    for v in x.values():
                        refs(v)
                elif isinstance(x, list):
                    for v in x:
                        refs(v)
            refs(p['value'])
        self.assertIn('WIKI_EARTH_WAVE_DAMAGE_ESTIMATED', self.packet['actor_flags']['dark_merudri'])

    def test_every_candidate_actor_and_scene_is_strictly_valid(self):
        import sys
        sys.path[:0] = [str(REPO / 'tools/content-schema/monster-authoring'), str(REPO / 'tools/content-schema/encounter-authoring')]
        import validate_monster as vm
        import validate_encounter as ve
        import creature_admission_stage as stage
        changed = {}
        for p in self.packet['patches']:
            files = changed.setdefault(p['monster'], {n: completion.read(POP / 'bundles' / p['monster'] / n) for n in ['monster.json', 'dependencies.json', 'catalog.json']})
            target = files[p['file']]; parts = p['pointer'].strip('/').split('/')
            for part in parts[:-1]:
                target = target[int(part)] if isinstance(target, list) else target[part]
            if parts[-1] == '-':
                if p['value'] not in target:target.append(copy.deepcopy(p['value']))
            else:target[parts[-1]] = copy.deepcopy(p['value'])
        for monster, files in changed.items():
            self.assertEqual(vm.validate(files['monster.json'], files['dependencies.json'], files['catalog.json']), [], monster)
            for collection, family in [('abilities','Ability'),('effects','Effect'),('formulas','Formula')]:
                for value in files['dependencies.json'][collection]:stage.Mapper({}).identity(family, value['identity'])
        mapper = stage.Mapper({})  # New scenes do not refer to Items.
        for row in self.packet['encounters']:
            directory = Path(self.temp.name) / row['relative_dir']
            e, catalog, manifest = (completion.read(directory / f) for f in ['encounter.json', 'catalog.json', 'manifest.json'])
            self.assertEqual(ve.validate(e, catalog, manifest), [], row['slug'])
            self.assertTrue(stage.encounter_details(e, mapper)['rules'])
        self.assertEqual(len(self.packet['encounters']), 9)

    def test_heal_gate_and_delayed_heal_are_not_blocked_by_their_own_lock(self):
        for name, delay, cooldown in [('lisa', 6000, 6000), ('tyrn', 0, 900000)]:
            e = completion.read(Path(self.temp.name) / 'encounters' / ('field_fill_' + name) / 'encounter.json')
            start, finish = e['rules']
            self.assertIn({'kind': 'flag', 'flag': 'healing_locked', 'value': False}, start['conditions'])
            self.assertEqual(e['state']['timers'][0]['duration_ms'], cooldown)
            self.assertEqual(sum(a['kind'] == 'heal' for a in start['actions']), int(not delay))
            self.assertEqual(sum(a['kind'] == 'heal' for a in finish['actions']), int(bool(delay)))
            self.assertEqual(finish['conditions'], [])

    def test_foam_matrix_phases_keep_source_order_and_delays(self):
        e = completion.read(Path(self.temp.name) / 'encounters/field_fill_foam_stalker/encounter.json')
        self.assertEqual([r['delay_ms'] for r in e['rules']], [1000, 2000, 3000])
        refs = [r['actions'][0]['ability']['key'] for r in e['rules']]
        self.assertEqual(len(set(refs)), 3)
        self.assertEqual(self.packet['counts']['source_core_rows'], 14)

    def test_baseline_drift_is_rejected_before_output(self):
        with tempfile.TemporaryDirectory() as folder:
            path = Path(folder)
            (path / 'population-index.json').write_text('{}')
            with self.assertRaisesRegex(ValueError, 'Unexpected population-index SHA'):
                completion.prepare(path, AUDIT, path / 'out')
            self.assertFalse((path / 'out').exists())


if __name__ == '__main__':
    unittest.main()
