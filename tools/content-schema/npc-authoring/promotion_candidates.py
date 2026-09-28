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
    "chosen": "wiki"}`. A wiki page with no position either (e.g. a seasonal/roaming NPC such as
    Santa Claus, city "Varies") still admits the definition, with an empty `placements` list and
    `{"fact": "placements", "rule": "WIKI_CONFIRMED", "chosen": "wiki"}`. No wiki page at all
    stays held UNPLACED. The same two fallbacks resolve a placement conflict between two sources
    (PLACEMENT_CONFLICT_WIKI_UNDECIDED / PLACEMENT_CONFLICT_NO_WIKI_POSITION) once a wiki page is
    matched: the wiki position, when it has one, wins outright over both disagreeing sources
    (WIKI_POSITION); with no wiki position either, the wiki still confirms the NPC (WIKI_CONFIRMED,
    empty placements). A conflict with no wiki page at all still stays held;
  - wiki NPC lookup matches on normalised title, `name` or `actualname` (the wiki infobox's
    in-game name, e.g. "Omniphant (NPC)"'s actualname "Omniphant"), title taking precedence on a
    key claimed by more than one page;
  - a single-source NPC absent from the wiki under its own name is tried again under a base name
    (stripping a trailing ` (Day)`/` (Night)`, or ` Init`/` Vampires Lair`/` Back`); a wiki match
    on the base name confirms the NPC (records `{"fact": "identity", "rule": "WIKI_BASE_NAME",
    "chosen": "wiki"}`); several name variants may then share one wiki page. Failing that, a
    strict spelling match (name >=10 chars, exactly one wiki NPC's name/actualname within one
    edit -- insertion, deletion, substitution or adjacent transposition) also confirms it
    (`{"fact": "identity", "rule": "WIKI_SPELLING", "chosen": "wiki"}`). No match at all stays
    held SINGLE_SOURCE_NOT_ON_WIKI;
  - a fixed, explicit `OWNER_REJECTED` table (owner decision 2026-09-27) holds a small set of
    source-only NPCs that exist only in that OT server, not in Tibia (`canary:npc/canary` "Canary",
    `crystal:npc/loot_buyer` "Loot Buyer"), before any wiki matching, so they are never promoted
    by any rule above;
- D11: a fixed, explicit `REMOVED_FROM_GAME` table holds NPCs that both wikis record as removed from
  Tibia Global (TibiaWiki BR `removed`, TibiaWiki Fandom `status = deprecated`), the same way;
- D12 (`--br-facts`): an admitted offer's price is replaced by the wiki price when TibiaWiki Fandom and
  TibiaWiki BR state the same explicit price for that NPC, item name and direction and it differs from the
  source price; the row records `{"fact": "trade.<item>.<direction>", "rule": "WIKI_PRICE", "chosen": "wiki",
  "item_name": <the offer's item name>, "price": <the wiki price>}`.
  One wiki alone, or two wikis that disagree, never change a price;
- key: `oteryn:npc.<slug>` where the slug is derived once from the registered name (ASCII fold,
  lower case, non-alphanumerics to `_`). After promotion the key is frozen: a later rename keeps it.
  Two NPCs with the same slug are both held (D4); a name with no alphanumerics is held (EMPTY_SLUG).

Usage: python promotion_candidates.py --canary out/canary/bundles --crystal out/crystal/bundles \
         --snapshot out/fandom/fandom-npc-snapshot.json --item-map out/native-map.json \
         --br-facts ../../../imports/tibiawiki/npc-br/2026-09-28/tibiawiki-br-npc-facts.json \
         --out samples/promotion-candidates-v1.json
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
WIKI_ARBITRATION_RULES = ('WIKI_ARBITER', 'WIKI_POSITION', 'WIKI_BASE_NAME', 'WIKI_SPELLING',
                           'WIKI_CONFIRMED', 'WIKI_PRICE')  # kept in the output
DAY_NIGHT_RE = re.compile(r'^(.*)\s+\((day|night)\)$', re.IGNORECASE)
VARIANT_NAME_SUFFIXES = (' Init', ' Vampires Lair', ' Back')
SPELLING_MIN_LENGTH = 10
# Owner decision 2026-09-27: these source-only NPCs exist only in that OT server, not in Tibia,
# and are never promoted by any rule (wiki confirmation, base-name, spelling or otherwise).
OWNER_REJECTED = {
    'canary:npc/canary': 'server-only NPC, owner decision 2026-09-27',
    'crystal:npc/loot_buyer': 'server-only NPC, owner decision 2026-09-27',
}
# D11: removed from Tibia Global in 13.12 with the Duelling Arena; TibiaWiki BR `removed = 13.12.13018`
# (imports/tibiawiki/npc-br/2026-09-28) and TibiaWiki Fandom `status = deprecated` agree.
DUELLING_ARENA_REMOVED = 'Duelling Arena supervisor, removed in 13.12 (TibiaWiki BR removed, Fandom deprecated)'
REMOVED_FROM_GAME = {
    f'{source}:npc/{stem}': DUELLING_ARENA_REMOVED
    for stem in ('brom', 'brutus', 'roughington', 'shadowpunch', 'victor') for source in ('canary', 'crystal')
}


def fold(text):
    return re.sub(r'\s+', ' ', text).strip().casefold()


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


def build_wiki_index(npcs):
    """D8: alias index for wiki NPC lookup. Normalised title is indexed first and always wins a
    key an alias also claims. Normalised `name`/`actualname` (the infobox in-game name, which may
    differ from a disambiguated page title, e.g. "Omniphant (NPC)"'s actualname "Omniphant") are
    then added, but only when exactly one distinct page produces that alias key: a key two or
    more different pages expose (and no exact title owns) is ambiguous and left out entirely,
    rather than silently bound to whichever page happened to be seen first."""
    index = {}
    for npc in npcs:
        key = normalize_name(npc['title'])
        if key:
            index.setdefault(key, npc)
    alias_pageids = defaultdict(set)
    alias_npc = {}
    for npc in npcs:
        for field in ('name', 'actualname'):
            key = normalize_name(npc.get(field))
            if not key or key in index:  # an exact title already owns this key; title wins
                continue
            alias_pageids[key].add(npc['pageid'])
            alias_npc[key] = npc
    for key, pageids in alias_pageids.items():
        if len(pageids) == 1:
            index[key] = alias_npc[key]
    return index


def within_one_edit(a, b):
    """D8 WIKI_SPELLING: restricted Damerau-Levenshtein distance == 1 -- exactly one insertion,
    deletion, substitution or adjacent transposition turns `a` into `b`. Equal strings are not
    "one edit" (they are an exact match, handled elsewhere)."""
    if a == b:
        return False
    la, lb = len(a), len(b)
    if la == lb:
        diffs = [i for i in range(la) if a[i] != b[i]]
        if len(diffs) == 1:
            return True
        if len(diffs) == 2 and diffs[1] == diffs[0] + 1:
            i, j = diffs
            return a[i] == b[j] and a[j] == b[i]
        return False
    if abs(la - lb) != 1:
        return False
    shorter, longer = (a, b) if la < lb else (b, a)
    i = 0
    while i < len(shorter) and shorter[i] == longer[i]:
        i += 1
    return shorter[i:] == longer[i + 1:]


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
    def __init__(self, snapshot, item_map, br_facts=None):
        # D12: TibiaWiki BR trade lists by folded NPC title/name, then direction, then folded item name
        self.br_trade = {}
        for page in (br_facts or {}).get('pages', []):
            trades = {}
            for direction, items in page['trades'].items():
                trades[direction] = {}
                for item, prices in items.items():  # every BR row of the offer, each with its own price
                    trades[direction].setdefault(fold(item), []).extend(prices)
            for label in (page['title'], page['name']):
                self.br_trade.setdefault(fold(label), trades)
        self.wiki_npcs = snapshot['npcs']
        self.wiki = build_wiki_index(self.wiki_npcs)
        self.wiki_trade = snapshot.get('trade', {})
        self.items = {row['source_item_id']: row for row in item_map['records']}
        self.held, self.stats = [], Counter()

    def hold(self, name, sources, reason, detail=None):
        self.held.append({'name': name, 'sources': sources, 'reason': reason, 'detail': detail})
        self.stats[f'held:{reason}'] += 1

    def fuzzy_wiki_matches(self, name_norm):
        """D8 WIKI_SPELLING: every distinct wiki NPC whose normalised name or actualname is
        exactly one edit away from `name_norm`."""
        matches = {}
        for npc in self.wiki_npcs:
            for field in ('name', 'actualname'):
                value = npc.get(field)
                candidate = normalize_name(value) if value else ''
                if candidate and within_one_edit(name_norm, candidate):
                    matches[npc['pageid']] = npc
                    break
        return list(matches.values())

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
                wiki_price = self.wiki_price(name, direction, offer['item_name'], wiki)
                if price is not None and wiki_price is not None and wiki_price != price:
                    arbitration.append({'fact': f'{label}.{direction}', 'rule': 'WIKI_PRICE', 'chosen': 'wiki',
                                        'item_name': offer['item_name'], 'price': wiki_price})
                    price = wiki_price
                if price is not None:
                    offers.append({'item': item, 'source_item_id': key[0], 'direction': direction, 'unit_price': price,
                                   'count': offer['count'], 'sub_type': offer['sub_type']})
        return offers

    def wiki_price(self, npc_name, direction, item_name, fandom):
        """D12: the price both wikis state for this offer, or None when either is silent or they disagree."""
        if not isinstance(item_name, str):
            return None
        row = fandom.get(item_name.lower())
        fandom_price = row and row['buy_price' if direction == 'SellToPlayer' else 'sell_price']
        br_prices = set(self.br_trade.get(fold(npc_name), {}).get(direction, {}).get(fold(item_name), []))
        # BR states one price for the offer only when every row of it gives the same explicit price
        br_price = br_prices.pop() if len(br_prices) == 1 else None
        return fandom_price if fandom_price is not None and fandom_price == br_price else None

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
        rejected = next((OWNER_REJECTED[b['key']] for b in bundles.values() if b['key'] in OWNER_REJECTED), None)
        if rejected is not None:
            return self.hold(name, sources, 'OWNER_REJECTED', rejected)
        removed = next((REMOVED_FROM_GAME[b['key']] for b in bundles.values() if b['key'] in REMOVED_FROM_GAME), None)
        if removed is not None:
            return self.hold(name, sources, 'REMOVED_FROM_GAME', removed)
        name_norm = normalize_name(name)
        wiki = self.wiki.get(name_norm)
        arbitration, left_out = [], []
        if len(bundles) == 1 and wiki is None:
            variant = base_name(name)
            wiki = self.wiki.get(normalize_name(variant)) if variant else None
            if wiki is not None:
                arbitration.append({'fact': 'identity', 'rule': 'WIKI_BASE_NAME', 'chosen': 'wiki'})
            elif len(name_norm) >= SPELLING_MIN_LENGTH:
                fuzzy = self.fuzzy_wiki_matches(name_norm)
                if len(fuzzy) == 1:
                    wiki = fuzzy[0]
                    arbitration.append({'fact': 'identity', 'rule': 'WIKI_SPELLING', 'chosen': 'wiki'})
            if wiki is None:
                return self.hold(name, sources, 'SINGLE_SOURCE_NOT_ON_WIKI')
        definition, conflicts = self.merge_definition(bundles, arbitration)
        if conflicts:
            return self.hold(name, sources, 'DEFINITION_CONFLICT', ','.join(conflicts))
        placements, problem = self.merge_placements(bundles, wiki, arbitration)
        if problem == 'PLACEMENT_CONFLICT_WIKI_UNDECIDED':
            # D6/D8: both sources disagree with each other and with the wiki; the wiki position
            # wins outright over either source (merge_placements only returns this problem when
            # wiki['position'] is set, so the fallback always succeeds here).
            placements = [wiki_position_placement(wiki)]
            arbitration.append({'fact': 'placements', 'rule': 'WIKI_POSITION', 'chosen': 'wiki'})
        elif problem == 'PLACEMENT_CONFLICT_NO_WIKI_POSITION' and wiki is not None:
            # D8 WIKI_CONFIRMED: the sources conflict and the wiki has no position either, but the
            # wiki still confirms the NPC exists; the definition is admitted with no placements.
            placements = []
            arbitration.append({'fact': 'placements', 'rule': 'WIKI_CONFIRMED', 'chosen': 'wiki'})
        elif problem:
            return self.hold(name, sources, problem)
        elif not placements:
            fallback = wiki_position_placement(wiki)
            if fallback is not None:
                placements = [fallback]
                arbitration.append({'fact': 'placements', 'rule': 'WIKI_POSITION', 'chosen': 'wiki'})
            elif wiki is not None:
                # D8 WIKI_CONFIRMED: a wiki page exists (e.g. a seasonal/roaming NPC with no fixed
                # position) but has no position to promote either; the definition is still admitted,
                # with no placements at all.
                arbitration.append({'fact': 'placements', 'rule': 'WIKI_CONFIRMED', 'chosen': 'wiki'})
            else:
                return self.hold(name, sources, 'UNPLACED')
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
    parser.add_argument('--br-facts', help='committed TibiaWiki BR NPC facts (D12 wiki prices)')
    parser.add_argument('--out', required=True)
    args = parser.parse_args()
    snapshot_bytes = Path(args.snapshot).read_bytes()
    item_map_bytes = Path(args.item_map).read_bytes()
    item_map = json.loads(item_map_bytes)
    if item_map['schema'] != 'OTERYN_PROTECTED_ITEM_IDENTITY_MAP_EXPORT/v1':
        raise SystemExit('unexpected item map schema')
    br_facts_bytes = Path(args.br_facts).read_bytes() if args.br_facts else None
    builder = Builder(json.loads(snapshot_bytes), item_map, json.loads(br_facts_bytes) if br_facts_bytes else None)
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
        'schema': SCHEMA, 'evidence': 'OTS_HYPOTHESIS_ONLY',
        'decisions': ['D4', 'D5', 'D6', 'D7', 'D8', 'D11'] + (['D12'] if br_facts_bytes else []),
        'snapshot_sha256': hashlib.sha256(snapshot_bytes).hexdigest(),
        'item_map_sha256': hashlib.sha256(item_map_bytes).hexdigest(),
        **({'br_facts_sha256': hashlib.sha256(br_facts_bytes).hexdigest()} if br_facts_bytes else {}),
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
