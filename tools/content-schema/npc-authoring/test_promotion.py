"""Unit tests for validate_promotion.py against the committed promotion candidates sample and
mutated copies of it.

Usage: python -m unittest test_promotion.py
"""
import json
import unittest
from pathlib import Path

import promotion_candidates
import validate_promotion

SAMPLE_PATH = Path(__file__).resolve().parent / 'samples' / 'promotion-candidates-v1.json'


def load_sample():
    return json.loads(SAMPLE_PATH.read_text(encoding='utf-8'))


def find_candidate(report, name):
    return next(c for c in report['candidates'] if c['name'] == name)


class PromotionValidatorTests(unittest.TestCase):
    def test_committed_sample_is_valid(self):
        report = load_sample()
        self.assertEqual(validate_promotion.errors(report), [])
        # a punctuation-only name ('...') has no slug and is held, never keyed 'oteryn:npc.'
        self.assertIn('...', {h['name'] for h in report['held'] if h['reason'] == 'EMPTY_SLUG'})

    def test_bad_key_format_fails(self):
        report = load_sample()
        candidate = find_candidate(report, 'Anderson')
        candidate['identity']['key'] = 'oteryn:npc:anderson'  # colon instead of dot
        errs = validate_promotion.errors(report)
        self.assertTrue(any('does not match the NPC key pattern' in e for e in errs))

    def test_key_name_slug_mismatch_fails(self):
        report = load_sample()
        candidate = find_candidate(report, 'Anderson')
        candidate['identity']['key'] = 'oteryn:npc.someone_else'
        errs = validate_promotion.errors(report)
        self.assertTrue(any('!= expected' in e and 'candidates' in e for e in errs))

    def test_duplicate_key_fails(self):
        report = load_sample()
        anderson = find_candidate(report, 'Anderson')
        other = next(c for c in report['candidates']
                      if c['name'] != 'Anderson' and c['identity']['key'] != 'oteryn:npc.')
        other['identity']['key'] = anderson['identity']['key']
        errs = validate_promotion.errors(report)
        self.assertTrue(any('duplicate candidate key' in e for e in errs))

    def test_single_source_candidate_without_wiki_fails(self):
        report = load_sample()
        single = next(c for c in report['candidates'] if len(c['provenance']) == 1 and c['wiki'] is not None)
        single['wiki'] = None
        errs = validate_promotion.errors(report)
        self.assertTrue(any('no wiki confirmation' in e for e in errs))

    def test_left_out_duplicate_of_promoted_route_fails(self):
        report = load_sample()
        candidate = next(c for c in report['candidates'] if c['travel_service'] and c['travel_service']['routes'])
        keyword = candidate['travel_service']['routes'][0]['destination_keyword']
        candidate['left_out'].append({'fact': f'travel.{keyword}', 'reason': 'ROUTE_UNCONFIRMED'})
        errs = validate_promotion.errors(report)
        self.assertTrue(any('also promoted as route' in e for e in errs))

    def test_text_field_injected_fails(self):
        report = load_sample()
        candidate = find_candidate(report, 'Anderson')
        candidate['presentation']['description'] = {'text': 'hello', 'sha256': 'x' * 64}
        errs = validate_promotion.errors(report)
        self.assertTrue(any('forbidden text-bearing key' in e for e in errs))

    def test_totals_mismatch_fails(self):
        report = load_sample()
        report['totals']['candidates'] += 1
        errs = validate_promotion.errors(report)
        self.assertTrue(any('totals.candidates' in e for e in errs))

    def test_travel_service_key_slug_mismatch_fails(self):
        report = load_sample()
        candidate = next(c for c in report['candidates'] if c['travel_service'])
        candidate['travel_service']['identity']['key'] = 'oteryn:service.travel.someone_else'
        errs = validate_promotion.errors(report)
        self.assertTrue(any('travel_service' in e and '!= expected' in e for e in errs))

    def test_slug_folds_and_normalizes_names(self):
        self.assertEqual(promotion_candidates.slug('Captain Bluebear'), 'captain_bluebear')
        self.assertEqual(promotion_candidates.slug("Ab'Dendriel Guard"), 'ab_dendriel_guard')
        self.assertEqual(promotion_candidates.slug('Éàçüö'), 'eacuo')


if __name__ == '__main__':
    unittest.main()
