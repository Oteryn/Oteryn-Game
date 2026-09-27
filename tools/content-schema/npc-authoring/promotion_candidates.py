"""Build NPC promotion candidates: merged Canary+Crystal facts with native keys (owner decisions D4-D8).

Evidence tooling only; nothing is written to content/. A candidate is what a later, reviewed
promotion would write to content/npcs/definitions/ and content/services/travel/.

Scope of a candidate (schema doc §8): NPC definition (name, profession, presentation, movement),
placements and ungated travel routes. Description, voices and dialogue are excluded until Oteryn
authors its own text (D5); trade waits for the native Item join (O5).

Merge rules:
- a fact both sources state identically, or only one source states, is adopted for definition fields;
- placements and travel routes that differ, or that only one source has, are arbitrated by the
  TibiaWiki (Fandom) snapshot (D6): the source the wiki agrees with wins (placements: the source with
  the better position match, MATCH over NEAR over MISMATCH; routes: equal price); no agreement keeps
  the fact open and it is left out of the candidate;
- a definition conflict holds the whole NPC (the wiki has no outfit/movement facts);
- a gated route (LUA_PREDICATE) is left out and reported;
- D8 (owner request 2026-09-27, completing held NPCs from the wiki):
  - an NPC neither source places, whose wiki page has a position, is promoted with that one
    wiki-origin placement (direction/spawn_interval_s/spawn_radius unknown, so null; marked
    `"origin": "wiki"`); arbitration records `{"fact": "placements", "rule": "WIKI_POSITION",
    "chosen": "wiki"}`. Still no wiki position (or no wiki page) stays held UNPLACED;
  - a single-source NPC absent from the wiki under its own name is tried again under a base name
    (stripping a trailing ` (Day)`/` (Night)`, or ` Init`/` Vampires Lair`/` Back`); a wiki match
    on the base name confirms the NPC (records `{"fact": "identity", "rule": "WIKI_BASE_NAME",
    "chosen": "wiki"}`); several name variants may then share one wiki page. No base-name match
    stays held SINGLE_SOURCE_NOT_ON_WIKI;
- key: `oteryn:npc.<slug>` where the slug is derived once from the registered name (ASCII fold,
  lower case, non-alphanumerics to `_`). After promotion the key is frozen: a later rename keeps it.
  Two NPCs with the same slug are both held (D4); a name with no alphanumerics is held (EMPTY_SLUG).

Usage: python promotion_candidates.py --canary out/canary/bundles --crystal out/crystal/bundles \
         --snapshot out/fandom/fandom-npc-snapshot.json --out samples/promotion-candidates-v1.json
"""
import argparse
import hashlib
import json
import re
import unicodedata
from collections import Counter, defaultdict
from pathlib import Path

import source_diff
from wiki_fandom import classify_position, compare_travel, normalize_name

SCHEMA = 'OTERYN_NPC_PROMOTION_CANDIDATES/v1'
POSITION_RANK = {'MATCH': 0, 'NEAR': 1, 'MISMATCH': 2}
LOADABLE = ('RESOLVED', 'PARTIAL')
PLACEMENT_FACTS = ('position', 'direction', 'spawn_interval_s', 'spawn_radius')
WIKI_ARBITRATION_RULES = ('WIKI_ARBITER', 'WIKI_POSITION', 'WIKI_BASE_NAME')  # kept in a candidate's arbitration list
DAY_NIGHT_RE = re.compile(r'^(.*)\s+\((day|night)\)$', re.IGNORECASE)
VARIANT_NAME_SUFFIXES = (' Init', ' Vampires Lair', ' Back')


def slug(name):
    folded = unicodedata.normalize('NFKD', name).encode('ascii', 'ignore').decode()
    return re.sub(r'[^a-z0-9]+', '_', folded.lower()).strip('_')


def base_name(name):
    """D8: strip a known name-variant suffix so a Day/Night (or Init/Vampires Lair/Back) variant can be
    matched to its base wiki page. Returns None when `name` carries none of these suffixes."""
    match = DAY_NIGHT_RE.match(name)
    if match:
        return match.group(1)
    for suffix in VARIANT_NAME_SUFFIXES:
        if name.endswith(suffix) and len(name) > len(suffix):
            return name[:-len(suffix)]
    return None


def wiki_position_placement(wiki):
    """D8: a synthetic placement for an UNPLACED NPC whose wiki page has a position. direction,
    spawn_interval_s and spawn_radius are unknown from the wiki, so they are left null; `origin`
    marks the row as wiki-derived rather than sourced from Canary/Crystal."""
    if not wiki or not wiki.get('position'):
        return None
    return {'position': wiki['position'], 'direction': None, 'spawn_interval_s': None,
            'spawn_radius': None, 'origin': 'wiki'}


def definition_facts(bundle):
    d = bundle['definition']
    return {'name': d['name'], 'profession': d['profession'], 'outfit': d['presentation']['outfit'],
            'speech_bubble': d['presentation']['speech_bubble'], 'movement': d['movement']}


def placement_key(placement):
    return json.dumps({'position': placement['position'], 'direction': placement['direction']}, sort_keys=True)


def routes(bundle):
    result = {}
    for row in bundle['services']['travel']:
        if row['keyword'] and row['destination'] and row['price'] is not None:
            result.setdefault(normalize_name(row['keyword']), row)
    return result


def route_fact(row):
    return (json.dumps(row['destination'], sort_keys=True), row['price'], row['premium'], row['min_level'], row['gate'])


def offer_key(offer):
    item = offer['client_id'] if offer['client_id'] is not None else offer['server_item_id']
    return (item, offer['count'], offer['sub_type'])


def source_offers(bundle):
    trade = bundle['services']['trade']
    result = {}
    for offer in (trade or {}).get('offers', []):
        result.setdefault(offer_key(offer), offer)
    return result


class Builder:
    def __init__(self, snapshot, item_map):
        self.wiki = {normalize_name(npc['title']): npc for npc in snapshot['npcs']}
        self.wiki_trade = snapshot.get('trade', {})
        self.items = {row['source_item_id']: row for row in item_map['records']}
        self.held, self.stats = [], Counter()

    def hold(self, name, sources, reason, detail=None):
        self.held.append({'name': name, 'sources': sources, 'reason': reason, 'detail': detail})
        self.stats[f'held:{reason}'] += 1

    def merge_definition(self, bundles, arbitration):
        facts = [definition_facts(b) for b in bundles.values()]
        merged, conflicts = {}, []
        for field in facts[0]:
            values = [f[field] for f in facts if f[field] is not None]
            distinct = {json.dumps(v, sort_keys=True) for v in values}
            if len(distinct) > 1:
                conflicts.append(field)
            merged[field] = values[0] if values else None
            if len(bundles) == 2 and values:
                arbitration.append({'fact': f'definition.{field}', 'rule': 'AGREE' if len(values) == 2 else 'ONE_SIDED'})
        return merged, conflicts

    def merge_placements(self, bundles, wiki, arbitration):
        # the spawn file path differs between datapacks and is not a placement fact
        lists = {source: sorted(({k: p[k] for k in PLACEMENT_FACTS} for p in b['placements']), key=placement_key)
                 for source, b in bundles.items()}
        if len(lists) == 1 or len({json.dumps(v, sort_keys=True) for v in lists.values()}) == 1:
            arbitration.append({'fact': 'placements', 'rule': 'AGREE' if len(lists) == 2 else 'SINGLE_SOURCE'})
            return next(iter(lists.values())), None
        if not wiki or not wiki.get('position'):
            return None, 'PLACEMENT_CONFLICT_NO_WIKI_POSITION'
        ranked = {}
        for source, placements in lists.items():
            verdict = classify_position(wiki['position'], placements)
            ranked[source] = (POSITION_RANK.get(verdict['status'], 3), verdict.get('chebyshev_distance', 0))
        best = min(ranked.values())
        winners = [s for s, r in ranked.items() if r == best]
        if len(winners) != 1 or best[0] == 2:
            return None, 'PLACEMENT_CONFLICT_WIKI_UNDECIDED'
        arbitration.append({'fact': 'placements', 'rule': 'WIKI_ARBITER', 'chosen': winners[0]})
        return lists[winners[0]], None

    def merge_routes(self, bundles, wiki, arbitration, left_out):
        per_source = {source: routes(b) for source, b in bundles.items()}
        wiki_prices = {row['destination']: row for source, b in bundles.items()
                       for row in compare_travel((wiki or {}).get('transport') or [], b['services']['travel'])}
        merged = []
        for keyword in sorted(set().union(*per_source.values())):
            present = {s: r[keyword] for s, r in per_source.items() if keyword in r}
            facts = {s: route_fact(r) for s, r in present.items()}
            wiki_row = wiki_prices.get(keyword, {})
            wiki_price = wiki_row.get('wiki_price')
            if len(present) == len(bundles) and len(set(facts.values())) == 1:
                chosen, rule = next(iter(present)), 'AGREE' if len(bundles) == 2 else 'SINGLE_SOURCE'
            else:
                agreeing = [s for s, r in present.items() if wiki_price is not None and r['price'] == wiki_price]
                destinations = {json.dumps(present[s]['destination'], sort_keys=True) for s in agreeing}
                if len(agreeing) >= 1 and len(destinations) == 1:
                    chosen, rule = sorted(agreeing)[0], 'WIKI_ARBITER'
                else:
                    left_out.append({'fact': f'travel.{keyword}', 'reason': 'ROUTE_UNCONFIRMED' if len(present) == 1
                                     else 'ROUTE_CONFLICT_WIKI_UNDECIDED'})
                    continue
            row = present[chosen]
            if row['gate'] != 'NONE':
                left_out.append({'fact': f'travel.{keyword}', 'reason': 'GATED_ROUTE'})
                continue
            if rule != 'AGREE' and rule != 'SINGLE_SOURCE':
                arbitration.append({'fact': f'travel.{keyword}', 'rule': rule, 'chosen': chosen})
            merged.append({'destination_keyword': keyword, 'destination': row['destination'], 'price': row['price'],
                           'premium': row['premium'], 'min_level': row['min_level'], 'discount': row['discount']})
        return merged

    def item_ref(self, source_item_id):
        row = self.items.get(source_item_id)
        return row and {'family': 'Item', 'key': row['native_key'], 'revision': row['native_revision']}

    def merge_offers(self, bundles, name, arbitration, left_out):
        per_source = {source: source_offers(b) for source, b in bundles.items()}
        wiki = {row['item'].lower(): row for row in self.wiki_trade.get(normalize_name(name), [])}
        offers = []
        for key in sorted(set().union(*per_source.values()), key=json.dumps):
            present = {s: o[key] for s, o in per_source.items() if key in o}
            facts = {s: (o['buy_price'], o['sell_price'], json.dumps(o['stock_gate'], sort_keys=True)) for s, o in present.items()}
            label = f'trade.{key[0]}' + (f'x{key[1]}' if key[1] else '') + (f's{key[2]}' if key[2] is not None else '')
            if len(present) == len(bundles) and len(set(facts.values())) == 1:
                chosen = next(iter(present))
            else:
                row = wiki.get(str(next(iter(present.values()))['item_name']).lower())
                agreeing = [s for s, o in present.items() if row
                            and (o['buy_price'] is None or o['buy_price'] == row['buy_price'])
                            and (o['sell_price'] is None or o['sell_price'] == row['sell_price'])]
                if agreeing and len({facts[s] for s in agreeing}) == 1:
                    chosen = sorted(agreeing)[0]
                    arbitration.append({'fact': label, 'rule': 'WIKI_ARBITER', 'chosen': chosen})
                else:
                    left_out.append({'fact': label, 'reason': 'OFFER_UNCONFIRMED' if len(present) == 1
                                     else 'OFFER_CONFLICT_WIKI_UNDECIDED'})
                    continue
            offer = present[chosen]
            if offer['stock_gate']:
                left_out.append({'fact': label, 'reason': 'GATED_OFFER'})
                continue
            item = self.item_ref(key[0])
            if item is None:
                left_out.append({'fact': label, 'reason': 'ITEM_NOT_REGISTERED'})
                continue
            # Canary `buy` is what the player pays the NPC; `sell` is what the NPC pays the player
            for direction, price in (('SellToPlayer', offer['buy_price']), ('BuyFromPlayer', offer['sell_price'])):
                if price is not None:
                    offers.append({'item': item, 'source_item_id': key[0], 'direction': direction, 'unit_price': price,
                                   'count': offer['count'], 'sub_type': offer['sub_type']})
        return offers

    def currency(self, bundles, left_out):
        values = {json.dumps(b['services']['trade']['currency'], sort_keys=True) for b in bundles.values()
                  if b['services']['trade']}
        if len(values) != 1:
            left_out.append({'fact': 'trade.currency', 'reason': 'CURRENCY_CONFLICT'})
            return None, False
        currency = json.loads(values.pop())
        if currency == 'GOLD':
            return None, True
        ref = self.item_ref(currency['client_id'])
        if ref is None:
            left_out.append({'fact': 'trade.currency', 'reason': 'ITEM_NOT_REGISTERED'})
            return None, False
        return ref, True

    def candidate(self, bundles):
        name = next(b['definition']['name'] for b in bundles.values())
        sources = {s: b['key'] for s, b in bundles.items()}
        wiki = self.wiki.get(normalize_name(name))
        arbitration, left_out = [], []
        if len(bundles) == 1 and wiki is None:
            variant = base_name(name)
            wiki = self.wiki.get(normalize_name(variant)) if variant else None
            if wiki is None:
                return self.hold(name, sources, 'SINGLE_SOURCE_NOT_ON_WIKI')
            arbitration.append({'fact': 'identity', 'rule': 'WIKI_BASE_NAME', 'chosen': 'wiki'})
        definition, conflicts = self.merge_definition(bundles, arbitration)
        if conflicts:
            return self.hold(name, sources, 'DEFINITION_CONFLICT', ','.join(conflicts))
        placements, problem = self.merge_placements(bundles, wiki, arbitration)
        if problem:
            return self.hold(name, sources, problem)
        if not placements:
            fallback = wiki_position_placement(wiki)
            if fallback is None:
                return self.hold(name, sources, 'UNPLACED')
            placements = [fallback]
            arbitration.append({'fact': 'placements', 'rule': 'WIKI_POSITION', 'chosen': 'wiki'})
        key_slug = slug(name)
        if not key_slug:  # a punctuation-only name has no slug; it needs a hand-chosen key
            return self.hold(name, sources, 'EMPTY_SLUG')
        travel = self.merge_routes(bundles, wiki, arbitration, left_out)
        trade = None
        if any(b['services']['trade'] for b in bundles.values()):
            currency, usable = self.currency(bundles, left_out)
            offers = self.merge_offers(bundles, name, arbitration, left_out) if usable else []
            if offers:
                trade = {'identity': {'family': 'Service', 'key': f'oteryn:service.trade.{key_slug}',
                                      'revision': 'definition-r1'}, 'currency': currency, 'offers': offers}
        record = {
            'identity': {'family': 'NPC', 'key': f'oteryn:npc.{key_slug}', 'revision': 'definition-r1'},
            'name': name, 'profession': definition['profession'],
            'presentation': {'outfit': definition['outfit'], 'speech_bubble': definition['speech_bubble']},
            'movement': definition['movement'],
            'placements': placements,
            'travel_service': ({'identity': {'family': 'Service', 'key': f'oteryn:service.travel.{key_slug}',
                                             'revision': 'definition-r1'}, 'routes': travel} if travel else None),
            'trade_service': trade,
            'provenance': {s: {'key': b['key'], 'sha256': b['source']['sha256']} for s, b in sorted(bundles.items())},
            'wiki': {'pageid': wiki['pageid'], 'revid': wiki['revid']} if wiki else None,
            'arbitration': [a for a in arbitration if a['rule'] in WIKI_ARBITRATION_RULES],
            'left_out': left_out,
        }
        return record


def main():
    parser = argparse.ArgumentParser(description=__doc__, formatter_class=argparse.RawDescriptionHelpFormatter)
    parser.add_argument('--canary', required=True)
    parser.add_argument('--crystal', required=True)
    parser.add_argument('--snapshot', required=True)
    parser.add_argument('--item-map', required=True, help='export_reference_item_identity_map output')
    parser.add_argument('--out', required=True)
    args = parser.parse_args()
    snapshot_bytes = Path(args.snapshot).read_bytes()
    item_map_bytes = Path(args.item_map).read_bytes()
    item_map = json.loads(item_map_bytes)
    if item_map['schema'] != 'OTERYN_PROTECTED_ITEM_IDENTITY_MAP_EXPORT/v1':
        raise SystemExit('unexpected item map schema')
    builder = Builder(json.loads(snapshot_bytes), item_map)
    canary, crystal = source_diff.load(args.canary), source_diff.load(args.crystal)
    records = []
    for left, right in source_diff.pair(canary, crystal):
        bundles = {s: b for s, b in (('canary', canary.get(left) if left else None),
                                     ('crystal', crystal.get(right) if right else None))
                   if b and b['status'] in LOADABLE and b['definition']['name']}
        if not bundles:
            builder.stats['held:NOT_LOADABLE'] += 1
            continue
        record = builder.candidate(bundles)
        if record:
            records.append(record)
    by_key = defaultdict(list)
    for record in records:
        by_key[record['identity']['key']].append(record)
    promoted = []
    for key, group in sorted(by_key.items()):
        if len(group) > 1:
            for record in group:
                builder.hold(record['name'], {s: p['key'] for s, p in record['provenance'].items()}, 'KEY_COLLISION', key)
            continue
        promoted.append(group[0])
    for record in promoted:  # counted over promoted candidates only, after key collisions are held
        builder.stats['arbitrated_facts'] += len(record['arbitration'])
        builder.stats['routes'] += len(record['travel_service']['routes']) if record['travel_service'] else 0
        builder.stats['offers'] += len(record['trade_service']['offers']) if record['trade_service'] else 0
        for row in record['left_out']:
            builder.stats['left_out_offers' if row['fact'].startswith('trade.') else 'left_out_routes'] += 1
    report = {
        'schema': SCHEMA, 'evidence': 'OTS_HYPOTHESIS_ONLY', 'decisions': ['D4', 'D5', 'D6', 'D7', 'D8'],
        'snapshot_sha256': hashlib.sha256(snapshot_bytes).hexdigest(),
        'item_map_sha256': hashlib.sha256(item_map_bytes).hexdigest(),
        'totals': {'candidates': len(promoted), 'with_travel': sum(1 for r in promoted if r['travel_service']),
                   'with_trade': sum(1 for r in promoted if r['trade_service']),
                   **dict(sorted(builder.stats.items()))},
        'candidates': promoted,
        'held': sorted(builder.held, key=lambda h: (h['reason'], h['name'])),
    }
    Path(args.out).write_text(json.dumps(report, indent=1, sort_keys=True) + '\n', encoding='utf-8')
    print(json.dumps(report['totals'], indent=1))


if __name__ == '__main__':
    main()
