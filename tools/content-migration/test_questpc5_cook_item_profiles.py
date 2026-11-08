"""Exact retained Wiki own IDs and served client facts must agree for Cook materials."""
import copy
import json
from pathlib import Path
import unittest
from unittest.mock import patch

import questpc5_cook_item_profiles as cook

ROOT = Path(__file__).resolve().parents[2]


class CookMaterialFacts(unittest.TestCase):
    def test_exact_three_canonical_identities_and_physical_policies(self):
        rows = cook.capture(ROOT)
        self.assertEqual([r['definition']['identity']['key'] for r in rows],
                         [f'oteryn:item.tibia.i{i}' for i in (44525, 44527, 44528)])
        self.assertEqual([r['witness']['weight'] for r in rows], [1800, 600, 600])
        for row in rows:
            self.assertEqual(row['binding']['identity_namespace'], 'tibiawiki/item_id')
            self.assertEqual(row['binding']['target'], row['definition']['identity'])
            self.assertEqual(row['profile']['authoring']['item'], row['definition']['identity'])
            self.assertEqual(row['profile']['admission']['stack_class'], 'NonStackable')
            self.assertEqual(row['definition']['semantics']['use_transform'], {'state': 'UNKNOWN'})
            self.assertEqual(row['definition']['semantics']['temporal'], {'state': 'UNKNOWN'})
            self.assertEqual(row['definition']['semantics']['charges'], {'state': 'UNKNOWN'})

    def test_legacy_records_and_bindings_survive_idempotent_extension(self):
        legacy = {'identity': {'family': 'Item', 'key': 'fixture:legacy', 'revision': 'r1'}}
        reference = {'records': [copy.deepcopy(legacy)]}
        sources = {'source_identity_bindings': []}
        cook.extend_reference(reference, sources, ROOT)
        expected = copy.deepcopy((reference, sources))
        cook.extend_reference(reference, sources, ROOT)
        self.assertEqual((reference, sources), expected)
        self.assertEqual(reference['records'][0], legacy)
        self.assertEqual(len(reference['records']), 4)
        self.assertEqual(len(sources['source_identity_bindings']), 3)

    def test_a_changed_wiki_weight_is_not_overridden_by_the_client_name(self):
        original = Path.read_bytes
        facts = json.loads((ROOT / cook.FACTS).read_bytes())
        facts = copy.deepcopy(facts)
        facts['records']['44527']['observations'][0]['fields']['weight'] = '40.00'
        def read(path):
            return json.dumps(facts).encode() if path == ROOT / cook.FACTS else original(path)
        with patch.object(Path, 'read_bytes', read):
            with self.assertRaisesRegex(ValueError, 'COOK_ITEM_WIKI_IDENTITY:44527'):
                cook.capture(ROOT)


if __name__ == '__main__':
    unittest.main()
