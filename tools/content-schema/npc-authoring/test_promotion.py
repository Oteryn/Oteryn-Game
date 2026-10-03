"""Unit tests for validate_promotion.py against the committed promotion candidates sample and
mutated copies of it.

Usage: python -m unittest test_promotion.py
"""
import copy
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

    def test_wiki_price_fact_names_the_exact_offer_variant(self):
        report = load_sample()
        ahmet = find_candidate(report, 'Ahmet')
        row = next(r for r in ahmet['arbitration'] if r['rule'] == 'WIKI_PRICE')
        item, direction = row['fact'].split('.')[1], row['fact'].rsplit('.', 1)[1]
        row['fact'] = f'trade.{item}x999.{direction}'  # no count-999 variant is admitted
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
        names = {'oteryn:item.test.fishing_rod': 'fishing rod'}
        def offered(price):  # the admitted offer the row names, at `price`
            built = report(150)
            built['candidates'][0]['trade_service'] = {'offers': [
                {'item': {'key': 'oteryn:item.test.fishing_rod'}, 'source_item_id': 3483,
                 'direction': 'SellToPlayer', 'unit_price': price}]}
            return built
        check = validate_promotion.wiki_price_errors
        self.assertEqual(check(offered(150), snapshot, br_facts, names), [])
        self.assertTrue(check(report(999999), snapshot, br_facts, names))
        self.assertEqual(check(report(150), b'{}', br_facts, names), ['--snapshot does not match snapshot_sha256'])
        # an omitted override: the offer keeps a source price although both wikis state another one
        omitted = offered(40)
        omitted['candidates'][0]['arbitration'] = []
        self.assertTrue(any('!= the price the wikis agree on (150)' in e for e in check(omitted, snapshot, br_facts, names)))
        # a malformed WIKI_PRICE row is reported by errors(), never a traceback here
        malformed = offered(150)
        del malformed['candidates'][0]['arbitration'][0]['fact']
        self.assertEqual(check(malformed, snapshot, br_facts, names), [])

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
        candidate = find_candidate(report, 'Anaztassja Moroia Init')
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
        candidate = find_candidate(report, 'Anaztassja Moroia Init')
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

    def price_builder(self, fandom_buy, br_prices, tibiopedia_prices=None, sub_type=None, source_name='fishing rod'):
        tibiopedia = tibiopedia_prices is not None and {'pages': [{'title': 'NPC: Ahmet', 'name': 'Ahmet', 'trades': {
            'SellToPlayer': {'Fishing Rod': tibiopedia_prices}, 'BuyFromPlayer': {}}}]}
        builder = promotion_candidates.Builder(
            {'npcs': [], 'trade': {'ahmet': [{'item': 'Fishing Rod', 'buy_price': fandom_buy, 'sell_price': None}]}},
            {'records': [{'source_item_id': 3483, 'native_key': 'oteryn:item.registry.i1', 'native_revision': 'r1'}]},
            {'pages': [{'title': 'Ahmet', 'name': 'Ahmet',
                        'trades': {'SellToPlayer': {'Fishing Rod': br_prices}, 'BuyFromPlayer': {}}}]},
            tibiopedia or None, {'oteryn:item.registry.i1': 'fishing rod'})
        bundle = make_bundle('Ahmet')
        bundle['services']['trade'] = {'currency': 'GOLD', 'offers': [
            {'client_id': 3483, 'server_item_id': None, 'count': None, 'sub_type': sub_type,
             'item_name': source_name,
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


    # -- D13: a price two of the three wikis agree on replaces the source price -------------------

    def test_majority_price_needs_two_of_three_wikis(self):
        for fandom, br, tibiopedia, wikis in ((150, [], [150], ['fandom', 'tibiopedia']),
                                              (None, [150], [150], ['br', 'tibiopedia']),
                                              (150, [120], [150, 150], ['fandom', 'tibiopedia'])):
            # the source names the item its own way; D13 looks it up under the registered name
            offers, arbitration = self.price_builder(fandom, br, tibiopedia, source_name='old fishing rod')
            self.assertEqual(offers[0]['unit_price'], 150)
            self.assertEqual(arbitration, [{'fact': 'trade.3483.SellToPlayer', 'rule': 'WIKI_MAJORITY_PRICE',
                                            'chosen': 'wiki', 'item_name': 'fishing rod', 'price': 150,
                                            'wikis': wikis}])
        # one wiki alone, three different prices, unparsed or mixed rows, or a price equal to the source's
        for fandom, br, tibiopedia in ((None, [], [150]), (150, [120], [100]), (150, [], [None]),
                                       (150, [], [150, 120]), (40, [], [40]), (None, [None], [150])):
            offers, arbitration = self.price_builder(fandom, br, tibiopedia)
            self.assertEqual(offers[0]['unit_price'], 40)
            self.assertEqual(arbitration, [])

    def test_majority_price_leaves_d12_and_fluids_alone(self):
        # Fandom and BR agree (D12) although Tibiopedia states another price: D12 decides
        offers, arbitration = self.price_builder(150, [150], [100])
        self.assertEqual((offers[0]['unit_price'], arbitration[0]['rule']), (150, 'WIKI_PRICE'))
        # with the Tibiopedia facts, Fandom and BR agreeing under the registered name (the source uses an alias)
        # is D12, so a majority never consists of Fandom and BR alone
        offers, arbitration = self.price_builder(150, [150], [], source_name='old fishing rod')
        self.assertEqual((offers[0]['unit_price'], arbitration), (150, [
            {'fact': 'trade.3483.SellToPlayer', 'rule': 'WIKI_PRICE', 'chosen': 'wiki', 'item_name': 'fishing rod',
             'price': 150}]))
        # an offer with a sub type is more than its registered Item; D13 never prices it
        offers, arbitration = self.price_builder(150, [], [150], sub_type=2)
        self.assertEqual((offers[0]['unit_price'], arbitration), (40, []))

    def majority_report(self, price, wikis=('fandom', 'tibiopedia')):
        report = load_sample()
        report['tibiopedia_facts_sha256'] = 'a' * 64
        ahmet = find_candidate(report, 'Ahmet')
        offer = next(o for o in ahmet['trade_service']['offers'] if o['count'] is None and o['sub_type'] is None)
        offer['unit_price'] = price
        ahmet['arbitration'].append({'fact': f"trade.{offer['source_item_id']}.{offer['direction']}",
                                     'rule': 'WIKI_MAJORITY_PRICE', 'chosen': 'wiki', 'price': 150,
                                     'item_name': 'x', 'wikis': list(wikis)})
        return report

    def test_majority_price_row_shape(self):
        self.assertEqual(validate_promotion.errors(self.majority_report(150)), [])
        self.assertTrue(any('!= WIKI_MAJORITY_PRICE price' in e for e in validate_promotion.errors(self.majority_report(10))))
        for wikis in (['fandom'], ['br', 'fandom'], ['tibiopedia', 'fandom'], ['fandom', 'fandom', 'tibiopedia'],
                      ['fandom', 'wiki'], 'fandom'):
            self.assertTrue(any('are not 2-3 sorted wikis' in e
                                for e in validate_promotion.errors(self.majority_report(150, wikis))), wikis)
        # BR and Tibiopedia need no Fandom page; a majority Fandom is part of does
        report = self.majority_report(150, ('br', 'tibiopedia'))
        ahmet = find_candidate(report, 'Ahmet')
        ahmet['wiki'], ahmet['arbitration'] = None, ahmet['arbitration'][-1:]  # only the row under test
        self.assertEqual([e for e in validate_promotion.errors(report) if 'requires a wiki page' in e], [])
        report = self.majority_report(150)
        ahmet = find_candidate(report, 'Ahmet')
        ahmet['wiki'], ahmet['arbitration'] = None, ahmet['arbitration'][-1:]
        self.assertTrue(any("'WIKI_MAJORITY_PRICE' requires a wiki page" in e for e in validate_promotion.errors(report)))
        report = self.majority_report(150)
        del report['tibiopedia_facts_sha256']
        report['decisions'] = validate_promotion.DECISIONS + ['D12']
        self.assertIn('WIKI_MAJORITY_PRICE arbitration without tibiopedia_facts_sha256 (D13)',
                      validate_promotion.errors(report))
        report = self.majority_report(150)
        report['decisions'] = validate_promotion.DECISIONS + ['D12']
        self.assertTrue(any(e.startswith('decisions ') for e in validate_promotion.errors(report)))

    def test_majority_price_matches_the_pinned_wikis(self):
        snapshot = json.dumps({'npcs': [], 'trade': {'ahmet': [
            {'item': 'Fishing Rod', 'buy_price': 150, 'sell_price': None}]}}).encode()
        br_facts = json.dumps({'pages': [{'title': 'Ahmet', 'name': 'Ahmet', 'trades': {
            'SellToPlayer': {'Fishing Rod': [120]}, 'BuyFromPlayer': {}}}]}).encode()
        tibiopedia = json.dumps({'pages': [{'title': 'NPC: Ahmet', 'name': 'Ahmet', 'trades': {
            'SellToPlayer': {'Fishing Rod': [150]}, 'BuyFromPlayer': {}}}]}).encode()
        names = {'oteryn:item.test.fishing_rod': 'fishing rod'}
        def report(price, rows):
            return {'snapshot_sha256': hashlib.sha256(snapshot).hexdigest(),
                    'br_facts_sha256': hashlib.sha256(br_facts).hexdigest(),
                    'tibiopedia_facts_sha256': hashlib.sha256(tibiopedia).hexdigest(),
                    'candidates': [{'name': 'Ahmet', 'arbitration': rows, 'trade_service': {'offers': [
                        {'item': {'key': 'oteryn:item.test.fishing_rod'}, 'source_item_id': 3483, 'count': None,
                         'sub_type': None, 'direction': 'SellToPlayer', 'unit_price': price}]}}]}
        row = {'fact': 'trade.3483.SellToPlayer', 'rule': 'WIKI_MAJORITY_PRICE', 'chosen': 'wiki',
               'item_name': 'fishing rod', 'price': 150, 'wikis': ['fandom', 'tibiopedia']}
        check = validate_promotion.wiki_price_errors
        self.assertEqual(check(report(150, [row]), snapshot, br_facts, names, tibiopedia), [])
        self.assertTrue(check(report(150, [{**row, 'wikis': ['br', 'tibiopedia']}]), snapshot, br_facts, names, tibiopedia))
        self.assertTrue(check(report(150, [{**row, 'rule': 'WIKI_PRICE'}]), snapshot, br_facts, names, tibiopedia))
        # an omitted override: the offer keeps the source price although two wikis state another one
        self.assertTrue(any('!= the price the wikis agree on (150)' in e
                            for e in check(report(40, []), snapshot, br_facts, names, tibiopedia)))
        self.assertEqual(check(report(150, [row]), snapshot, br_facts, names, b'{}'),
                         ['--tibiopedia-facts does not match tibiopedia_facts_sha256'])


    # -- D13 offers: offers two of three wikis agree on that the sources lack --------------------------

    def offer_builder(self, fandom_rope, tibiopedia_rope, name='Ahmet', names=None, left_out=()):
        builder = promotion_candidates.Builder(
            {'npcs': [], 'trade': {name.lower(): [{'item': 'Rope', 'buy_price': fandom_rope, 'sell_price': None}]}},
            {'records': [{'source_item_id': 3483, 'native_key': 'oteryn:item.registry.i1', 'native_revision': 'r1'},
                         {'source_item_id': 3003, 'native_key': 'oteryn:item.registry.i2', 'native_revision': 'r1'}]},
            {'pages': []},
            {'pages': [{'title': f'NPC: {name}', 'name': name, 'trades': {
                'SellToPlayer': {'Rope': tibiopedia_rope}, 'BuyFromPlayer': {}}}]},
            names or {'oteryn:item.registry.i1': 'fishing rod', 'oteryn:item.registry.i2': 'rope'})
        have = [{'item': {'key': 'oteryn:item.registry.i1'}, 'direction': 'SellToPlayer'}]
        arbitration = []
        return builder.wiki_offers(name, have, list(left_out), arbitration), arbitration

    def test_wiki_offer_needs_two_wikis_and_one_registered_item(self):
        offers, rows = self.offer_builder(50, [50])
        self.assertEqual(offers, [{'item': {'family': 'Item', 'key': 'oteryn:item.registry.i2', 'revision': 'r1'},
                                   'source_item_id': 3003, 'direction': 'SellToPlayer', 'unit_price': 50,
                                   'count': None, 'sub_type': None, 'origin': 'wiki'}])
        self.assertEqual(rows, [{'fact': 'trade.3003.SellToPlayer', 'rule': 'WIKI_OFFER', 'chosen': 'wiki',
                                 'item_name': 'rope', 'price': 50, 'wikis': ['fandom', 'tibiopedia']}])
        for fandom, tibiopedia in ((50, [60]), (None, [50]), (50, [])):
            self.assertEqual(self.offer_builder(fandom, tibiopedia), ([], []))
        # two registered Items with the name "rope": the wiki name settles neither
        ambiguous = {'oteryn:item.registry.i1': 'rope', 'oteryn:item.registry.i2': 'rope'}
        self.assertEqual(self.offer_builder(50, [50], names=ambiguous), ([], []))
        # the sources offer the Item in a left-out offer (gated, unconfirmed, conflicting): the wikis do not add it
        for reason in ('GATED_OFFER', 'OFFER_UNCONFIRMED', 'OFFER_CONFLICT_WIKI_UNDECIDED'):
            self.assertEqual(self.offer_builder(50, [50], left_out=[{'fact': 'trade.3003x2', 'reason': reason}]),
                             ([], []))

    def test_wiki_offer_rows(self):
        report = self.majority_report(150)
        ahmet = find_candidate(report, 'Ahmet')
        row = ahmet['arbitration'].pop()
        ahmet['arbitration'].append({**row, 'rule': 'WIKI_OFFER', 'wikis': ['br', 'fandom']})
        # the row without a wiki-origin offer, then with it
        self.assertTrue(any('do not match the WIKI_OFFER rows' in e for e in validate_promotion.errors(report)))
        source_item_id, direction = row['fact'].split('.')[1], row['fact'].split('.')[2]
        offer = next(o for o in ahmet['trade_service']['offers']
                     if o['source_item_id'] == int(source_item_id) and o['direction'] == direction)
        offer['origin'] = 'wiki'
        self.assertEqual(validate_promotion.errors(report), [])
        # a wiki-origin offer whose provenance row is dropped
        ahmet['arbitration'].pop()
        self.assertTrue(any('do not match the WIKI_OFFER rows' in e for e in validate_promotion.errors(report)))
        ahmet['arbitration'].append({**row, 'rule': 'WIKI_OFFER', 'wikis': ['br', 'fandom']})
        ahmet['arbitration'][-1]['wikis'] = ['fandom']
        self.assertTrue(any('are not 2-3 sorted wikis' in e for e in validate_promotion.errors(report)))
        ahmet['arbitration'][-1]['wikis'] = ['br', 'fandom']
        del report['tibiopedia_facts_sha256']
        report['decisions'] = validate_promotion.DECISIONS + ['D12']
        self.assertIn('WIKI_OFFER arbitration without tibiopedia_facts_sha256 (D13)', validate_promotion.errors(report))

    def test_wiki_offer_matches_the_pinned_wikis(self):
        snapshot = json.dumps({'npcs': [], 'trade': {'ahmet': [
            {'item': 'Rope', 'buy_price': 50, 'sell_price': None}]}}).encode()
        br_facts = json.dumps({'pages': []}).encode()
        tibiopedia = json.dumps({'pages': [{'title': 'NPC: Ahmet', 'name': 'Ahmet', 'trades': {
            'SellToPlayer': {'Rope': [50]}, 'BuyFromPlayer': {}}}]}).encode()
        item_map = json.dumps({'records': [{'source_item_id': 3003, 'native_key': 'oteryn:item.test.rope',
                                            'native_revision': 'definition-r1'}]}).encode()
        names = {'oteryn:item.test.rope': 'rope'}
        row = {'fact': 'trade.3003.SellToPlayer', 'rule': 'WIKI_OFFER', 'chosen': 'wiki', 'item_name': 'rope',
               'price': 50, 'wikis': ['fandom', 'tibiopedia']}
        def report(rows, offers):
            return {'snapshot_sha256': hashlib.sha256(snapshot).hexdigest(),
                    'br_facts_sha256': hashlib.sha256(br_facts).hexdigest(),
                    'tibiopedia_facts_sha256': hashlib.sha256(tibiopedia).hexdigest(),
                    'item_map_sha256': hashlib.sha256(item_map).hexdigest(),
                    'candidates': [{'name': 'Ahmet', 'arbitration': rows, 'left_out': [],
                                    'trade_service': {'currency': None, 'offers': offers} if offers else None}]}
        rope = {'item': {'key': 'oteryn:item.test.rope'}, 'source_item_id': 3003, 'count': None, 'sub_type': None,
                'direction': 'SellToPlayer', 'unit_price': 50, 'origin': 'wiki'}
        check = validate_promotion.wiki_price_errors
        self.assertEqual(check(report([row], [rope]), snapshot, br_facts, names, tibiopedia, item_map), [])
        # an omitted wiki offer, and an invented one
        self.assertTrue(any('WIKI_OFFER rows differ' in e for e in check(report([], []), snapshot, br_facts, names,
                                                                          tibiopedia, item_map)))
        self.assertTrue(any('WIKI_OFFER rows differ' in e for e in check(
            report([row, {**row, 'fact': 'trade.3003.BuyFromPlayer'}], [rope]), snapshot, br_facts, names,
            tibiopedia, item_map)))
        self.assertEqual(check(report([row], [rope]), snapshot, br_facts, names, tibiopedia, b'{}'),
                         ['--item-map does not match item_map_sha256'])


    def test_wiki_offers_never_at_held_or_non_gold_shops(self):
        report = self.majority_report(150)
        ahmet = find_candidate(report, 'Ahmet')
        row = ahmet['arbitration'].pop()
        ahmet['arbitration'].append({**row, 'rule': 'WIKI_OFFER', 'wikis': ['br', 'fandom']})
        source_item_id, direction = row['fact'].split('.')[1], row['fact'].split('.')[2]
        next(o for o in ahmet['trade_service']['offers']
             if o['source_item_id'] == int(source_item_id) and o['direction'] == direction)['origin'] = 'wiki'
        self.assertEqual(validate_promotion.errors(report), [])
        ahmet['name'] = 'Cillia'
        self.assertTrue(any('wiki offers at a held or non-gold shop' in e for e in validate_promotion.errors(report)))
        ahmet['name'] = 'Ahmet'
        ahmet['trade_service']['currency'] = {'family': 'Item', 'key': 'oteryn:item.test.token', 'revision': 'definition-r1'}
        self.assertTrue(any('wiki offers at a held or non-gold shop' in e for e in validate_promotion.errors(report)))

    def test_rebuild_compares_the_whole_report(self):
        from unittest import mock
        report = load_sample()
        with mock.patch.object(promotion_candidates, 'build_report', return_value=load_sample()):
            self.assertEqual(validate_promotion.rebuild_errors(report, 'c', 'x', b'', b'', b'', b''), [])
            # an offer stripped of both its origin and its WIKI_OFFER row is still caught against the rebuild
            ahmet = find_candidate(report, 'Ahmet')
            ahmet['arbitration'] = [r for r in ahmet['arbitration'] if r['rule'] != 'WIKI_OFFER']
            for offer in ahmet['trade_service']['offers']:
                offer.pop('origin', None)
            self.assertEqual(validate_promotion.rebuild_errors(report, 'c', 'x', b'', b'', b'', b''),
                             ["the report is not what the pinned inputs build (differing candidates: ['Ahmet'])"])


    # -- D14: the Crystal supplement revision ----------------------------------------------------

    def supplement_report(self, key='crystal:npc/thorim', revision=promotion_candidates.CRYSTAL_SUPPLEMENT_REVISION):
        report = load_sample()
        report['crystal_supplement'] = {'revision': promotion_candidates.CRYSTAL_SUPPLEMENT_REVISION,
                                        'bundles_sha256': 'b' * 64}
        candidate = next(c for c in report['candidates'] if set(c['provenance']) == {'crystal'})
        candidate['provenance']['crystal'].update(key=key, revision=revision)
        return report

    def test_supplement_revision_only_on_listed_files(self):
        self.assertEqual([e for e in validate_promotion.errors(self.supplement_report()) if 'revision' in e], [])
        for report in (self.supplement_report('crystal:npc/omar'), self.supplement_report(revision='0' * 40)):
            self.assertTrue(any('not the D14 supplement revision' in e for e in validate_promotion.errors(report)))
        report = self.supplement_report()
        del report['crystal_supplement']
        report['decisions'] = report['decisions'][:-1]
        self.assertIn('a provenance revision without crystal_supplement (D14)', validate_promotion.errors(report))

    def test_supplement_lists_are_disjoint(self):
        self.assertEqual(len(promotion_candidates.SUPPLEMENT_ADMITTED), 7)
        self.assertEqual(len(promotion_candidates.SUPPLEMENT_HELD), 13)
        self.assertFalse(promotion_candidates.SUPPLEMENT_ADMITTED & set(promotion_candidates.SUPPLEMENT_HELD))


    # -- D15: Tibiopedia or BR confirms a single-source NPC; a disputed price stays, pending ---------

    def fan_builder(self, tibiopedia_names=('Somebody Else',), br_pages=()):
        empty = {'SellToPlayer': {}, 'BuyFromPlayer': {}}
        tibiopedia = {'pages': [{'title': f'NPC: {name}', 'name': name, 'url': f'https://tibiopedia.pl/npcs/{name}',
                                 'sha256': 'a' * 64, 'trades': empty} for name in tibiopedia_names]}
        br = {'pages': [{'title': name, 'name': name, 'pageid': 7, 'revid': 8, 'removed': removed, 'trades': empty}
                        for name, removed in br_pages]}
        return promotion_candidates.Builder({'npcs': [], 'trade': {}}, {'records': []}, br, tibiopedia, {})

    def test_fan_wiki_confirms_a_single_source_npc(self):
        placed = {'crystal': make_bundle('Weary Lion Knight', placements=[make_placement(100, 100, 7)])}
        record = self.fan_builder(['Weary Lion Knight'], [('Weary Lion Knight', '')]).candidate(placed)
        self.assertIn({'fact': 'identity', 'rule': 'FAN_WIKI_CONFIRMED', 'chosen': 'br', 'wikis': ['br', 'tibiopedia'],
                       'pages': {'br': {'pageid': 7, 'revid': 8}, 'tibiopedia': {
                           'url': 'https://tibiopedia.pl/npcs/Weary Lion Knight', 'sha256': 'a' * 64}}},
                      record['arbitration'])
        self.assertIsNone(record['wiki'])
        # with no source placement it is admitted with no placements, as D8 does for Fandom
        record = self.fan_builder(['Weary Lion Knight']).candidate({'crystal': make_bundle('Weary Lion Knight')})
        self.assertEqual((record['placements'], record['arbitration'][0]['wikis']), ([], ['tibiopedia']))
        # a BR page recording the NPC as removed confirms nothing; no page at all leaves it held
        for builder in (self.fan_builder(br_pages=[('Weary Lion Knight', '15.20')]), self.fan_builder()):
            self.assertIsNone(builder.candidate(placed))
            self.assertEqual(builder.held[-1]['reason'], 'SINGLE_SOURCE_NOT_ON_WIKI')

    def test_fan_wiki_confirmed_row_shape(self):
        report = load_sample()
        candidate = find_candidate(report, 'Weary Lion Knight')
        row = next(r for r in candidate['arbitration'] if r['rule'] == 'FAN_WIKI_CONFIRMED')
        self.assertEqual(validate_promotion.errors(report), [])
        for field, value in (('wikis', ['fandom']), ('chosen', 'br'), ('pages', {}), ('fact', 'placements')):
            broken = copy.deepcopy(report)
            bad = next(r for r in find_candidate(broken, 'Weary Lion Knight')['arbitration'] if r == row)
            bad[field] = value
            self.assertTrue(validate_promotion.errors(broken), field)
        broken = copy.deepcopy(report)
        find_candidate(broken, 'Weary Lion Knight')['wiki'] = {'pageid': 1, 'revid': 1}
        self.assertTrue(validate_promotion.errors(broken))

    def test_disputed_price_stays_pending(self):
        # no two wikis agree and one states another price: the source price stays, with the stated prices
        for fandom, br, tibiopedia, pending in ((10, [], [40], {'fandom': 10, 'tibiopedia': 40}),
                                                (None, [], [30], {'tibiopedia': 30}),
                                                (150, [120], [100], {'br': 120, 'fandom': 150, 'tibiopedia': 100})):
            offers, arbitration = self.price_builder(fandom, br, tibiopedia)
            self.assertEqual((offers[0]['unit_price'], offers[0].get('parity_pending'), arbitration), (40, pending, []))
        # a majority settles the price, every wiki agrees with the source, no wiki speaks, or the offer is a fluid
        for fandom, br, tibiopedia, sub_type in ((150, [], [150], None), (40, [], [40], None),
                                                 (None, [], [], None), (10, [], [30], 2)):
            offers, _ = self.price_builder(fandom, br, tibiopedia, sub_type=sub_type)
            self.assertNotIn('parity_pending', offers[0])

    def test_parity_pending_shape(self):
        report = load_sample()
        offer = next(o for c in report['candidates'] for o in ((c.get('trade_service') or {}).get('offers') or [])
                     if 'parity_pending' in o)
        for value in ({}, {'fandom': offer['unit_price']}, {'wiki': 1}, {'fandom': 1, 'tibiopedia': 1}, True):
            broken = copy.deepcopy(report)
            next(o for c in broken['candidates'] for o in ((c.get('trade_service') or {}).get('offers') or [])
                 if o == offer)['parity_pending'] = value
            self.assertTrue(validate_promotion.errors(broken), value)

    def test_wiki_offer_names_count_pinned_item_map_keys_only(self):
        # since ITEM-ID-1b a retired key and the Tibia id definition it aliases to share one name
        builder = promotion_candidates.Builder(
            {'npcs': [], 'trade': {}},
            {'records': [{'source_item_id': 3003, 'native_key': 'oteryn:item.registry.i1', 'native_revision': 'r1'}]},
            None, None, {'oteryn:item.registry.i1': 'rope', 'oteryn:item.tibia.i3003': 'rope'})
        self.assertEqual(builder.registry_keys, {'rope': 'oteryn:item.registry.i1'})

    # -- D16: wiki majority confirms an offer; reviewed definitions; unconfirmed Crystal facts -----

    def test_majority_arbiter_confirms_a_one_sided_offer(self):
        # Fandom and Tibiopedia list the Crystal-only offer's buy price; Tibiopedia alone its sell price
        builder = promotion_candidates.Builder(
            {'npcs': [], 'trade': {'ahmet': [{'item': 'Fishing Rod', 'buy_price': 150, 'sell_price': None}]}},
            {'records': [{'source_item_id': 3483, 'native_key': 'oteryn:item.registry.i1', 'native_revision': 'r1'}]},
            {'pages': []}, {'pages': [{'title': 'NPC: Ahmet', 'name': 'Ahmet', 'trades': {
                'SellToPlayer': {'Fishing Rod': [150]}, 'BuyFromPlayer': {'Fishing Rod': [40]}}}]},
            {'oteryn:item.registry.i1': 'fishing rod'})
        offer = {'client_id': 3483, 'server_item_id': None, 'count': None, 'sub_type': None,
                 'item_name': 'fishing rod', 'buy_price': 150, 'sell_price': 40, 'stock_gate': None}
        present = {'crystal': offer}
        self.assertEqual(builder.majority_arbiter('Ahmet', (3483, None, None), present, {
            'fishing rod': {'item': 'Fishing Rod', 'buy_price': 150, 'sell_price': None}}),
            ('crystal', ['fandom', 'tibiopedia'], [
                {'direction': 'SellToPlayer', 'price': 150, 'status': 'CONFIRMED', 'wikis': ['fandom', 'tibiopedia']},
                {'direction': 'BuyFromPlayer', 'price': 40, 'status': 'UNCONFIRMED', 'stated_by': 'crystal'}]))
        # a price the majority contradicts, or a gated offer, confirms nothing
        for changed in ({'buy_price': 120}, {'stock_gate': {'storage': 1}}):
            self.assertIsNone(builder.majority_arbiter('Ahmet', (3483, None, None), {'crystal': {**offer, **changed}}, {
                'fishing rod': {'item': 'Fishing Rod', 'buy_price': 150, 'sell_price': None}}))

    def test_arbiter_rows_record_each_direction(self):
        # owner 1c: the whole offer is admitted; each direction says whether the wiki majority confirms it
        report = load_sample()
        rows = [row for candidate in report['candidates'] for row in candidate.get('arbitration') or []
                if row.get('rule') == 'WIKI_MAJORITY_ARBITER']
        self.assertTrue(rows)
        self.assertTrue(all(any(d['status'] == 'CONFIRMED' for d in row['directions']) for row in rows))
        self.assertEqual(validate_promotion.arbiter_direction_errors('x', rows[0], rows[0]['chosen']), [])
        broken = copy.deepcopy(rows[0])
        broken['directions'][0]['status'] = 'UNCONFIRMED'
        self.assertTrue(validate_promotion.arbiter_direction_errors('x', broken, broken['chosen']))

    def test_arbiter_directions_match_the_pinned_wikis(self):
        # owner 1c: the validator re-derives each direction's status from the pinned wikis, not only its shape
        def pinned(fandom_sell):
            snapshot = json.dumps({'npcs': [], 'trade': {'ahmet': [
                {'item': 'Fishing Rod', 'buy_price': 150, 'sell_price': fandom_sell}]}}).encode()
            tibiopedia = json.dumps({'pages': [{'title': 'NPC: Ahmet', 'name': 'Ahmet', 'trades': {
                'SellToPlayer': {'Fishing Rod': [150]}, 'BuyFromPlayer': {'Fishing Rod': [40]}}}]}).encode()
            return snapshot, json.dumps({'pages': []}).encode(), tibiopedia
        names = {'oteryn:item.test.fishing_rod': 'fishing rod'}
        confirmed = {'direction': 'SellToPlayer', 'price': 150, 'status': 'CONFIRMED', 'wikis': ['fandom', 'tibiopedia']}
        unconfirmed = {'direction': 'BuyFromPlayer', 'price': 40, 'status': 'UNCONFIRMED', 'stated_by': 'crystal'}
        def check(directions, fandom_sell=None):
            snapshot, br_facts, tibiopedia = pinned(fandom_sell)
            row = {'fact': 'trade.3483', 'rule': 'WIKI_MAJORITY_ARBITER', 'chosen': 'crystal',
                   'wikis': ['fandom', 'tibiopedia'], 'item_name': 'fishing rod', 'directions': directions}
            offers = [{'item': {'key': 'oteryn:item.test.fishing_rod'}, 'source_item_id': 3483, 'count': None,
                       'sub_type': None, 'direction': d, 'unit_price': p}
                      for d, p in (('SellToPlayer', 150), ('BuyFromPlayer', 40))]
            report = {'snapshot_sha256': hashlib.sha256(snapshot).hexdigest(),
                      'br_facts_sha256': hashlib.sha256(br_facts).hexdigest(),
                      'tibiopedia_facts_sha256': hashlib.sha256(tibiopedia).hexdigest(),
                      'candidates': [{'name': 'Ahmet', 'arbitration': [row], 'trade_service': {'offers': offers}}]}
            return [e for e in validate_promotion.wiki_price_errors(report, snapshot, br_facts, names, tibiopedia)
                    if 'WIKI_MAJORITY_ARBITER' in e]
        self.assertEqual(check([confirmed, unconfirmed]), [])
        # a flipped status in either direction, or confirming wikis that differ from the pinned ones
        self.assertTrue(check([{**confirmed, 'status': 'UNCONFIRMED'}, unconfirmed]))
        self.assertTrue(check([confirmed, {**unconfirmed, 'status': 'CONFIRMED', 'wikis': ['br', 'tibiopedia']}]))
        self.assertTrue(check([{**confirmed, 'wikis': ['br', 'fandom']}, unconfirmed]))
        # a later snapshot where two wikis agree on the sell price: only that direction's status changes
        self.assertTrue(check([confirmed, unconfirmed], fandom_sell=40))
        self.assertEqual(check([confirmed, {'direction': 'BuyFromPlayer', 'price': 40, 'status': 'CONFIRMED',
                                            'wikis': ['fandom', 'tibiopedia']}], fandom_sell=40), [])

    def test_arbiter_rows_cover_every_direction_of_the_offer(self):
        # #1358 4149546495: a row lists each admitted direction and price of the offer its fact names
        report = load_sample()
        self.assertEqual(validate_promotion.errors(report), [])
        baltim = find_candidate(report, 'Baltim')
        row = next(r for r in baltim['arbitration'] if r['rule'] == 'WIKI_MAJORITY_ARBITER'
                   and any(d['direction'] == 'BuyFromPlayer' and d['status'] == 'UNCONFIRMED' for d in r['directions']))
        broken = copy.deepcopy(report)
        broken_row = next(r for r in find_candidate(broken, 'Baltim')['arbitration'] if r == row)
        broken_row['directions'] = [d for d in broken_row['directions'] if d['direction'] != 'BuyFromPlayer']
        self.assertTrue(any('directions' in e and 'offer' in e for e in validate_promotion.errors(broken)))
        broken = copy.deepcopy(report)
        broken_row = next(r for r in find_candidate(broken, 'Baltim')['arbitration'] if r == row)
        broken_row['directions'][0]['price'] += 1
        self.assertTrue(validate_promotion.errors(broken))

    def test_owner_review_rows_keep_the_selected_value(self):
        # #1358 4149546503: without the rebuild inputs, the reviewed field still equals the chosen source's value
        report = load_sample()
        for name, path in (('Storkus', ('presentation', 'outfit', 'look_type')),
                           ('Flickering Soul', ('movement', 'walk_radius')),
                           ('Grumpy Stone', ('presentation', 'outfit', 'item_look'))):
            broken = copy.deepcopy(report)
            target = find_candidate(broken, name)
            for key in path[:-1]:
                target = target[key]
            target[path[-1]] += 1
            self.assertTrue(any('reviewed' in e and 'value' in e for e in validate_promotion.errors(broken)), name)

    def test_image_fit_scores_are_at_most_35(self):
        self.assertTrue(all(fit['score'] <= validate_promotion.WIKI_IMAGE_FIT_MAX_SCORE
                            for fit in promotion_candidates.WIKI_IMAGE_FIT.values()))
        self.assertEqual(promotion_candidates.DEFINITION_REVIEWED['Enpa-Deia Pema']['outfit']['rule'], 'OWNER_REVIEW')

    def test_wiki_item_alias(self):
        self.assertEqual(promotion_candidates.wiki_item('Straw Mat Foot Section'), 'straw bed foot section')
        self.assertEqual(promotion_candidates.wiki_item('Straw Mat Head Section'), 'straw mat head section')

    def test_reviewed_definitions_and_fits_are_in_the_sample(self):
        report = load_sample()
        manop = find_candidate(report, 'Ambassador Manop')
        self.assertEqual({k: manop['presentation']['outfit'][k] for k in ('head', 'body', 'legs', 'feet')},
                         {'head': 2, 'body': 10, 'legs': 22, 'feet': 81})
        self.assertEqual(manop['movement']['walk_interval_ms'], 2000)
        self.assertEqual(find_held(report, 'Testserver Assistant')['reason'], 'DEFINITION_CONFLICT')
        thug = find_candidate(report, 'Raubritter Thug')
        self.assertEqual(thug['source_unconfirmed'], ['text'])
        self.assertTrue(any(row['rule'] == 'WIKI_IMAGE_FIT' for row in thug['arbitration']))
        broken = copy.deepcopy(report)
        find_candidate(broken, 'Raubritter Thug')['presentation']['outfit']['body'] += 1
        self.assertTrue(validate_promotion.errors(broken))
        broken = copy.deepcopy(report)
        find_candidate(broken, 'Brewmaster Bhaan')['source_unconfirmed'] = ['walk']
        self.assertTrue(validate_promotion.errors(broken))

if __name__ == '__main__':
    unittest.main()
