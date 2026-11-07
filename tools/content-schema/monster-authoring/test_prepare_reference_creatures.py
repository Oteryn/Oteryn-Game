"""Reference completion keeps exact identities and never silently admits pending mechanics."""
import copy
import json
import os
import tempfile
import unittest
from pathlib import Path

import prepare_reference_creatures as prep


class ReferencePreparation(unittest.TestCase):
    def test_correction_is_owner_scoped_and_typed(self):
        monster = {'creature': {'identity': {'key': 'canary:creature/grand_mother_foulscale'}},
                   'behavior': {'summons': [{'creature': {'family': 'Creature', 'key': 'canary:creature/dragon_hatchlings'}}]}}
        deps = {'description': 'canary:creature/dragon_hatchlings',
                'abilities': [{'creature': {'family': 'Creature', 'key': 'canary:creature/dragon_hatchlings'}}]}
        catalog = {'definitions': [{'family': 'Creature', 'key': 'canary:creature/dragon_hatchlings'}]}
        before = copy.deepcopy((monster, deps, catalog))
        m, d, c, report = prep.canonical_reference_correction(monster, deps, catalog)
        self.assertEqual((monster, deps, catalog), before)
        self.assertEqual(m['behavior']['summons'][0]['creature']['key'], 'canary:creature/dragon_hatchling')
        self.assertEqual(d['description'], 'canary:creature/dragon_hatchlings')
        self.assertEqual(c['definitions'][0]['key'], 'canary:creature/dragon_hatchling')
        self.assertEqual(report['changed_typed_references'], 3)
        self.assertFalse(report['global_alias_created'])

    def test_correction_refuses_other_owner_or_missing_exact_reference(self):
        for key in ('canary:creature/dragon', 'canary:creature/grand_mother_foulscale'):
            with self.assertRaises(ValueError):
                prep.canonical_reference_correction({'creature': {'identity': {'key': key}}}, {}, {})

    def test_source_allowlist_keeps_bone_bear_crystal_identity(self):
        sample = json.loads(prep.SAMPLE.read_text())
        rows = {r['slug']: r for r in sample['creatures']}
        self.assertEqual(len(rows), 6)
        self.assertEqual(rows['bone_bear']['repository'], 'zimbadev/crystalserver')
        self.assertEqual(rows['bone_bear']['revision'], '00ce02a57ca5a12e48f32a3476e37471167e4c3f')
        self.assertEqual(rows['parasite']['literal_health'], 550)
        self.assertEqual(rows['carnisylvan_sapling']['literal_health'], 750)
        self.assertEqual(rows['eruption_of_destruction']['literal_health'], 8500)

    def test_digest_covers_all_four_bundle_files(self):
        with tempfile.TemporaryDirectory() as directory:
            out = Path(directory)
            for name in prep.BUNDLE_FILES:
                (out / name).write_text('{}\n')
            before = prep.digest(out)
            (out / 'manifest.json').write_text('{"unresolved":true}\n')
            self.assertNotEqual(before, prep.digest(out))

    def test_prepare_refuses_repository_or_nonempty_output_before_source_reads(self):
        with self.assertRaises(ValueError):
            prep.prepare(Path('/absent-canary'), Path('/absent-crystal'), prep.REPO / 'tmp')
        with tempfile.TemporaryDirectory() as directory:
            out = Path(directory)
            (out / 'qualified-result.json').write_text('{}')
            with self.assertRaises(ValueError):
                prep.prepare(Path('/absent-canary'), Path('/absent-crystal'), out)
            self.assertEqual((out / 'qualified-result.json').read_text(), '{}')

    @unittest.skipUnless(os.environ.get('OTERYN_CANARY') and os.environ.get('OTERYN_CRYSTAL'), 'pinned donors not selected')
    def test_real_donors_preserve_stats_and_pending_custom_mechanics(self):
        with tempfile.TemporaryDirectory() as directory:
            report = prep.prepare(Path(os.environ['OTERYN_CANARY']), Path(os.environ['OTERYN_CRYSTAL']), Path(directory))
            self.assertEqual(len(report['creatures']), 6)
            self.assertFalse(report['native_admission_authorized'])
            for row in report['creatures']:
                self.assertEqual(row['structure_errors'], [])
                self.assertEqual(row['stats']['max_health'], row['source']['literal_health'])
                self.assertEqual(row['digest'], prep.digest(Path(directory) / row['monster']))
            # Custom scripts stay explicit; no omission can become an accidental success.
            parasite = next(r for r in report['creatures'] if r['monster'] == 'parasite')
            self.assertTrue(any(r['source_field'] == 'events=ParasiteDeath' for r in parasite['open_rows']))


    @unittest.skipUnless(os.environ.get('OTERYN_CANARY') and os.environ.get('OTERYN_CRYSTAL'), 'pinned donors not selected')
    def test_playable_preserves_sapling_damage_and_flags_exact_omissions(self):
        with tempfile.TemporaryDirectory() as directory:
            out = Path(directory)
            report = prep.prepare_playable(Path(os.environ['OTERYN_CANARY']), Path(os.environ['OTERYN_CRYSTAL']), out)
            self.assertEqual(len(report['creatures']), 6)
            for row in report['creatures']:
                self.assertEqual(row['readiness_errors'], [])
                source = json.loads((out / 'source-bundles' / row['monster'] / 'monster.json').read_text())
                playable = json.loads((out / 'bundles' / row['monster'] / 'monster.json').read_text())
                self.assertEqual(source['creature']['stats'], playable['creature']['stats'])
                self.assertEqual(source.get('loot'), playable.get('loot'))
                if row['monster'] != 'bone_bear':
                    self.assertIn('SOURCE_MECHANICS_OMITTED', row['quality_flags'])
                    self.assertEqual(len(row['omitted_source_mechanics']), 1)
            sapling = json.loads((out / 'bundles/carnisylvan_sapling/monster.json').read_text())
            cast = sapling['behavior']['attacks'][0]
            self.assertEqual(cast['magnitude'], {'minimum': 700, 'maximum': 1000})
            self.assertEqual(cast['interval_ms'], 2000)
            self.assertEqual(cast['chance_percent'], 100)
            self.assertEqual(cast['range_tiles'], 1)
            deps = json.loads((out / 'bundles/carnisylvan_sapling/dependencies.json').read_text())
            self.assertEqual(deps['effects'][0]['operation'], 'damage')
            self.assertEqual(deps['effects'][0]['damage_type'], 'fire')
            self.assertEqual(deps['abilities'][0]['area']['matrix']['north'][3], 'xxxCxxx')
            parasite = next(r for r in report['creatures'] if r['monster'] == 'parasite')
            self.assertEqual(parasite['omitted_source_mechanics'][0]['source_row']['status'], 'unresolved_semantics')


if __name__ == '__main__':
    unittest.main()
