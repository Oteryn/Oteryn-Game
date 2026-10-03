import copy
import unittest
from unittest.mock import patch
import complete_monster_quality as quality


class QualityTests(unittest.TestCase):
    def loot(self):
        ref = {'family': 'Item', 'key': 'canary:item/3031', 'revision': 'canary-abc'}
        monster = {'loot': {'entries': [{'item': ref, 'min_count': 1, 'max_count': 2,
                                        'probability_percent': 0.1234}]}}
        catalog = {'definitions': [ref]}
        manifest = {'sources': [{'kind': 'mediawiki', 'page_id': 1, 'revision_id': 2,
                                 'content_sha256': 'a' * 64}], 'entries': [
            {'source_index': 0, 'destination': '/monster/loot/entries/0', 'status': 'mapped',
             'resolution': 'LOW_CONFIDENCE_WIKI_ADDITION'}]}
        return monster, catalog, manifest

    def test_qualified_loot_keeps_original_chance_and_uncertainty(self):
        data = self.loot()
        before = copy.deepcopy(data)
        result = quality.qualify_loot(*data)
        self.assertEqual(data, before)
        self.assertEqual(result[0]['wiki_source']['revision_id'], 2)
        self.assertIn('LOW_CONFIDENCE', result[0]['resolution'])

    def test_wrong_item_revision_fails(self):
        monster, catalog, manifest = self.loot()
        catalog['definitions'] = [dict(catalog['definitions'][0], revision='other')]
        with self.assertRaisesRegex(ValueError, 'exact catalog'):
            quality.qualify_loot(monster, catalog, manifest)

    def test_fractional_ppm_fails(self):
        monster, catalog, manifest = self.loot()
        monster['loot']['entries'][0]['probability_percent'] = 0.123456
        with self.assertRaisesRegex(ValueError, 'exact ppm'):
            quality.qualify_loot(monster, catalog, manifest)

    def test_stale_source_pointer_fails(self):
        monster, catalog, manifest = self.loot()
        manifest['entries'][0]['destination'] = '/monster/loot/entries/9'
        with self.assertRaisesRegex(ValueError, 'outside'):
            quality.qualify_loot(monster, catalog, manifest)

    def test_source_blob_mismatch_cannot_classify(self):
        source = {'path': 'monster/test.lua', 'revision': 'abc', 'blob_sha1': '0' * 40}
        with patch('subprocess.check_output', return_value=b'Game.createMonsterType("Boss")'):
            with self.assertRaisesRegex(ValueError, 'blob mismatch'):
                quality.registration_evidence(source, '/tmp')

    def test_quality_flags_preserve_unknown_global_and_prior_flags(self):
        original = {'monsters': [{'monster': 'dark_merudri', 'completion_flags': ['NEEDS_VERIFICATION']}]}
        changed = quality.apply_flag_additions(original, {'flag_additions': {
            'dark_merudri': ['NON_GLOBAL_TEMPLATE_BALANCE']}})
        self.assertEqual(changed['monsters'][0]['completion_flags'],
                         ['NEEDS_VERIFICATION', 'NON_GLOBAL_TEMPLATE_BALANCE'])
        self.assertEqual(original['monsters'][0]['completion_flags'], ['NEEDS_VERIFICATION'])


if __name__ == '__main__':
    unittest.main()
