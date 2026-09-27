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

    def test_trade_service_key_slug_mismatch_fails(self):
        report = load_sample()
        candidate = next(c for c in report['candidates'] if c['trade_service'])
        candidate['trade_service']['identity']['key'] = 'oteryn:service.trade.someone_else'
        errs = validate_promotion.errors(report)
        self.assertTrue(any('trade_service' in e and '!= expected' in e for e in errs))

    def test_bad_offer_direction_fails(self):
        report = load_sample()
        candidate = next(c for c in report['candidates'] if c['trade_service'])
        candidate['trade_service']['offers'][0]['direction'] = 'Trade'
        errs = validate_promotion.errors(report)
        self.assertTrue(any('direction' in e and "not in ['BuyFromPlayer', 'SellToPlayer']" in e for e in errs))

    def test_duplicate_offer_tuple_fails(self):
        report = load_sample()
        candidate = next(c for c in report['candidates'] if c['trade_service'] and c['trade_service']['offers'])
        candidate['trade_service']['offers'].append(dict(candidate['trade_service']['offers'][0]))
        errs = validate_promotion.errors(report)
        self.assertTrue(any('duplicate offer tuple' in e for e in errs))

    def test_inconsistent_source_item_id_mapping_fails(self):
        report = load_sample()
        trade_candidates = [c for c in report['candidates'] if c['trade_service'] and c['trade_service']['offers']]
        first, second = trade_candidates[0], trade_candidates[1]
        first_offer, second_offer = first['trade_service']['offers'][0], second['trade_service']['offers'][0]
        self.assertNotEqual(first_offer['item']['key'], second_offer['item']['key'])
        second_offer['source_item_id'] = first_offer['source_item_id']  # same id, different item key
        errs = validate_promotion.errors(report)
        self.assertTrue(any('maps to item key' in e for e in errs))

    def test_missing_item_map_sha256_fails(self):
        report = load_sample()
        del report['item_map_sha256']
        errs = validate_promotion.errors(report)
        self.assertTrue(any('item_map_sha256' in e for e in errs))

    def test_with_trade_totals_mismatch_fails(self):
        report = load_sample()
        report['totals']['with_trade'] += 1
        errs = validate_promotion.errors(report)
        self.assertTrue(any('totals.with_trade' in e for e in errs))

    # -- D8: complete held NPCs from the wiki --------------------------------------------------

    def test_base_name_variant_promoted_via_wiki(self):
        report = load_sample()
        candidate = find_candidate(report, 'Uzon Back')
        self.assertEqual(len(candidate['provenance']), 1)
        self.assertIsNotNone(candidate['wiki'])
        self.assertIn({'fact': 'identity', 'rule': 'WIKI_BASE_NAME', 'chosen': 'wiki'}, candidate['arbitration'])
        self.assertEqual(validate_promotion.errors(report), [])

    def test_unplaced_with_wiki_position_promoted(self):
        report = load_sample()
        candidate = next(c for c in report['candidates']
                          if any(a['rule'] == 'WIKI_POSITION' for a in c['arbitration']))
        self.assertEqual(len(candidate['placements']), 1)
        placement = candidate['placements'][0]
        self.assertEqual(placement['origin'], 'wiki')
        self.assertIsNone(placement['direction'])
        self.assertIsNone(placement['spawn_interval_s'])
        self.assertIn({'fact': 'placements', 'rule': 'WIKI_POSITION', 'chosen': 'wiki'}, candidate['arbitration'])
        self.assertEqual(validate_promotion.errors(report), [])

    def test_wiki_origin_placement_with_direction_fails(self):
        report = load_sample()
        candidate = next(c for c in report['candidates']
                          if any(a['rule'] == 'WIKI_POSITION' for a in c['arbitration']))
        candidate['placements'][0]['direction'] = 'NORTH'
        errs = validate_promotion.errors(report)
        self.assertTrue(any('wiki-origin placement direction must be null' in e for e in errs))

    def test_bad_arbitration_rule_fails(self):
        report = load_sample()
        candidate = find_candidate(report, 'Uzon Back')
        candidate['arbitration'][0]['rule'] = 'WIKI_GUESS'
        errs = validate_promotion.errors(report)
        self.assertTrue(any("rule 'WIKI_GUESS' not in" in e for e in errs))

    def test_unplaced_npc_without_wiki_position_stays_held(self):
        report = load_sample()
        held_unplaced = {h['name'] for h in report['held'] if h['reason'] == 'UNPLACED'}
        self.assertTrue(held_unplaced)  # NPCs that are neither placed nor on the wiki with a position remain held
        self.assertNotIn('Uzon Back', held_unplaced)
        self.assertFalse(held_unplaced & {c['name'] for c in report['candidates']})


if __name__ == '__main__':
    unittest.main()
