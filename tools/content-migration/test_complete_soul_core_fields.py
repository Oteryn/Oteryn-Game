import importlib.util
import json
import tempfile
import unittest
from pathlib import Path

spec = importlib.util.spec_from_file_location('soul_core', Path(__file__).with_name('complete_soul_core_fields.py'))
core = importlib.util.module_from_spec(spec)
spec.loader.exec_module(core)

class SoulCoreTests(unittest.TestCase):
    def fixture(self, directory, primary='canary'):
        population, sources = directory / 'population', directory / 'sources'
        bundle = population / 'bundles' / 'feversleep'
        bundle.mkdir(parents=True)
        index = {'source': {'revision': 'canary-pin'}, 'crystal': {'revision': 'crystal-pin'},
                 'monsters': [{'monster': 'feversleep'}]}
        (population / 'population-index.json').write_text(json.dumps(index))
        monster = {'creature': {'display_name': 'Feversleep', 'identity': {'revision': 'canary-r1'}}}
        (bundle / 'monster.json').write_text(json.dumps(monster))
        (bundle / 'catalog.json').write_text(json.dumps({'definitions': []}))
        (bundle / 'manifest.json').write_text(json.dumps({'sources': [
            {'repository': core.REPOSITORIES[primary], 'revision': primary + '-pin'}]}))
        for project, item_id in [('canary', 47783), ('crystal', 47779)]:
            path = sources / project / 'data/items/items.xml'
            path.parent.mkdir(parents=True)
            path.write_text(f'<item id="{item_id}" name="feversleep soul core">\n')
        audit = {'index_sha256': core.sha(population / 'population-index.json'),
                 'source_pins': {'canary': 'canary-pin', 'crystal': 'crystal-pin'},
                 'soul_core_mapping_candidates': [{'monster': 'feversleep', 'display_name': 'Feversleep',
                    'matching_soul_core_items': {'canary': {'id': 47783, 'line': 1},
                                                'crystal': {'id': 47779, 'line': 1}}}]}
        audit['source_proofs'] = [{'project': project, 'path': 'data/items/items.xml',
            'revision': project + '-pin', 'sha256': core.sha(sources / project / 'data/items/items.xml')}
            for project in core.REPOSITORIES]
        audit_path = directory / 'audit.json'
        audit_path.write_text(json.dumps(audit))
        return population, audit_path, sources

    def test_conflicting_ids_choose_actual_donor_not_namespace(self):
        with tempfile.TemporaryDirectory() as temp:
            population, audit, sources = self.fixture(Path(temp), 'crystal')
            packet = core.prepare(population, audit, sources, {47779: 'oteryn:item.accepted'}, {})
            self.assertEqual(packet['patches'][0]['value']['key'], 'canary:item/47779')
            self.assertEqual(packet['patches'][1]['pointer'], '/definitions/-')
            self.assertEqual(packet['patches'][0]['source']['repository'], 'zimbadev/crystalserver')
            self.assertFalse(packet['runtime_qualification']['conditional_soul_core_drop_consumer_implemented'])
            self.assertFalse(any('/loot' in patch['pointer'] for patch in packet['patches']))

    def test_unadmitted_selected_item_never_falls_back_to_other_donor(self):
        with tempfile.TemporaryDirectory() as temp:
            population, audit, sources = self.fixture(Path(temp))
            packet = core.prepare(population, audit, sources, {47779: 'oteryn:item.other'}, {})
            self.assertEqual(packet['patches'], [])
            self.assertEqual(packet['unresolved'][0]['source']['source_item_id'], 47783)
            self.assertEqual(packet['counts']['unresolved_actors'], 1)

    def test_wrong_source_line_or_item_identity_is_unresolved(self):
        with tempfile.TemporaryDirectory() as temp:
            population, audit, sources = self.fixture(Path(temp))
            (sources / 'canary/data/items/items.xml').write_text('<item id="47783" name="another soul core">\n')
            with self.assertRaisesRegex(ValueError, 'source SHA drift'):
                core.prepare(population, audit, sources, {47783: 'oteryn:item.accepted'}, {})

    def test_monster_donor_revision_drift_rejected(self):
        with tempfile.TemporaryDirectory() as temp:
            population, audit, sources = self.fixture(Path(temp))
            manifest = population / 'bundles/feversleep/manifest.json'
            manifest.write_text(json.dumps({'sources': [{'repository': 'opentibiabr/canary', 'revision': 'wrong'}]}))
            with self.assertRaisesRegex(ValueError, 'donor source revision drift'):
                core.prepare(population, audit, sources, {47783: 'oteryn:item.accepted'}, {})

    def test_stale_population_baseline_rejected(self):
        with tempfile.TemporaryDirectory() as temp:
            population, audit, sources = self.fixture(Path(temp))
            with (population / 'population-index.json').open('a') as file:
                file.write('\n')
            with self.assertRaisesRegex(ValueError, 'baseline SHA mismatch'):
                core.prepare(population, audit, sources, {}, {})

    def test_conflict_without_primary_donor_is_not_guessed(self):
        with self.assertRaisesRegex(ValueError, 'conflict'):
            core.choose_item({'canary': {'id': 1}, 'crystal': {'id': 2}}, 'other')

    def test_catalog_already_declared_ref_is_not_duplicated(self):
        with tempfile.TemporaryDirectory() as temp:
            population, audit, sources = self.fixture(Path(temp))
            ref = {'family': 'Item', 'key': 'canary:item/47783', 'revision': 'canary-r1'}
            (population / 'bundles/feversleep/catalog.json').write_text(json.dumps({'definitions': [ref]}))
            packet = core.prepare(population, audit, sources, {47783: 'oteryn:item.accepted'}, {})
            self.assertEqual(len(packet['patches']), 1)
            self.assertEqual(packet['patches'][0]['value'], ref)

if __name__ == '__main__':
    unittest.main()
