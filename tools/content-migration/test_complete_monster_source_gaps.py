import copy
import json
from pathlib import Path
import tempfile
import unittest

import complete_monster_source_gaps as batch


class SourceGapMergeTests(unittest.TestCase):
    def prepare(self, root):
        baseline = root / 'baseline'
        source = batch.population.MONSTERS / 'samples/canary-47dfd51f/rat'
        batch.shutil.copytree(source, baseline / 'bundles/rat')
        row = {'monster': 'rat', 'file': 'mammals/rat',
               'sha256': batch.population.admission.bundle_digest(source)}
        batch.write(baseline / 'population-index.json', {'monsters': [row], 'bundles': 1})
        batch.write(baseline / 'completion-quality.json', {'original': True})
        loot, spells = root / 'loot', root / 'spells'
        for component, folder in ((loot, 'patched-bundles'), (spells, 'bundles')):
            batch.shutil.copytree(source, component / folder / 'rat')
        monster = batch.read(loot / 'patched-bundles/rat/monster.json')
        monster['loot']['entries'][0]['probability_percent'] = 17
        batch.write(loot / 'patched-bundles/rat/monster.json', monster)
        manifest = batch.read(loot / 'patched-bundles/rat/manifest.json')
        wiki = {'kind': 'mediawiki', 'api': 'https://tibia.fandom.com/api.php',
                'title': 'Loot Statistics:Rat', 'page_id': 1, 'revision_id': 2, 'content_sha256': 'a' * 64}
        manifest['sources'].append(wiki)
        manifest['entries'].append({'source_index': len(manifest['sources']) - 1,
                                    'source_file': wiki['title'], 'source_line': 1,
                                    'source_field': 'Loot2.regression', 'kind': 'field', 'status': 'mapped',
                                    'destination': '/monster/loot/entries/0/probability_percent',
                                    'resolution': 'Pinned observed estimate.'})
        batch.write(loot / 'patched-bundles/rat/manifest.json', manifest)
        spell_monster = batch.read(spells / 'bundles/rat/monster.json')
        spell_monster['behavior']['attacks'][0]['interval_ms'] += 100
        batch.write(spells / 'bundles/rat/monster.json', spell_monster)
        batch.write(loot / 'loot-applied-receipt.json', {
            'index_input_sha256': batch.sha(baseline / 'population-index.json'),
            'changed': [{'monster': 'rat', 'bundle_sha256': batch.population.admission.bundle_digest(loot / 'patched-bundles/rat'),
                         'completion_flags': ['WIKI_LOOT_ESTIMATE']}]})
        batch.write(spells / 'completion.json', {
            'actors': [{'monster': 'rat', 'original_bundle_digest': row['sha256'],
                        'bundle_digest': batch.population.admission.bundle_digest(spells / 'bundles/rat'),
                        'completion_flags': ['SOURCE_TYPED_CORE_RESTORED']}], 'encounters': []})
        encounters = root / 'encounters'
        encounters.mkdir()
        return baseline, loot, spells, encounters, monster, spell_monster, wiki

    def test_overlapping_workers_keep_loot_spell_and_exact_wiki_provenance(self):
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            baseline, loot, spells, encounters, monster, spell_monster, wiki = self.prepare(root)
            frozen = batch.population.admission.bundle_digest(baseline / 'bundles/rat')
            batch.merge(baseline, loot, spells, encounters, root / 'out')
            result = batch.read(root / 'out/bundles/rat/monster.json')
            self.assertEqual(result['loot'], monster['loot'])
            self.assertEqual(result['behavior'], spell_monster['behavior'])
            self.assertEqual(batch.population.admission.bundle_digest(baseline / 'bundles/rat'), frozen)
            manifest = batch.read(root / 'out/bundles/rat/manifest.json')
            evidence = [e for e in manifest['entries'] if e['source_field'] == 'Loot2.regression']
            self.assertEqual(len(evidence), 1)
            self.assertEqual(manifest['sources'][evidence[0]['source_index']], wiki)

    def test_changed_baseline_rejects_old_packet(self):
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            baseline, loot, spells, encounters, *_ = self.prepare(root)
            batch.write(baseline / 'population-index.json', {'monsters': [], 'bundles': 0})
            with self.assertRaisesRegex(ValueError, 'different baseline'):
                batch.merge(baseline, loot, spells, encounters, root / 'out')


if __name__ == '__main__':
    unittest.main()
