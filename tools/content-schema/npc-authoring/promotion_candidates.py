"""Build NPC promotion candidates: merged Canary+Crystal facts with native keys (owner decisions D4-D6).

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


def slug(name):
    folded = unicodedata.normalize('NFKD', name).encode('ascii', 'ignore').decode()
    return re.sub(r'[^a-z0-9]+', '_', folded.lower()).strip('_')


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


class Builder:
    def __init__(self, snapshot):
        self.wiki = {normalize_name(npc['title']): npc for npc in snapshot['npcs']}
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

    def candidate(self, bundles):
        name = next(b['definition']['name'] for b in bundles.values())
        sources = {s: b['key'] for s, b in bundles.items()}
        wiki = self.wiki.get(normalize_name(name))
        if len(bundles) == 1 and wiki is None:
            return self.hold(name, sources, 'SINGLE_SOURCE_NOT_ON_WIKI')
        arbitration, left_out = [], []
        definition, conflicts = self.merge_definition(bundles, arbitration)
        if conflicts:
            return self.hold(name, sources, 'DEFINITION_CONFLICT', ','.join(conflicts))
        placements, problem = self.merge_placements(bundles, wiki, arbitration)
        if problem:
            return self.hold(name, sources, problem)
        if not placements:
            return self.hold(name, sources, 'UNPLACED')
        key_slug = slug(name)
        if not key_slug:  # a punctuation-only name has no slug; it needs a hand-chosen key
            return self.hold(name, sources, 'EMPTY_SLUG')
        travel = self.merge_routes(bundles, wiki, arbitration, left_out)
        record = {
            'identity': {'family': 'NPC', 'key': f'oteryn:npc.{key_slug}', 'revision': 'definition-r1'},
            'name': name, 'profession': definition['profession'],
            'presentation': {'outfit': definition['outfit'], 'speech_bubble': definition['speech_bubble']},
            'movement': definition['movement'],
            'placements': placements,
            'travel_service': ({'identity': {'family': 'Service', 'key': f'oteryn:service.travel.{key_slug}',
                                             'revision': 'definition-r1'}, 'routes': travel} if travel else None),
            'provenance': {s: {'key': b['key'], 'sha256': b['source']['sha256']} for s, b in sorted(bundles.items())},
            'wiki': {'pageid': wiki['pageid'], 'revid': wiki['revid']} if wiki else None,
            'arbitration': [a for a in arbitration if a['rule'] == 'WIKI_ARBITER'],
            'left_out': left_out,
        }
        self.stats['arbitrated_facts'] += len(record['arbitration'])
        self.stats['routes'] += len(travel)
        self.stats['left_out_routes'] += len(left_out)
        return record


def main():
    parser = argparse.ArgumentParser(description=__doc__, formatter_class=argparse.RawDescriptionHelpFormatter)
    parser.add_argument('--canary', required=True)
    parser.add_argument('--crystal', required=True)
    parser.add_argument('--snapshot', required=True)
    parser.add_argument('--out', required=True)
    args = parser.parse_args()
    snapshot_bytes = Path(args.snapshot).read_bytes()
    builder = Builder(json.loads(snapshot_bytes))
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
    report = {
        'schema': SCHEMA, 'evidence': 'OTS_HYPOTHESIS_ONLY', 'decisions': ['D4', 'D5', 'D6'],
        'snapshot_sha256': hashlib.sha256(snapshot_bytes).hexdigest(),
        'totals': {'candidates': len(promoted), 'with_travel': sum(1 for r in promoted if r['travel_service']),
                   **dict(sorted(builder.stats.items()))},
        'candidates': promoted,
        'held': sorted(builder.held, key=lambda h: (h['reason'], h['name'])),
    }
    Path(args.out).write_text(json.dumps(report, indent=1, sort_keys=True) + '\n', encoding='utf-8')
    print(json.dumps(report['totals'], indent=1))


if __name__ == '__main__':
    main()
