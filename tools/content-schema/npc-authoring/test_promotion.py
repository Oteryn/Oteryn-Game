"""Unit tests for validate_promotion.py against the committed promotion candidates sample and
mutated copies of it.

Usage: python -m unittest test_promotion.py
"""
import hashlib
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


def find_held(report, name):
    return next(h for h in report['held'] if h['name'] == name)


def make_builder(npcs):
    return promotion_candidates.Builder({'npcs': npcs, 'trade': {}}, {'records': []})


def make_bundle(name, source='crystal', sha='0' * 64, placements=()):
    return {'key': f'{source}:npc/{promotion_candidates.slug(name)}', 'status': 'RESOLVED',
            'source': {'sha256': sha}, 'placements': list(placements), 'services': {'travel': [], 'trade': None},
            'definition': {'name': name, 'profession': 'None',
                            'presentation': {'outfit': {}, 'speech_bubble': None}, 'movement': None}}


def make_placement(x, y, z, direction='NORTH', interval=60, radius=0):
    return {'position': {'x': x, 'y': y, 'z': z}, 'direction': direction,
            'spawn_interval_s': interval, 'spawn_radius': radius}


class PromotionValidatorTests(unittest.TestCase):
    def test_committed_sample_is_valid(self):
        report = load_sample()
        self.assertEqual(validate_promotion.errors(report), [])
        # a punctuation-only name ('...') has no slug and is held, never keyed 'oteryn:npc.'
        self.assertIn('...', {h['name'] for h in report['held'] if h['reason'] == 'EMPTY_SLUG'})

    def test_wiki_price_requires_br_facts(self):
        report = load_sample()
        del report['br_facts_sha256']
        report['decisions'] = validate_promotion.DECISIONS
        errs = validate_promotion.errors(report)
        self.assertIn('WIKI_PRICE arbitration without br_facts_sha256 (D12)', errs)

    def test_wiki_price_must_name_its_offer(self):
        report = load_sample()
        ahmet = find_candidate(report, 'Ahmet')
        row = next(r for r in ahmet['arbitration'] if r['rule'] == 'WIKI_PRICE')
        row['fact'] = 'trade.999999.SellToPlayer'
        self.assertTrue(any('names no admitted offer' in e for e in validate_promotion.errors(report)))

    def test_wiki_price_must_match_the_offer_price(self):
        report = load_sample()
        ahmet = find_candidate(report, 'Ahmet')
        row = next(r for r in ahmet['arbitration'] if r['rule'] == 'WIKI_PRICE')
        source_item_id = int(row['fact'].split('.')[1])
        offer = next(o for o in ahmet['trade_service']['offers']
                     if o['source_item_id'] == source_item_id and o['direction'] == 'SellToPlayer')
        offer['unit_price'] = 999999
        self.assertTrue(any('!= WIKI_PRICE price' in e for e in validate_promotion.errors(report)))

    def test_wiki_price_item_name_is_the_offer_item(self):
        report = load_sample()
        names = validate_promotion.registry_item_names()
        self.assertEqual(validate_promotion.item_name_errors(report, names), [])
        ahmet = find_candidate(report, 'Ahmet')
        row = next(r for r in ahmet['arbitration'] if r['rule'] == 'WIKI_PRICE' and r['item_name'] == 'fishing rod')
        row['item_name'] = 'shovel'  # another item's wiki price must not justify this offer
        self.assertTrue(any('is not the offer\'s registered item' in e
                            for e in validate_promotion.item_name_errors(report, names)))

    def test_wiki_price_matches_both_pinned_wikis(self):
        snapshot = json.dumps({'npcs': [], 'trade': {'ahmet': [
            {'item': 'Fishing Rod', 'buy_price': 150, 'sell_price': None}]}}).encode()
        br_facts = json.dumps({'pages': [{'title': 'Ahmet', 'name': 'Ahmet', 'trades': {
            'SellToPlayer': {'Fishing Rod': [150]}, 'BuyFromPlayer': {}}}]}).encode()
        def report(price):
            return {'snapshot_sha256': hashlib.sha256(snapshot).hexdigest(),
                    'br_facts_sha256': hashlib.sha256(br_facts).hexdigest(),
                    'candidates': [{'name': 'Ahmet', 'arbitration': [
                        {'fact': 'trade.3483.SellToPlayer', 'rule': 'WIKI_PRICE', 'chosen': 'wiki',
                         'item_name': 'fishing rod', 'price': price}]}]}
        self.assertEqual(validate_promotion.wiki_price_errors(report(150), snapshot, br_facts), [])
        self.assertTrue(validate_promotion.wiki_price_errors(report(999999), snapshot, br_facts))
        self.assertEqual(validate_promotion.wiki_price_errors(report(150), b'{}', br_facts),
                         ['--snapshot does not match snapshot_sha256'])

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

    def test_wiki_position_without_wiki_page_fails(self):
        report = load_sample()
        candidate = next(c for c in report['candidates']
                          if len(c['provenance']) == 2
                          and any(a['rule'] == 'WIKI_POSITION' for a in c['arbitration']))
        candidate['wiki'] = None
        errs = validate_promotion.errors(report)
        self.assertTrue(any("rule 'WIKI_POSITION' requires a wiki page" in e for e in errs))

    def test_wiki_origin_placement_with_radius_fails(self):
        report = load_sample()
        candidate = next(c for c in report['candidates']
                          if any(a['rule'] == 'WIKI_POSITION' for a in c['arbitration']))
        candidate['placements'][0]['spawn_radius'] = 999
        errs = validate_promotion.errors(report)
        self.assertTrue(any('wiki-origin placement spawn_radius must be null' in e for e in errs))

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

    # -- D8: actualname aliasing and the strict spelling rule ----------------------------------

    def test_within_one_edit_examples(self):
        within = promotion_candidates.within_one_edit
        self.assertTrue(within('awarness of the emperor', 'awareness of the emperor'))  # deletion
        self.assertTrue(within('dahr-enpa rahng', 'dhar-enpa rahng'))  # adjacent transposition
        self.assertTrue(within('cat', 'cats'))  # insertion
        self.assertTrue(within('kitten', 'sitten'))  # substitution
        self.assertFalse(within('abc', 'abc'))  # equal is not "one edit"
        self.assertFalse(within('abcdefghij', 'abzdefyhij'))  # two substitutions

    def test_wiki_spelling_promotes_typo_named_npc(self):
        report = load_sample()
        for name in ('Awarness Of The Emperor', 'Dahr-Enpa Rahng'):
            candidate = find_candidate(report, name)
            self.assertEqual(len(candidate['provenance']), 1)
            self.assertIn({'fact': 'identity', 'rule': 'WIKI_SPELLING', 'chosen': 'wiki'}, candidate['arbitration'])
        self.assertEqual(validate_promotion.errors(report), [])

    def test_actualname_confirms_omniphant(self):
        report = load_sample()
        candidate = find_candidate(report, 'Omniphant')
        self.assertEqual(len(candidate['provenance']), 1)
        self.assertIsNotNone(candidate['wiki'])
        # matched directly by actualname alias, not the fuzzy spelling rule
        self.assertFalse(any(a['fact'] == 'identity' for a in candidate['arbitration']))

    def test_fuzzy_match_ambiguous_between_two_wiki_names_is_not_unique(self):
        builder = make_builder([
            {'pageid': 1, 'title': 'Page A', 'name': 'aaaaaaaaab', 'actualname': None, 'position': None},
            {'pageid': 2, 'title': 'Page B', 'name': 'baaaaaaaaa', 'actualname': None, 'position': None},
        ])
        matches = builder.fuzzy_wiki_matches('aaaaaaaaaa')
        self.assertEqual(len(matches), 2)
        result = builder.candidate({'crystal': make_bundle('Aaaaaaaaaa')})
        self.assertIsNone(result)
        self.assertEqual(builder.held[-1]['reason'], 'SINGLE_SOURCE_NOT_ON_WIKI')

    def test_short_name_is_not_tried_for_spelling(self):
        builder = make_builder([
            {'pageid': 1, 'title': 'Shortnyme', 'name': 'Shortnyme', 'actualname': None, 'position': None},
        ])
        # 'Shortnym' (8 chars, < SPELLING_MIN_LENGTH) is one deletion away from 'Shortnyme' but too short to try
        self.assertEqual(len(promotion_candidates.normalize_name('Shortnym')), 8)
        result = builder.candidate({'crystal': make_bundle('Shortnym')})
        self.assertIsNone(result)
        self.assertEqual(builder.held[-1]['reason'], 'SINGLE_SOURCE_NOT_ON_WIKI')

    # -- Codex review: alias collisions and per-rule arbitration checks ------------------------

    def test_ambiguous_alias_without_exact_title_leaves_npc_held(self):
        # two different pages both expose actualname 'Twinname'; neither's title is 'Twinname'
        builder = make_builder([
            {'pageid': 1, 'title': 'Page One', 'name': 'Page One', 'actualname': 'Twinname',
             'position': {'x': 1, 'y': 1, 'z': 1}},
            {'pageid': 2, 'title': 'Page Two', 'name': 'Page Two', 'actualname': 'Twinname',
             'position': {'x': 2, 'y': 2, 'z': 2}},
        ])
        self.assertNotIn('twinname', builder.wiki)  # ambiguous alias: dropped, not bound to whichever came first
        result = builder.candidate({'crystal': make_bundle('Twinname')})
        self.assertIsNone(result)
        self.assertEqual(builder.held[-1]['reason'], 'SINGLE_SOURCE_NOT_ON_WIKI')

    def test_exact_title_wins_over_colliding_alias(self):
        # 'Real Page' is an exact title match; another page's alias collides with it but must not win
        builder = make_builder([
            {'pageid': 1, 'title': 'Real Page', 'name': 'Real Page', 'actualname': None, 'position': None},
            {'pageid': 2, 'title': 'Other Page', 'name': 'Other Page', 'actualname': 'Real Page', 'position': None},
        ])
        self.assertEqual(builder.wiki['real page']['pageid'], 1)

    def test_identity_rule_on_two_source_candidate_fails(self):
        report = load_sample()
        candidate = next(c for c in report['candidates'] if len(c['provenance']) == 2)
        candidate['arbitration'].append({'fact': 'identity', 'rule': 'WIKI_BASE_NAME', 'chosen': 'wiki'})
        errs = validate_promotion.errors(report)
        self.assertTrue(any('requires a single-source candidate' in e for e in errs))

    def test_wiki_position_with_identity_fact_fails(self):
        report = load_sample()
        candidate = next(c for c in report['candidates']
                          if any(a['rule'] == 'WIKI_POSITION' for a in c['arbitration']))
        row = next(a for a in candidate['arbitration'] if a['rule'] == 'WIKI_POSITION')
        row['fact'] = 'identity'
        errs = validate_promotion.errors(report)
        self.assertTrue(any("fact 'identity' != 'placements'" in e for e in errs))

    def test_wiki_placement_without_wiki_position_row_fails(self):
        report = load_sample()
        candidate = next(c for c in report['candidates']
                          if any(a['rule'] == 'WIKI_POSITION' for a in c['arbitration']))
        candidate['arbitration'] = [a for a in candidate['arbitration'] if a['rule'] != 'WIKI_POSITION']
        errs = validate_promotion.errors(report)
        self.assertTrue(any('wiki-origin placement present without a WIKI_POSITION arbitration row' in e
                             for e in errs))

    # -- D8 WIKI_CONFIRMED: wiki-page-but-no-position NPCs, and conflict resolution ------------

    def test_wiki_confirmed_promotes_unplaced_npc_with_no_wiki_position(self):
        report = load_sample()
        for name in ('Santa Claus', 'Messenger of Santa'):
            candidate = find_candidate(report, name)
            self.assertEqual(candidate['placements'], [])
            self.assertIn({'fact': 'placements', 'rule': 'WIKI_CONFIRMED', 'chosen': 'wiki'}, candidate['arbitration'])
            self.assertIsNotNone(candidate['wiki'])
        self.assertEqual(validate_promotion.errors(report), [])

    def test_wiki_confirmed_with_a_placement_fails(self):
        report = load_sample()
        candidate = find_candidate(report, 'Santa Claus')
        candidate['placements'] = [make_placement(1, 1, 1)]
        errs = validate_promotion.errors(report)
        self.assertTrue(any("rule 'WIKI_CONFIRMED' requires an empty placements list" in e for e in errs))

    def test_empty_placements_without_wiki_confirmed_fails(self):
        report = load_sample()
        candidate = find_candidate(report, 'Santa Claus')
        candidate['arbitration'] = [a for a in candidate['arbitration'] if a['rule'] != 'WIKI_CONFIRMED']
        errs = validate_promotion.errors(report)
        self.assertTrue(any('no placements' in e for e in errs))

    def test_placement_conflict_wiki_undecided_resolved_by_wiki_position(self):
        report = load_sample()
        for name in ('A Sleeping Dragon', 'Captain Haba', 'John', 'Uzon', 'Zirella'):
            candidate = find_candidate(report, name)
            self.assertEqual(len(candidate['provenance']), 2)
            self.assertEqual(len(candidate['placements']), 1)
            self.assertEqual(candidate['placements'][0]['origin'], 'wiki')
            self.assertIn({'fact': 'placements', 'rule': 'WIKI_POSITION', 'chosen': 'wiki'}, candidate['arbitration'])
        self.assertEqual(validate_promotion.errors(report), [])

    def test_two_source_placement_conflict_wiki_undecided_synthetic(self):
        # both sources disagree with each other and with the wiki (MISMATCH); the wiki wins outright
        builder = make_builder([
            {'pageid': 1, 'revid': 1, 'title': 'Foobar', 'name': 'Foobar', 'actualname': None,
             'position': {'x': 100, 'y': 100, 'z': 7}},
        ])
        bundles = {'canary': make_bundle('Foobar', 'canary', placements=[make_placement(500, 500, 7)]),
                   'crystal': make_bundle('Foobar', 'crystal', placements=[make_placement(600, 600, 7)])}
        record = builder.candidate(bundles)
        self.assertIsNotNone(record)
        self.assertEqual(record['placements'], [{'position': {'x': 100, 'y': 100, 'z': 7}, 'direction': None,
                                                   'spawn_interval_s': None, 'spawn_radius': None, 'origin': 'wiki'}])
        self.assertIn({'fact': 'placements', 'rule': 'WIKI_POSITION', 'chosen': 'wiki'}, record['arbitration'])

    def test_two_source_placement_conflict_no_wiki_position_synthetic(self):
        # both sources disagree; the wiki page exists but has no position either -> WIKI_CONFIRMED
        builder = make_builder([
            {'pageid': 1, 'revid': 1, 'title': 'Bazqux', 'name': 'Bazqux', 'actualname': None, 'position': None},
        ])
        bundles = {'canary': make_bundle('Bazqux', 'canary', placements=[make_placement(500, 500, 7)]),
                   'crystal': make_bundle('Bazqux', 'crystal', placements=[make_placement(600, 600, 7)])}
        record = builder.candidate(bundles)
        self.assertIsNotNone(record)
        self.assertEqual(record['placements'], [])
        self.assertIn({'fact': 'placements', 'rule': 'WIKI_CONFIRMED', 'chosen': 'wiki'}, record['arbitration'])

    def test_placement_conflict_with_no_wiki_page_still_held(self):
        builder = make_builder([])
        bundles = {'canary': make_bundle('Nowhereman', 'canary', placements=[make_placement(500, 500, 7)]),
                   'crystal': make_bundle('Nowhereman', 'crystal', placements=[make_placement(600, 600, 7)])}
        result = builder.candidate(bundles)
        self.assertIsNone(result)
        self.assertEqual(builder.held[-1]['reason'], 'PLACEMENT_CONFLICT_NO_WIKI_POSITION')

    # -- Owner decision 2026-09-27: reject server-only NPCs ------------------------------------

    def test_owner_rejected_npcs_stay_held(self):
        report = load_sample()
        for name in ('Canary', 'Loot Buyer'):
            held = find_held(report, name)
            self.assertEqual(held['reason'], 'OWNER_REJECTED')
            self.assertEqual(held['detail'], 'server-only NPC, owner decision 2026-09-27')
        self.assertFalse({'Canary', 'Loot Buyer'} & {c['name'] for c in report['candidates']})

    def test_owner_rejected_bypasses_wiki_matching(self):
        # even with a matching wiki page available, the owner-rejected table wins outright
        builder = make_builder([{'pageid': 1, 'title': 'Canary', 'name': 'Canary', 'actualname': None,
                                  'position': {'x': 1, 'y': 1, 'z': 1}}])
        result = builder.candidate({'canary': make_bundle('Canary', 'canary')})
        self.assertIsNone(result)
        held = builder.held[-1]
        self.assertEqual(held['reason'], 'OWNER_REJECTED')
        self.assertEqual(held['detail'], 'server-only NPC, owner decision 2026-09-27')


    # -- D11: NPCs removed from Tibia Global -----------------------------------------------------

    def test_removed_from_game_npcs_stay_held(self):
        report = load_sample()
        names = ('Brom', 'Brutus', 'Roughington', 'Shadowpunch', 'Victor')
        for name in names:
            held = find_held(report, name)
            self.assertEqual(held['reason'], 'REMOVED_FROM_GAME')
        self.assertFalse(set(names) & {c['name'] for c in report['candidates']})

    def test_removed_from_game_bypasses_wiki_matching(self):
        builder = make_builder([{'pageid': 1, 'title': 'Brom', 'name': 'Brom', 'actualname': None,
                                  'position': {'x': 1, 'y': 1, 'z': 1}}])
        result = builder.candidate({'crystal': make_bundle('Brom', 'crystal')})
        self.assertIsNone(result)
        self.assertEqual(builder.held[-1]['reason'], 'REMOVED_FROM_GAME')


    # -- D12: a price both wikis agree on replaces the source price -----------------------------

    def price_builder(self, fandom_buy, br_prices):
        builder = promotion_candidates.Builder(
            {'npcs': [], 'trade': {'ahmet': [{'item': 'Fishing Rod', 'buy_price': fandom_buy, 'sell_price': None}]}},
            {'records': [{'source_item_id': 3483, 'native_key': 'oteryn:item.registry.i1', 'native_revision': 'r1'}]},
            {'pages': [{'title': 'Ahmet', 'name': 'Ahmet',
                        'trades': {'SellToPlayer': {'Fishing Rod': br_prices}, 'BuyFromPlayer': {}}}]})
        bundle = make_bundle('Ahmet')
        bundle['services']['trade'] = {'currency': 'GOLD', 'offers': [
            {'client_id': 3483, 'server_item_id': None, 'count': None, 'sub_type': None, 'item_name': 'fishing rod',
             'buy_price': 40, 'sell_price': None, 'stock_gate': None}]}
        arbitration = []
        offers = builder.merge_offers({'crystal': bundle}, 'Ahmet', arbitration, [])
        return offers, arbitration

    def test_wiki_price_needs_both_wikis_to_agree(self):
        for br in ([150], [150, 150]):
            offers, arbitration = self.price_builder(150, br)
            self.assertEqual(offers[0]['unit_price'], 150)
            self.assertEqual(arbitration, [{'fact': 'trade.3483.SellToPlayer', 'rule': 'WIKI_PRICE', 'chosen': 'wiki',
                                            'item_name': 'fishing rod', 'price': 150}])
        # BR must state one explicit price in every row of the offer
        for fandom, br in ((150, [120]), (150, [None]), (None, [150]), (150, [150, 120]), (150, [150, None]), (150, [])):
            offers, arbitration = self.price_builder(fandom, br)
            self.assertEqual(offers[0]['unit_price'], 40)
            self.assertEqual(arbitration, [])


if __name__ == '__main__':
    unittest.main()
