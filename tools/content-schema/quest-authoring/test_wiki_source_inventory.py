import copy
import unittest
from pathlib import Path

import wiki_source_inventory as b

SAMPLES = Path(__file__).parent / 'samples/wiki-source-specs'
PREVIOUS = SAMPLES / 'previous25.json'


class InventoryTests(unittest.TestCase):
    def setUp(self):
        self.catalogue = b.read(SAMPLES/'selection-input.json')
        self.previous = b.read(PREVIOUS)
        self.coverage = b.read(SAMPLES/'historical-status.json')
        self.added = b.new_specs(self.catalogue,self.previous,self.coverage)

    def test_all_105_unbound_titles_included(self):
        combined = b.merge_inventory(self.catalogue,self.previous,self.added)
        self.assertEqual(combined['titles'],105)
        self.assertEqual(len(self.added),80)
        self.assertEqual({e['wiki_title'] for e in combined['entries']},{q['wiki_title'] for q in b.currently_unbound(self.catalogue)})

    def test_historical_present_records_are_not_excluded(self):
        self.assertTrue(all(any(v!='ABSENT' for v in e['historical_donor_status'].values()) for e in self.added))
        legacy = [e for e in self.added if all(v=='ABSENT' for v in e['historical_donor_status'].values())]
        self.assertEqual(legacy,[])

    def test_previous_25_records_unchanged(self):
        combined = b.merge_inventory(self.catalogue,self.previous,self.added)
        by_title = {e['wiki_title']:e for e in combined['entries']}
        self.assertTrue(all(b.fingerprint(e)==b.fingerprint(by_title[e['wiki_title']]) for e in self.previous['entries']))

    def test_exact_186_source_field_sets(self):
        self.assertEqual(sum(len(e['source_fields_with_provenance']) for e in self.added),186)
        self.assertTrue(all(s['revid'] and s['content_sha256'] and s['access_method']=='remote_desktop_browser' for e in self.added for s in e['source_fields_with_provenance']))

    def test_missing_title_is_rejected(self):
        with self.assertRaisesRegex(ValueError,'Incomplete'):
            b.merge_inventory(self.catalogue,self.previous,self.added[:-1])

    def test_duplicate_title_is_rejected(self):
        with self.assertRaisesRegex(ValueError,'Duplicate'):
            b.merge_inventory(self.catalogue,self.previous,self.added+self.added[:1])

    def test_false_definition_completeness_is_rejected(self):
        self.added[0]['definition_complete'] = True
        with self.assertRaisesRegex(ValueError,'completeness'):
            b.merge_inventory(self.catalogue,self.previous,self.added)

    def test_missing_revision_is_rejected(self):
        row = next(q for q in b.currently_unbound(self.catalogue) if q['wiki_title']==self.added[0]['wiki_title'])
        row['fresh_sources'][0]['revid'] = None
        with self.assertRaisesRegex(ValueError,'revision'):
            b.new_specs(self.catalogue,self.previous,self.coverage)

    def test_stale_content_hash_is_rejected(self):
        row = next(q for q in b.currently_unbound(self.catalogue) if q['wiki_title']==self.added[0]['wiki_title'])
        row['fresh_sources'][0]['content_sha256'] = '0'*64
        with self.assertRaisesRegex(ValueError,'Stale'):
            b.new_specs(self.catalogue,self.previous,self.coverage)

    def test_twenty_unparsed_requirements_keep_exact_provenance(self):
        b.add_unparsed_holds(self.added)
        holds = [h for e in self.added for h in e['unresolved'] if h['field_group']=='Source requirement not parsed']
        self.assertEqual(len(holds),20)
        self.assertTrue(all(h['classification']=='UNKNOWN' and h['evidence']['revid'] and h['evidence']['line'] and h['evidence']['line_sha256'] for h in holds))

    def test_no_semantic_rewrite_or_step_invention(self):
        rows = {q['wiki_title']:q for q in self.catalogue['quests']}
        for entry in self.added:
            expected = rows[entry['wiki_title']]['fresh_sources']
            for actual, source in zip(entry['source_specification'],expected):
                self.assertEqual(actual['semantic_enrichment'],source['source_fields']['semantic_enrichment'])
            self.assertFalse(entry['definition_complete'])
            self.assertEqual(entry['runtime_readiness'],'UNKNOWN')

    def test_field_provider_conflicts_are_preserved(self):
        entry = next(e for e in self.added if e['wiki_title']=='Citizen of Issavi Outfits Quest')
        sources = entry['source_fields_with_provenance']
        self.assertEqual({s['target_cut'] for s in sources},{'2026-09-27','2026-10-01'})
        self.assertEqual(len(sources),3)

    def test_portable_generator_recreates_exact_pinned_inventory(self):
        self.assertEqual(b.generate(SAMPLES), b.read(SAMPLES/'source-specs-105.json'))


if __name__=='__main__':
    unittest.main()
