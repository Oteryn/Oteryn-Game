"""Small synthetic inputs exercise lab failures and actor accounting."""
import json
from pathlib import Path
import tempfile
import unittest
import lab


class InventoryTests(unittest.TestCase):
    def setUp(self):
        self.temp = tempfile.TemporaryDirectory()
        self.addCleanup(self.temp.cleanup)
        self.root = Path(self.temp.name)
        bundles = self.root / 'bundles'
        rows = []
        for name in ('rat', 'boss', 'helper'):
            folder = bundles / name
            folder.mkdir(parents=True)
            payload = {'creature': {'stats': {'max_health': 20}}}
            if name == 'rat':
                payload['creature']['stats']['mitigation_percent'] = 0.12
                payload['creature']['bestiary'] = {}
            for filename in lab.BUNDLE_FILES:
                value = payload if filename == 'monster.json' else {}
                (folder / filename).write_text(json.dumps(value))
            rows.append({'monster': name, 'sha256': lab.bundle_digest(folder)})
        self.index = {'monsters': rows, 'bundles': 3, 'bundle_files': list(lab.BUNDLE_FILES)}
        self.stage = {'authoring_profiles': [{'target': {'key': 'oteryn:creature.rat'}, 'data': {'kind': 'Creature'}}],
                      'counts': {'creatures': 1}, 'deferred': {'encounter': ['boss'],
                      'unresolved_reference': [{'monster': 'helper', 'references': ['missing']}],
                      'encounters': [{'encounter': 'boss_fight'}]}}
        self.config = {'read_only_inputs': {'index': str(self.root / 'index.json'),
                       'stage': str(self.root / 'stage.json'), 'bundles': str(bundles)}}
        self.write_inputs()
        (self.root / 'config.json').write_text(json.dumps(self.config))

    def write_inputs(self):
        (self.root / 'index.json').write_text(json.dumps(self.index))
        (self.root / 'stage.json').write_text(json.dumps(self.stage))

    def test_native_hold_accounting_and_determinism(self):
        report = lab.inventory(self.config)
        self.assertEqual(report, lab.inventory(self.config))
        self.assertEqual((report['counts']['prepared'], report['counts']['native_admitted'], report['counts']['native_held']), (3, 1, 2))
        self.assertEqual(report['counts']['mitigation_unknown'], 2)
        self.assertEqual(report['counts']['encounter_definitions_held'], 1)
        self.assertEqual(len(lab.preparation_plan(report)['actors']), 2)
        self.assertTrue(all(r['runtime_status'] == 'GAMEPLAY_UNVERIFIED' for r in report['monsters']))

    def test_malformed_index_count(self):
        self.index['bundles'] = 4
        self.write_inputs()
        with self.assertRaisesRegex(lab.LabError, 'Malformed index'):
            lab.inventory(self.config)

    def test_source_completion_flags_survive_inventory(self):
        self.index['monsters'][0]['completion_flags'] = ['SOURCE_BEHAVIOR_PARTIAL']
        self.write_inputs()
        row = next(r for r in lab.inventory(self.config)['monsters'] if r['monster'] == 'rat')
        self.assertIn('SOURCE_BEHAVIOR_PARTIAL', row['quality_flags'])
        self.assertEqual(row['runtime_status'], 'GAMEPLAY_UNVERIFIED')

    def test_malformed_completion_flags_cannot_be_hidden(self):
        for flags in ('SOURCE_BEHAVIOR_PARTIAL', [None], ['']):
            with self.subTest(flags=flags):
                self.index['monsters'][0]['completion_flags'] = flags
                self.write_inputs()
                with self.assertRaisesRegex(lab.LabError, 'Malformed completion flags'):
                    lab.inventory(self.config)

    def test_digest_mismatch(self):
        (self.root / 'bundles/rat/monster.json').write_text('{}')
        with self.assertRaisesRegex(lab.LabError, 'digest mismatch'):
            lab.inventory(self.config)

    def test_nonexistent_input(self):
        self.config['read_only_inputs']['stage'] = str(self.root / 'absent.json')
        with self.assertRaisesRegex(lab.LabError, 'Cannot read JSON'):
            lab.inventory(self.config)

    def test_missing_bundle_file(self):
        (self.root / 'bundles/rat/catalog.json').unlink()
        with self.assertRaisesRegex(lab.LabError, 'Cannot read bundle file'):
            lab.inventory(self.config)

    def test_duplicate_index_actor(self):
        self.index['monsters'][1]['monster'] = 'rat'
        self.write_inputs()
        with self.assertRaisesRegex(lab.LabError, 'duplicate index actor'):
            lab.inventory(self.config)

    def test_native_and_held_actor_rejected(self):
        self.stage['deferred']['encounter'].append('rat')
        self.write_inputs()
        with self.assertRaisesRegex(lab.LabError, 'exclusively'):
            lab.inventory(self.config)

    def test_unaccounted_actor_rejected(self):
        self.stage['deferred']['encounter'] = []
        self.write_inputs()
        with self.assertRaisesRegex(lab.LabError, 'exclusively'):
            lab.inventory(self.config)

    def test_unknown_held_actor_rejected(self):
        self.stage['deferred']['encounter'].append('unknown')
        self.write_inputs()
        with self.assertRaisesRegex(lab.LabError, 'absent from bundle index'):
            lab.inventory(self.config)

    def test_output_cannot_overwrite_input(self):
        before = (self.root / 'index.json').read_bytes()
        self.assertEqual(lab.main(['report', '--config', str(self.root / 'config.json'), '--output', str(self.root / 'index.json')]), 2)
        self.assertEqual((self.root / 'index.json').read_bytes(), before)

    def test_path_traversal_rejected(self):
        self.index['monsters'][0]['monster'] = '../outside'
        self.write_inputs()
        with self.assertRaisesRegex(lab.LabError, 'Invalid or duplicate'):
            lab.inventory(self.config)


if __name__ == '__main__':
    unittest.main()
