"""Validate an OTERYN_NPC_PROMOTION_CANDIDATES/v1 report (promotion_candidates.py output) against the
schema doc (OTERYN_NPC_AUTHORING_SCHEMA_V1.md §3 D4-D8, §8).

Semantic rules:
- schema == 'OTERYN_NPC_PROMOTION_CANDIDATES/v1', evidence == 'OTS_HYPOTHESIS_ONLY',
  decisions == ['D4', 'D5', 'D6', 'D7', 'D8', 'D11'], plus 'D12' exactly when br_facts_sha256 is
  present; snapshot_sha256, item_map_sha256 and br_facts_sha256 are 64 hex chars;
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
  otherwise invalid) and a non-null `wiki`; or rule 'WIKI_PRICE' (D12) with `chosen` == 'wiki', a
  `trade.<item>.<direction>` fact and a non-null `wiki`; or rule 'WIKI_BASE_NAME'/'WIKI_SPELLING' with
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
import json
import re
import sys
from pathlib import Path

from promotion_candidates import slug

SCHEMA = 'OTERYN_NPC_PROMOTION_CANDIDATES/v1'
EVIDENCE = 'OTS_HYPOTHESIS_ONLY'
DECISIONS = ['D4', 'D5', 'D6', 'D7', 'D8', 'D11']
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
        elif rule == 'WIKI_PRICE':
            if chosen != 'wiki':
                errs.append(f"{alabel}: chosen {chosen!r} != 'wiki' for rule {rule!r}")
            if not isinstance(fact, str) or not fact.startswith('trade.') or not fact.endswith(('.SellToPlayer', '.BuyFromPlayer')):
                errs.append(f"{alabel}: fact {fact!r} is not a trade offer direction for rule {rule!r}")
            if candidate.get('wiki') is None:
                errs.append(f"{alabel}: rule {rule!r} requires a wiki page, candidate.wiki is null")
        else:
            errs.append(f"{alabel}: rule {rule!r} not in "
                         f"['WIKI_ARBITER', 'WIKI_BASE_NAME', 'WIKI_CONFIRMED', 'WIKI_POSITION', "
                         f"'WIKI_PRICE', 'WIKI_SPELLING']")

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
    expected = DECISIONS + (['D12'] if 'br_facts_sha256' in report else [])
    if report.get('decisions') != expected:
        errs.append(f"decisions {report.get('decisions')!r} != {expected!r}")
    if 'br_facts_sha256' in report and not re.fullmatch(r'[0-9a-f]{64}', str(report['br_facts_sha256'])):
        errs.append('br_facts_sha256 is not 64 hex chars')
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


def main():
    parser = argparse.ArgumentParser(description=__doc__, formatter_class=argparse.RawDescriptionHelpFormatter)
    parser.add_argument('report')
    args = parser.parse_args()
    report = json.loads(Path(args.report).read_text(encoding='utf-8'))
    all_errors = errors(report)

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
