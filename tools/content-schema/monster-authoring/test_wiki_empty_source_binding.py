"""Wiki-authored empty loot must keep its reviewed source facts after conversion."""
import copy
import json
import os
import unittest
from pathlib import Path

import canary_batch as cb
import wiki_authored as wiki
from validate_monster import validate


class WikiEmptySourceBinding(unittest.TestCase):
    def setUp(self):
        self.sources = json.loads(wiki.SAMPLE.read_text())['sources']

    def test_actual_dark_merudri_population_bundle_preserves_wiki_empty_loot(self):
        source = Path(os.environ.get('OTERYN_CANARY_TEST_SOURCE', '/workspace/monster-reference-sources/canary'))
        self.assertTrue((source / 'data/items/items.xml').exists(), 'pinned population fixture is required')
        objects = cb.load_appearance_objects(source / 'data/items/appearances.dat')
        items = cb.load_items_xml(source / 'data/items/items.xml')
        names, index = cb.name_index(objects, items)
        converter = cb.Converter(source, objects, items, names, index)
        (slug, monster, deps, catalog, manifest, binding), = wiki.bundles(converter)
        self.assertEqual(slug, 'dark_merudri')
        self.assertEqual(validate(monster, deps, catalog, manifest), [])
        self.assertNotIn('loot', monster['creature'])
        self.assertNotIn('loot', monster)
        empty = [row for row in manifest['entries'] if row['source_field'] == 'infobox.loot']
        self.assertEqual(len(empty), 2)
        for row in empty:
            self.assertEqual(row['status'], 'approved_omission')
            self.assertNotIn('destination', row)
            source_meta = manifest['sources'][row['source_index']]
            self.assertIn(source_meta['api'], (cb.WIKI_API, cb.BR_API))
            self.assertEqual(source_meta['title'], 'Dark Merudri')
            meta = next(v for v in self.sources.values() if v['api'] == source_meta['api'] and v['title'] == source_meta['title'])
            self.assertEqual(row['source_line'], meta['facts']['loot']['line'])
            self.assertEqual(source_meta['revision_id'], meta['revision_id'])
            self.assertIn(meta['facts']['loot']['value'], row['resolution'])

    def test_changed_loot_fact_or_mapped_loot_cannot_be_approved_empty(self):
        row = {'source_field': 'loot', 'kind': 'field', 'status': 'approved_omission', 'resolution': 'Empty collection'}
        for source_name in wiki.EMPTY_LOOT_FACTS:
            changed = copy.deepcopy(self.sources)
            changed[source_name]['facts']['loot']['value'] = 'Gold Coin'
            with self.assertRaisesRegex(ValueError, 'loot changed'):
                wiki.manifest_for(changed, [row], {})
        with self.assertRaisesRegex(ValueError, 'explicit empty-source omission'):
            wiki.manifest_for(self.sources, [{**row, 'status': 'mapped', 'destination': '/monster/creature/loot'}], {})
        with self.assertRaisesRegex(ValueError, 'no source mapping'):
            wiki.manifest_for(self.sources, [{'source_field': 'inventedField', 'status': 'mapped'}], {})


if __name__ == '__main__':
    unittest.main()
