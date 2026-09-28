"""Validate an OTERYN_NPC_PROMOTION_CANDIDATES/v1 report (promotion_candidates.py output) against the
schema doc (OTERYN_NPC_AUTHORING_SCHEMA_V1.md §3 D4-D8, §8).

Semantic rules:
- schema == 'OTERYN_NPC_PROMOTION_CANDIDATES/v1', evidence == 'OTS_HYPOTHESIS_ONLY',
  decisions == ['D4', 'D5', 'D6', 'D7', 'D8', 'D11'], plus 'D12' exactly when br_facts_sha256 is
  present and 'D13' exactly when tibiopedia_facts_sha256 is present (only with br_facts_sha256); a
  WIKI_PRICE row requires br_facts_sha256 and a WIKI_MAJORITY_PRICE row tibiopedia_facts_sha256;
  snapshot_sha256, item_map_sha256, br_facts_sha256 and tibiopedia_facts_sha256 are 64 hex chars;
- each candidate identity is family NPC, key `oteryn:npc.<slug>` (D4) and revision 'definition-r1';
  the key suffix equals the slug of `name` (same slug() as promotion_candidates.py);
- candidate keys are unique and the candidates list is sorted by key;
- travel_service is null, or identity family Service, key `oteryn:service.travel.<same slug>`,
  revision 'definition-r1', with >=1 route; every route has an in-range destination
  ({x,y,z}, x/y 0..65535, z 0..15), an integer price >= 0, and a destination_keyword unique within
  the candidate;
- trade_service is null, or identity family Service, key `oteryn:service.trade.<same slug>`,
  revision 'definition-r1' (O5), with >=1 offer; currency is null or an Item ref (family Item,
  key matching `^oteryn:item\\.[a-z0-9_.]+$`, revision 'definition-r1'); every offer has an Item ref
  (same key pattern), source_item_id (int > 0), direction in SellToPlayer/BuyFromPlayer, unit_price
  (int >= 0), count (int >= 1 or null) and sub_type (int or null); the tuple
  (source_item_id, count, sub_type, direction) is unique within a candidate; a source_item_id maps
  to exactly one item key across the whole report (cross-candidate consistency);
- every candidate has >=1 placement, with an in-range position, a compass direction
  (NORTH/EAST/SOUTH/WEST) and spawn_interval_s in 1..86400; a D8 wiki-origin placement
  (`origin: 'wiki'`) instead has direction and spawn_interval_s both null (unknown from the wiki),
  and `origin`, where present, is null or 'wiki'; an empty `placements` list is allowed only when
  a D8 WIKI_CONFIRMED arbitration row is present (a wiki-confirmed NPC with no wiki position);
- provenance has 1 or 2 of canary/crystal, each with a key namespaced to that source
  (`<source>:npc/...`) and a 64-hex sha256; a single-source candidate must carry a non-null `wiki`
  confirmation (D6: single-source NPCs need wiki confirmation);
- arbitration rows have rule 'WIKI_ARBITER' with `chosen` in canary/crystal and one of the
  candidate's provenance sources (a fact may be `travel.<id>...` or `trade.<id>...`); or (D8)
  rule 'WIKI_POSITION' with `chosen` == 'wiki', `fact` == 'placements' and exactly one placement
  with `origin` == 'wiki' (and, conversely, any such placement requires this row); or rule
  'WIKI_CONFIRMED' with `chosen` == 'wiki', `fact` == 'placements', an empty `placements` list
  (and, conversely, an empty `placements` list requires either this row or the candidate is
  otherwise invalid) and a non-null `wiki`; or rule 'WIKI_PRICE' (D12) with `chosen` == 'wiki', an
  `item_name`, a `price` equal to the unit_price of the admitted offer its fact names (with `--snapshot`
  and `--br-facts`, also the price both pinned wikis state; every admitted offer whose registered item
  both wikis price must then carry that price) and an `item_name` that is the registered
  name of that offer's Item (committed `content/items/definitions`), a
  `trade.<item>.<direction>` fact and a non-null `wiki`; or rule 'WIKI_MAJORITY_PRICE' (D13), checked
  the same way, whose `wikis` are two or three distinct names from fandom/br/tibiopedia, sorted and
  including tibiopedia, and a non-null `wiki` only when fandom is among them (with `--tibiopedia-facts` as
  well, its price is the price those wikis state, and every admitted offer two wikis price the same carries
  that price); or rule 'WIKI_BASE_NAME'/'WIKI_SPELLING' with
  `chosen` == 'wiki', `fact` == 'identity', a single-source candidate and a non-null `wiki`;
- left_out rows have reason in GATED_ROUTE / ROUTE_CONFLICT_WIKI_UNDECIDED / ROUTE_UNCONFIRMED /
  GATED_OFFER / OFFER_UNCONFIRMED / OFFER_CONFLICT_WIKI_UNDECIDED / ITEM_NOT_REGISTERED /
  CURRENCY_CONFLICT; trade facts start with `trade.`, route facts with `travel.`; no left_out
  travel keyword also appears among the candidate's promoted routes;
- no text anywhere (D5): no 'text', 'description', 'voices' or 'dialogue' key at any depth of a
  candidate;
- totals.candidates == len(candidates), totals.with_travel == the number of candidates with a
  non-null travel_service, totals.with_trade == the number of candidates with a non-null
  trade_service, and totals.offers == the total number of offer rows across all candidates;
- held rows carry a non-empty reason and name.

Usage: python validate_promotion.py <report.json>
"""
import argparse
import hashlib
import json
import re
import sys
from pathlib import Path

import promotion_candidates
from promotion_candidates import slug

SCHEMA = 'OTERYN_NPC_PROMOTION_CANDIDATES/v1'
EVIDENCE = 'OTS_HYPOTHESIS_ONLY'
DECISIONS = ['D4', 'D5', 'D6', 'D7', 'D8', 'D11']
# `trade.<source item id>[x<count>][s<sub type>].<direction>`, as promotion_candidates labels an offer
PRICE_RULES = ('WIKI_PRICE', 'WIKI_MAJORITY_PRICE')  # D12, D13
ITEM_RULES = PRICE_RULES + ('WIKI_OFFER',)  # rows that name an offer's registered Item
WIKIS = {'fandom', 'br', 'tibiopedia'}
WIKI_PRICE_FACT = re.compile(r'trade\.(\d+)(?:x(\d+))?(?:s(-?\d+))?\.(SellToPlayer|BuyFromPlayer)')


def named_offers(candidate, match):
    """The admitted offers a WIKI_PRICE fact names: the exact (item, count, sub type, direction) tuple."""
    source_item_id, count, sub_type, direction = match.groups()
    return [offer for offer in (candidate.get('trade_service') or {}).get('offers') or []
            if offer.get('source_item_id') == int(source_item_id) and offer.get('direction') == direction
            and (offer.get('count') or None) == (int(count) if count else None)
            and offer.get('sub_type') == (int(sub_type) if sub_type is not None else None)]
KEY_RE = re.compile(r'^oteryn:npc\.[a-z0-9]+(_[a-z0-9]+)*$')
ITEM_KEY_RE = re.compile(r'^oteryn:item\.[a-z0-9_.]+$')
SHA256_RE = re.compile(r'^[0-9a-f]{64}$')
DIRECTIONS = {'NORTH', 'EAST', 'SOUTH', 'WEST'}
TRADE_DIRECTIONS = {'SellToPlayer', 'BuyFromPlayer'}
LEFT_OUT_REASONS = {'GATED_ROUTE', 'ROUTE_CONFLICT_WIKI_UNDECIDED', 'ROUTE_UNCONFIRMED',
                     'GATED_OFFER', 'OFFER_UNCONFIRMED', 'OFFER_CONFLICT_WIKI_UNDECIDED',
                     'ITEM_NOT_REGISTERED', 'CURRENCY_CONFLICT'}
TEXT_KEYS = {'text', 'description', 'voices', 'dialogue'}
PROVENANCE_PREFIX = {'canary': 'canary:npc/', 'crystal': 'crystal:npc/'}


def _is_int(value):
    return isinstance(value, int) and not isinstance(value, bool)


def find_text_keys(value, path=''):
    found = []
    if isinstance(value, dict):
        for k, v in value.items():
            here = f'{path}.{k}' if path else k
            if k in TEXT_KEYS:
                found.append(here)
            found += find_text_keys(v, here)
    elif isinstance(value, list):
        for i, v in enumerate(value):
            found += find_text_keys(v, f'{path}[{i}]')
    return found


def in_range_point(point):
    if not isinstance(point, dict) or set(point) != {'x', 'y', 'z'}:
        return False
    x, y, z = point['x'], point['y'], point['z']
    return (_is_int(x) and _is_int(y) and _is_int(z)
            and 0 <= x <= 65535 and 0 <= y <= 65535 and 0 <= z <= 15)


def identity_errors(identity, label, family, key_re, expected_key):
    errs = []
    if not isinstance(identity, dict):
        return [f'{label}: identity missing or malformed']
    if identity.get('family') != family:
        errs.append(f"{label}: identity.family {identity.get('family')!r} != {family!r}")
    key = identity.get('key')
    if not isinstance(key, str) or not key_re.match(key):
        errs.append(f'{label}: identity.key {key!r} does not match the {family} key pattern')
    if expected_key is not None and key != expected_key:
        errs.append(f'{label}: identity.key {key!r} != expected {expected_key!r} (slug of name)')
    if identity.get('revision') != 'definition-r1':
        errs.append(f"{label}: identity.revision {identity.get('revision')!r} != 'definition-r1'")
    return errs


def _travel_key_re(name_slug):
    return re.compile(r'^oteryn:service\.travel\.' + re.escape(name_slug) + r'$')


def _trade_key_re(name_slug):
    return re.compile(r'^oteryn:service\.trade\.' + re.escape(name_slug) + r'$')


def candidate_errors(candidate, index):
    errs = []
    label = f'candidates[{index}]'
    name = candidate.get('name')
    name_slug = slug(name) if isinstance(name, str) else None
    expected_key = f'oteryn:npc.{name_slug}' if name_slug is not None else None
    errs += identity_errors(candidate.get('identity'), label, 'NPC', KEY_RE, expected_key)

    arbitration_rows = candidate.get('arbitration') or []
    wiki_confirmed = any(row.get('rule') == 'WIKI_CONFIRMED' for row in arbitration_rows)

    placements = candidate.get('placements') or []
    if not placements and not wiki_confirmed:
        errs.append(f'{label}: no placements')
    wiki_placements = [p for p in placements if isinstance(p, dict) and p.get('origin') == 'wiki']
    for i, placement in enumerate(placements):
        plabel = f'{label}.placements[{i}]'
        if not in_range_point(placement.get('position')):
            errs.append(f"{plabel}: position {placement.get('position')!r} out of range or malformed")
        origin = placement.get('origin')
        if origin is not None and origin != 'wiki':
            errs.append(f"{plabel}: origin {origin!r} not null or 'wiki'")
        wiki_origin = origin == 'wiki'
        direction = placement.get('direction')
        if wiki_origin:
            if direction is not None:
                errs.append(f'{plabel}: wiki-origin placement direction must be null, got {direction!r}')
        elif direction not in DIRECTIONS:
            errs.append(f'{plabel}: direction {direction!r} not in {sorted(DIRECTIONS)}')
        interval = placement.get('spawn_interval_s')
        if wiki_origin:
            if interval is not None:
                errs.append(f'{plabel}: wiki-origin placement spawn_interval_s must be null, got {interval!r}')
        elif not _is_int(interval) or not (1 <= interval <= 86400):
            errs.append(f'{plabel}: spawn_interval_s {interval!r} not an int in 1..86400')
        radius = placement.get('spawn_radius')
        if wiki_origin:
            if radius is not None:
                errs.append(f'{plabel}: wiki-origin placement spawn_radius must be null, got {radius!r}')
        elif not _is_int(radius) or radius < 0:
            errs.append(f'{plabel}: spawn_radius {radius!r} not a non-negative int')

    provenance = candidate.get('provenance') or {}
    if not isinstance(provenance, dict) or not (1 <= len(provenance) <= 2) \
            or not set(provenance).issubset(PROVENANCE_PREFIX):
        errs.append(f'{label}: provenance must have 1 or 2 of canary/crystal, got {sorted(provenance)}')
    for source, entry in provenance.items():
        plabel = f'{label}.provenance.{source}'
        entry = entry or {}
        prefix = PROVENANCE_PREFIX.get(source)
        key = entry.get('key')
        if prefix and (not isinstance(key, str) or not key.startswith(prefix)):
            errs.append(f'{plabel}: key {key!r} does not start with {prefix!r}')
        sha = entry.get('sha256')
        if not isinstance(sha, str) or not SHA256_RE.match(sha):
            errs.append(f'{plabel}: sha256 {sha!r} is not 64 hex chars')
    if len(provenance) == 1 and candidate.get('wiki') is None:
        errs.append(f'{label}: single-source candidate has no wiki confirmation (D6)')

    route_keywords = set()
    travel = candidate.get('travel_service')
    if travel is not None:
        tlabel = f'{label}.travel_service'
        travel_expected_key = f'oteryn:service.travel.{name_slug}' if name_slug is not None else None
        travel_key_re = _travel_key_re(name_slug) if name_slug is not None else re.compile(r'^oteryn:service\.travel\.')
        errs += identity_errors(travel.get('identity'), tlabel, 'Service', travel_key_re, travel_expected_key)
        routes = travel.get('routes') or []
        if not routes:
            errs.append(f'{tlabel}: no routes')
        for j, route in enumerate(routes):
            rlabel = f'{tlabel}.routes[{j}]'
            if not in_range_point(route.get('destination')):
                errs.append(f"{rlabel}: destination {route.get('destination')!r} out of range or malformed")
            price = route.get('price')
            if not _is_int(price) or price < 0:
                errs.append(f'{rlabel}: price {price!r} is not a non-negative integer')
            keyword = route.get('destination_keyword')
            if keyword in route_keywords:
                errs.append(f'{rlabel}: duplicate destination_keyword {keyword!r}')
            route_keywords.add(keyword)

    trade = candidate.get('trade_service')
    if trade is not None:
        tlabel = f'{label}.trade_service'
        trade_expected_key = f'oteryn:service.trade.{name_slug}' if name_slug is not None else None
        trade_key_re = _trade_key_re(name_slug) if name_slug is not None else re.compile(r'^oteryn:service\.trade\.')
        errs += identity_errors(trade.get('identity'), tlabel, 'Service', trade_key_re, trade_expected_key)
        currency = trade.get('currency')
        if currency is not None:
            errs += identity_errors(currency, f'{tlabel}.currency', 'Item', ITEM_KEY_RE, None)
        offers = trade.get('offers') or []
        if not offers:
            errs.append(f'{tlabel}: no offers')
        offer_tuples = set()
        for j, offer in enumerate(offers):
            olabel = f'{tlabel}.offers[{j}]'
            errs += identity_errors(offer.get('item'), f'{olabel}.item', 'Item', ITEM_KEY_RE, None)
            source_item_id = offer.get('source_item_id')
            if not _is_int(source_item_id) or source_item_id <= 0:
                errs.append(f'{olabel}: source_item_id {source_item_id!r} is not a positive integer')
            if offer.get('direction') not in TRADE_DIRECTIONS:
                errs.append(f"{olabel}: direction {offer.get('direction')!r} not in {sorted(TRADE_DIRECTIONS)}")
            unit_price = offer.get('unit_price')
            if not _is_int(unit_price) or unit_price < 0:
                errs.append(f'{olabel}: unit_price {unit_price!r} is not a non-negative integer')
            count = offer.get('count')
            if count is not None and (not _is_int(count) or count < 1):
                errs.append(f'{olabel}: count {count!r} is not None or an int >= 1')
            if offer.get('origin', None) not in (None, 'wiki'):
                errs.append(f"{olabel}: origin {offer.get('origin')!r} is neither absent nor 'wiki'")
            sub_type = offer.get('sub_type')
            if sub_type is not None and not _is_int(sub_type):
                errs.append(f'{olabel}: sub_type {sub_type!r} is not None or an int')
            offer_tuple = (source_item_id, count, sub_type, offer.get('direction'))
            if offer_tuple in offer_tuples:
                errs.append(f'{olabel}: duplicate offer tuple {offer_tuple!r} within candidate')
            offer_tuples.add(offer_tuple)

    for i, row in enumerate(arbitration_rows):
        alabel = f'{label}.arbitration[{i}]'
        rule = row.get('rule')
        chosen = row.get('chosen')
        fact = row.get('fact')
        if rule == 'WIKI_ARBITER':
            if chosen not in ('canary', 'crystal'):
                errs.append(f'{alabel}: chosen {chosen!r} not in canary/crystal')
            elif chosen not in provenance:
                errs.append(f'{alabel}: chosen {chosen!r} is not one of this candidate\'s provenance sources')
        elif rule == 'WIKI_POSITION':
            if chosen != 'wiki':
                errs.append(f"{alabel}: chosen {chosen!r} != 'wiki' for rule {rule!r}")
            if fact != 'placements':
                errs.append(f"{alabel}: fact {fact!r} != 'placements' for rule {rule!r}")
            if len(wiki_placements) != 1:
                errs.append(f"{alabel}: rule 'WIKI_POSITION' requires exactly one wiki-origin "
                             f"placement, found {len(wiki_placements)}")
            if candidate.get('wiki') is None:
                errs.append(f"{alabel}: rule {rule!r} requires a wiki page, candidate.wiki is null")
        elif rule == 'WIKI_CONFIRMED':
            if chosen != 'wiki':
                errs.append(f"{alabel}: chosen {chosen!r} != 'wiki' for rule {rule!r}")
            if fact != 'placements':
                errs.append(f"{alabel}: fact {fact!r} != 'placements' for rule {rule!r}")
            if placements:
                errs.append(f"{alabel}: rule 'WIKI_CONFIRMED' requires an empty placements list, "
                             f"found {len(placements)}")
            if candidate.get('wiki') is None:
                errs.append(f"{alabel}: rule {rule!r} requires a wiki page, candidate.wiki is null")
        elif rule in ('WIKI_BASE_NAME', 'WIKI_SPELLING'):
            if chosen != 'wiki':
                errs.append(f"{alabel}: chosen {chosen!r} != 'wiki' for rule {rule!r}")
            if fact != 'identity':
                errs.append(f"{alabel}: fact {fact!r} != 'identity' for rule {rule!r}")
            if len(provenance) != 1:
                errs.append(f"{alabel}: rule {rule!r} requires a single-source candidate, "
                             f"provenance has {sorted(provenance)}")
            if candidate.get('wiki') is None:
                errs.append(f"{alabel}: rule {rule!r} requires a wiki page, candidate.wiki is null")
        elif rule in ITEM_RULES:
            wikis = row.get('wikis')
            if rule == 'WIKI_MAJORITY_PRICE' and (
                    not isinstance(wikis, list) or not 2 <= len(wikis) <= 3 or wikis != sorted(set(wikis))
                    or not set(wikis) <= WIKIS or 'tibiopedia' not in wikis):
                errs.append(f"{alabel}: wikis {wikis!r} are not 2-3 sorted wikis from fandom/br/tibiopedia "
                            f"including tibiopedia")
            if rule == 'WIKI_OFFER' and (
                    not isinstance(wikis, list) or not 2 <= len(wikis) <= 3 or wikis != sorted(set(wikis))
                    or not set(wikis) <= WIKIS):
                errs.append(f"{alabel}: wikis {wikis!r} are not 2-3 sorted wikis from fandom/br/tibiopedia")
            if rule == 'WIKI_OFFER' and isinstance(fact, str) and not re.fullmatch(r'trade\.\d+\.\w+', fact):
                errs.append(f"{alabel}: a WIKI_OFFER fact {fact!r} names an offer with a count or sub type")
            if rule == 'WIKI_PRICE' and 'wikis' in row:
                errs.append(f"{alabel}: rule 'WIKI_PRICE' carries no wikis")
            if chosen != 'wiki':
                errs.append(f"{alabel}: chosen {chosen!r} != 'wiki' for rule {rule!r}")
            if not isinstance(row.get('item_name'), str) or not row['item_name']:
                errs.append(f"{alabel}: item_name {row.get('item_name')!r} is not a non-empty string")
            price = row.get('price')
            if not _is_int(price) or price < 0:
                errs.append(f"{alabel}: price {price!r} is not a non-negative integer")
            match = WIKI_PRICE_FACT.fullmatch(fact) if isinstance(fact, str) else None
            if match is None:
                errs.append(f"{alabel}: fact {fact!r} is not a trade offer direction for rule {rule!r}")
            else:
                # the fact names an admitted offer, and that offer carries the wiki price
                named = named_offers(candidate, match)
                if not named:
                    errs.append(f"{alabel}: fact {fact!r} names no admitted offer")
                elif any(offer.get('unit_price') != price for offer in named):
                    errs.append(f"{alabel}: offer unit_price {[o.get('unit_price') for o in named]!r} != "
                                f"{rule} price {price!r}")
            # a Fandom page backs every WIKI_PRICE row and a majority Fandom is part of; BR and Tibiopedia alone need none
            if candidate.get('wiki') is None and (rule == 'WIKI_PRICE' or 'fandom' in (wikis or [])):
                errs.append(f"{alabel}: rule {rule!r} requires a wiki page, candidate.wiki is null")
        else:
            errs.append(f"{alabel}: rule {rule!r} not in "
                         f"['WIKI_ARBITER', 'WIKI_BASE_NAME', 'WIKI_CONFIRMED', 'WIKI_MAJORITY_PRICE', "
                         f"'WIKI_POSITION', 'WIKI_PRICE', 'WIKI_SPELLING']")

    # D13 offers: a wiki-origin offer and its WIKI_OFFER row come together, one row per offer
    wiki_offer_facts = sorted(f"trade.{offer.get('source_item_id')}.{offer.get('direction')}"
                              for offer in (candidate.get('trade_service') or {}).get('offers') or []
                              if offer.get('origin') == 'wiki')
    offer_rows = sorted(row.get('fact') for row in arbitration_rows if row.get('rule') == 'WIKI_OFFER')
    if wiki_offer_facts != offer_rows:
        errs.append(f'{label}: wiki-origin offers {wiki_offer_facts} do not match the WIKI_OFFER rows {offer_rows}')
    if wiki_placements and not any(row.get('rule') == 'WIKI_POSITION' for row in arbitration_rows):
        errs.append(f'{label}: wiki-origin placement present without a WIKI_POSITION arbitration row')

    left_out_keywords = set()
    for i, row in enumerate(candidate.get('left_out') or []):
        llabel = f'{label}.left_out[{i}]'
        if row.get('reason') not in LEFT_OUT_REASONS:
            errs.append(f"{llabel}: reason {row.get('reason')!r} not in {sorted(LEFT_OUT_REASONS)}")
        fact = row.get('fact') or ''
        if fact.startswith('travel.'):
            left_out_keywords.add(fact[len('travel.'):])
    overlap = left_out_keywords & route_keywords
    if overlap:
        errs.append(f'{label}: left_out travel keyword(s) also promoted as route(s): {sorted(overlap)}')

    text_hits = find_text_keys(candidate)
    if text_hits:
        errs.append(f'{label}: forbidden text-bearing key(s) present: {text_hits}')

    return errs


def held_errors(held, index):
    label = f'held[{index}]'
    errs = []
    if not isinstance(held.get('reason'), str) or not held['reason']:
        errs.append(f'{label}: missing or empty reason')
    if not isinstance(held.get('name'), str) or not held['name']:
        errs.append(f'{label}: missing or empty name')
    return errs


def errors(report):
    errs = []
    if report.get('schema') != SCHEMA:
        errs.append(f"schema {report.get('schema')!r} != {SCHEMA!r}")
    if report.get('evidence') != EVIDENCE:
        errs.append(f"evidence {report.get('evidence')!r} != {EVIDENCE!r}")
    expected = DECISIONS + (['D12'] if 'br_facts_sha256' in report else []) + (
        ['D13'] if 'tibiopedia_facts_sha256' in report else [])
    if report.get('decisions') != expected:
        errs.append(f"decisions {report.get('decisions')!r} != {expected!r}")
    for field in ('br_facts_sha256', 'tibiopedia_facts_sha256'):
        if field in report and not re.fullmatch(r'[0-9a-f]{64}', str(report[field])):
            errs.append(f'{field} is not 64 hex chars')
    if 'tibiopedia_facts_sha256' in report and 'br_facts_sha256' not in report:
        errs.append('tibiopedia_facts_sha256 without br_facts_sha256 (D13 needs D12)')
    # a WIKI_MAJORITY_PRICE row is only valid with the Tibiopedia facts it was decided from (D13)
    for d13_rule in ('WIKI_MAJORITY_PRICE', 'WIKI_OFFER'):
        if 'tibiopedia_facts_sha256' not in report and any(
                row.get('rule') == d13_rule for candidate in report.get('candidates') or []
                for row in candidate.get('arbitration') or []):
            errs.append(f'{d13_rule} arbitration without tibiopedia_facts_sha256 (D13)')
    # a WIKI_PRICE row is only valid with the BR facts it was decided from (D12)
    if 'br_facts_sha256' not in report and any(row.get('rule') == 'WIKI_PRICE' for candidate in
                                               report.get('candidates') or [] for row in candidate.get('arbitration') or []):
        errs.append('WIKI_PRICE arbitration without br_facts_sha256 (D12)')
    snapshot_sha = report.get('snapshot_sha256')
    if not isinstance(snapshot_sha, str) or not SHA256_RE.match(snapshot_sha):
        errs.append(f'snapshot_sha256 {snapshot_sha!r} is not 64 hex chars')
    item_map_sha = report.get('item_map_sha256')
    if not isinstance(item_map_sha, str) or not SHA256_RE.match(item_map_sha):
        errs.append(f'item_map_sha256 {item_map_sha!r} is not 64 hex chars')

    candidates = report.get('candidates') or []
    keys = []
    item_key_by_source = {}
    for i, candidate in enumerate(candidates):
        errs += candidate_errors(candidate, i)
        keys.append(candidate.get('identity', {}).get('key'))
        trade = candidate.get('trade_service')
        for offer in (trade or {}).get('offers') or []:
            source_item_id = offer.get('source_item_id')
            item_key = (offer.get('item') or {}).get('key')
            if not isinstance(source_item_id, int) or not isinstance(item_key, str):
                continue
            seen_key = item_key_by_source.get(source_item_id)
            if seen_key is None:
                item_key_by_source[source_item_id] = item_key
            elif seen_key != item_key:
                errs.append(f'candidates[{i}]: source_item_id {source_item_id} maps to item key '
                            f'{item_key!r} but earlier mapped to {seen_key!r}')
    if len(keys) != len(set(keys)):
        dupes = sorted({k for k in keys if keys.count(k) > 1})
        errs.append(f'duplicate candidate key(s): {dupes}')
    sort_key = [k if isinstance(k, str) else '' for k in keys]
    if sort_key != sorted(sort_key):
        errs.append('candidates are not sorted by identity.key')

    totals = report.get('totals') or {}
    if totals.get('candidates') != len(candidates):
        errs.append(f"totals.candidates {totals.get('candidates')!r} != {len(candidates)}")
    with_travel = sum(1 for c in candidates if c.get('travel_service'))
    if totals.get('with_travel') != with_travel:
        errs.append(f"totals.with_travel {totals.get('with_travel')!r} != {with_travel}")
    with_trade = sum(1 for c in candidates if c.get('trade_service'))
    if totals.get('with_trade') != with_trade:
        errs.append(f"totals.with_trade {totals.get('with_trade')!r} != {with_trade}")
    offers_total = sum(len((c.get('trade_service') or {}).get('offers') or []) for c in candidates)
    if totals.get('offers') != offers_total:
        errs.append(f"totals.offers {totals.get('offers')!r} != {offers_total}")

    for i, held in enumerate(report.get('held') or []):
        errs += held_errors(held, i)

    return errs


registry_item_names = promotion_candidates.registry_item_names


def item_name_errors(report, registry_names):
    """Every WIKI_PRICE/WIKI_MAJORITY_PRICE row's item_name is the registered name of the offer its fact names, so one item's
    wiki price can never justify another item's offer. `registry_names` maps Item keys to folded names."""
    errs = []
    for index, candidate in enumerate(report.get('candidates') or []):
        for row in candidate.get('arbitration') or []:
            match = WIKI_PRICE_FACT.fullmatch(row.get('fact') or '') if row.get('rule') in ITEM_RULES else None
            if match is None:
                continue
            named = {registry_names.get((offer.get('item') or {}).get('key')) for offer in named_offers(candidate, match)}
            if named != {promotion_candidates.fold(str(row.get('item_name')))}:
                errs.append(f"candidates[{index}]: {row['rule']} {row['fact']} item_name {row.get('item_name')!r} "
                            f"is not the offer's registered item {sorted(n for n in named if n)!r}")
    return errs


def wiki_price_errors(report, snapshot_bytes, br_facts_bytes, registry_names, tibiopedia_bytes=None, item_map_bytes=None):
    """With the pinned inputs at hand (D12, and D13 with the Tibiopedia facts), using the same lookup the
    candidates were built with: every WIKI_PRICE row is the price Fandom and BR both state, every
    WIKI_MAJORITY_PRICE row the price its wikis state, and every admitted offer whose registered item two
    wikis price the same for that NPC and direction carries that price, so an omitted override cannot pass.
    With the item map as well (D13 offers), each candidate's WIKI_OFFER rows are exactly the ones the builder
    derives from its other offers, so an omitted or invented wiki offer cannot pass either."""
    errs = []
    if hashlib.sha256(snapshot_bytes).hexdigest() != report.get('snapshot_sha256'):
        errs.append('--snapshot does not match snapshot_sha256')
    if hashlib.sha256(br_facts_bytes).hexdigest() != report.get('br_facts_sha256'):
        errs.append('--br-facts does not match br_facts_sha256')
    if (tibiopedia_bytes and hashlib.sha256(tibiopedia_bytes).hexdigest()) != report.get('tibiopedia_facts_sha256'):
        errs.append('--tibiopedia-facts does not match tibiopedia_facts_sha256')
    if item_map_bytes is not None and hashlib.sha256(item_map_bytes).hexdigest() != report.get('item_map_sha256'):
        errs.append('--item-map does not match item_map_sha256')
    if errs:
        return errs
    item_map = json.loads(item_map_bytes) if item_map_bytes is not None else {'records': []}
    builder = promotion_candidates.Builder(json.loads(snapshot_bytes), item_map, json.loads(br_facts_bytes),
                                           json.loads(tibiopedia_bytes) if tibiopedia_bytes else None, registry_names)

    def expected(name, direction, item_name, fandom, plain=True):
        """The price (D12, else D13) the offer must carry, the rule that sets it and that rule's wikis; D13 only
        prices a plain offer, with no count or sub type."""
        price = builder.wiki_price(name, direction, item_name, fandom)
        if price is not None or not tibiopedia_bytes or not plain:
            return price, 'WIKI_PRICE', None
        price, wikis = builder.majority_price(name, direction, item_name, fandom)
        return price, 'WIKI_MAJORITY_PRICE', wikis

    for index, candidate in enumerate(report.get('candidates') or []):
        name = candidate.get('name')
        if not isinstance(name, str):
            continue  # reported by errors()
        fandom = {row['item'].lower(): row
                  for row in builder.wiki_trade.get(promotion_candidates.normalize_name(name), [])}
        for row in candidate.get('arbitration') or []:
            match = WIKI_PRICE_FACT.fullmatch(row.get('fact') or '') if row.get('rule') in PRICE_RULES else None
            if match is None:
                continue  # not a price row, or a malformed one errors() reports
            stated, rule, wikis = expected(name, match.group(4), row.get('item_name'), fandom,
                                           match.group(2) is None and match.group(3) is None)
            if (stated, rule, wikis) != (row.get('price'), row['rule'], row.get('wikis')):
                errs.append(f"candidates[{index}]: {row['rule']} {row['fact']} price {row.get('price')!r} "
                            f"wikis {row.get('wikis')!r} != what the wikis state ({rule} {stated!r} {wikis!r})")
        trade = candidate.get('trade_service') or {}
        if tibiopedia_bytes and item_map_bytes is not None and trade.get('currency') is None \
                and name not in promotion_candidates.WIKI_SHOP_HELD:
            stated_rows = [row for row in candidate.get('arbitration') or [] if row.get('rule') == 'WIKI_OFFER']
            base = [offer for offer in trade.get('offers') or [] if offer.get('origin') != 'wiki']
            derived = []
            builder.wiki_offers(name, base, candidate.get('left_out') or [], derived)
            if sorted(json.dumps(row, sort_keys=True) for row in derived) != sorted(json.dumps(row, sort_keys=True) for row in stated_rows):
                errs.append(f"candidates[{index}]: WIKI_OFFER rows differ from the offers two wikis agree on "
                            f"({len(stated_rows)} stated, {len(derived)} derived)")
        for offer in trade.get('offers') or []:
            item_name = registry_names.get((offer.get('item') or {}).get('key'))
            plain = offer.get('count') is None and offer.get('sub_type') is None
            stated = expected(name, offer.get('direction'), item_name, fandom, plain)[0] if item_name else None
            if stated is not None and offer.get('unit_price') != stated:
                errs.append(f"candidates[{index}]: offer {item_name!r} {offer.get('direction')} unit_price "
                            f"{offer.get('unit_price')!r} != the price the wikis agree on ({stated!r})")
    return errs


def main():
    parser = argparse.ArgumentParser(description=__doc__, formatter_class=argparse.RawDescriptionHelpFormatter)
    parser.add_argument('report')
    parser.add_argument('--snapshot', type=Path, help='the pinned Fandom snapshot; checks every WIKI_PRICE row')
    parser.add_argument('--br-facts', type=Path, help='the pinned TibiaWiki BR facts; checks every WIKI_PRICE row')
    parser.add_argument('--item-map', type=Path, help='the pinned item map; checks every WIKI_OFFER row (D13 offers)')
    parser.add_argument('--tibiopedia-facts', type=Path,
                        help='the pinned Tibiopedia facts; checks every WIKI_MAJORITY_PRICE row (with the two above)')
    args = parser.parse_args()
    if bool(args.snapshot) != bool(args.br_facts):
        parser.error('--snapshot and --br-facts check the WIKI_PRICE evidence together; pass both or neither')
    parser_item_map = args.item_map
    if args.tibiopedia_facts and not args.snapshot:
        parser.error('--tibiopedia-facts checks the D13 evidence with --snapshot and --br-facts')
    if parser_item_map and not args.tibiopedia_facts:
        parser.error('--item-map checks the D13 wiki offers with --tibiopedia-facts')
    report = json.loads(Path(args.report).read_text(encoding='utf-8'))
    registry_names = registry_item_names()
    all_errors = errors(report) + item_name_errors(report, registry_names)
    if args.snapshot and args.br_facts:
        all_errors += wiki_price_errors(report, args.snapshot.read_bytes(), args.br_facts.read_bytes(), registry_names,
                                        args.tibiopedia_facts.read_bytes() if args.tibiopedia_facts else None,
                                        args.item_map.read_bytes() if args.item_map else None)

    candidates = report.get('candidates') or []
    total = len(candidates)
    by_candidate, other = {}, []
    for error in all_errors:
        match = re.match(r'candidates\[(\d+)\]', error)
        if match:
            by_candidate.setdefault(int(match.group(1)), []).append(error)
        else:
            other.append(error)

    for index in sorted(by_candidate):
        candidate = candidates[index]
        print(f"candidates[{index}] ({candidate.get('name')!r}, key={candidate.get('identity', {}).get('key')!r}):")
        for error in by_candidate[index][:10]:
            print(f'  {error[:300]}')
    if other:
        print('report:')
        for error in other[:20]:
            print(f'  {error[:300]}')

    valid = total - len(by_candidate)
    print(f'{valid}/{total} candidates valid')
    return 1 if all_errors or not total else 0


if __name__ == '__main__':
    sys.exit(main())
