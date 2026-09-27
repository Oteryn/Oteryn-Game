"""Compare Canary and Crystal candidate NPC bundles field by field.

Owner decision D2 (docs/architecture/OTERYN_NPC_AUTHORING_SCHEMA_V1.md): Canary and Crystal are equal
sources. No source wins a conflict; every CONFLICT or one-sided row stays an open decision.

NPCs are paired by source file stem, then by registered name. Each section is reduced to keyed facts
(text by SHA-256 only) and every fact gets one outcome: SAME, CONFLICT, CANARY_ONLY or CRYSTAL_ONLY.

Usage: python source_diff.py --canary <out/canary/bundles> --crystal <out/crystal/bundles> --out report.json
"""
import argparse
import json
from collections import Counter
from pathlib import Path

SCHEMA = 'OTERYN_NPC_SOURCE_DIFF/v1'
SECTIONS = ('definition', 'voices', 'messages', 'keywords', 'trade', 'travel', 'spells', 'blessings', 'promotion',
            'kick', 'placements')


def sha(ref):
    if ref is None:
        return None
    if 'parts' in ref:
        return [part and part['sha256'] for part in ref['parts']]
    return ref['sha256']


def keyword_facts(nodes, trail=()):
    facts = {}
    for node in nodes:
        path = trail + ('|'.join(node['keywords']),)
        key = ' > '.join(path)
        fact = (node['kind'], json.dumps(sha(node['text'])), node['gate'], node['effect'], json.dumps(node['flags'], sort_keys=True))
        # alias keywords repeat a node; the first definition is the one the engine matches first
        facts.setdefault(key, fact)
        facts.update({k: v for k, v in keyword_facts(node['children'], path).items() if k not in facts})
    return facts


def facts(bundle):
    d = bundle['definition']
    services = bundle['services']
    trade = services['trade'] or {'offers': [], 'currency': None}
    result = {
        'definition': {
            'name': d['name'], 'profession': d['profession'], 'description': json.dumps(sha(d['description'])),
            'outfit': json.dumps(d['presentation']['outfit'], sort_keys=True), 'speech_bubble': d['presentation']['speech_bubble'],
            'movement': json.dumps(d['movement'], sort_keys=True), 'vitals': json.dumps(d['vitals'], sort_keys=True),
            'currency': json.dumps(trade['currency']),
        },
        'voices': {},
        'messages': {label: json.dumps(sha(ref)) for label, ref in bundle['dialogue']['messages'].items()},
        'keywords': keyword_facts(bundle['dialogue']['keywords']),
        'trade': {}, 'travel': {}, 'spells': {}, 'blessings': {}, 'promotion': {}, 'kick': {},
        'placements': {json.dumps(p['position'], sort_keys=True): (p['direction'], p['spawn_interval_s'])
                       for p in bundle['placements']},
    }
    # a field one source does not declare at all is one-sided, not a conflict
    result['definition'] = {k: v for k, v in result['definition'].items() if v not in (None, 'null')}
    if bundle['voices']:
        result['voices']['cadence'] = (bundle['voices']['interval_ms'], bundle['voices']['chance_percent'])
        for line in bundle['voices']['lines']:
            result['voices'][json.dumps(sha(line['text']))] = line['yell']
    for offer in trade['offers']:
        key = f"{offer['client_id'] if offer['client_id'] is not None else offer['server_item_id']}:{offer['item_name']}"
        if offer['count']:
            key += f":x{offer['count']}"
        if offer['sub_type'] is not None:
            key += f":s{offer['sub_type']}"
        result['trade'].setdefault(key, (offer['buy_price'], offer['sell_price'], json.dumps(offer['stock_gate'], sort_keys=True)))
    for row in services['travel']:
        result['travel'].setdefault(str(row['keyword']), (json.dumps(row['destination'], sort_keys=True), row['price'],
                                                          row['premium'], row['min_level'], row['gate']))
    for row in services['spells']:
        result['spells'].setdefault(str(row['spell_name']), (row['price'], row['min_level'], json.dumps(row['vocations']),
                                                             row['premium']))
    for row in services['blessings']:
        result['blessings'].setdefault(str(row['keyword']), (json.dumps(row['blessing']), json.dumps(row['price'])))
    for row in services['promotion']:
        result['promotion'].setdefault(str(row['keyword']), (row['price'], row['min_level'], row['promotion']))
    for row in services['kick']:
        result['kick'].setdefault(json.dumps(row['destinations'], sort_keys=True), True)
    return result


def compare(left, right):
    counts = Counter()
    conflicts = []
    for key in sorted(set(left) | set(right)):
        if key not in right:
            counts['CANARY_ONLY'] += 1
        elif key not in left:
            counts['CRYSTAL_ONLY'] += 1
        elif left[key] == right[key]:
            counts['SAME'] += 1
        else:
            counts['CONFLICT'] += 1
            conflicts.append(key)
    return counts, conflicts


def load(directory):
    bundles = {}
    for path in sorted(Path(directory).glob('*.json')):
        bundle = json.loads(path.read_text(encoding='utf-8'))
        bundles[path.stem] = bundle
    return bundles


def pair(canary, crystal):
    pairs, used = [], set()
    by_name = {str(b.get('definition', {}).get('name')).lower(): stem for stem, b in crystal.items() if 'definition' in b}
    for stem, bundle in sorted(canary.items()):
        other = stem if stem in crystal else by_name.get(str(bundle.get('definition', {}).get('name')).lower())
        if other and other not in used:
            used.add(other)
            pairs.append((stem, other))
        else:
            pairs.append((stem, None))
    pairs.extend((None, stem) for stem in sorted(crystal) if stem not in used)
    return pairs


def main():
    parser = argparse.ArgumentParser(description=__doc__, formatter_class=argparse.RawDescriptionHelpFormatter)
    parser.add_argument('--canary', required=True)
    parser.add_argument('--crystal', required=True)
    parser.add_argument('--out', required=True)
    args = parser.parse_args()
    canary, crystal = load(args.canary), load(args.crystal)
    rows, totals, npc_outcomes = [], {s: Counter() for s in SECTIONS}, Counter()
    for left_stem, right_stem in pair(canary, crystal):
        left = canary.get(left_stem) if left_stem else None
        right = crystal.get(right_stem) if right_stem else None
        row = {'canary': left and left['key'], 'crystal': right and right['key']}
        if not left or not right or 'definition' not in left or 'definition' not in right:
            outcome = ('CANARY_ONLY_NPC' if not right else 'CRYSTAL_ONLY_NPC' if not left else 'NOT_COMPARABLE')
            row['outcome'] = outcome
            npc_outcomes[outcome] += 1
            rows.append(row)
            continue
        left_facts, right_facts = facts(left), facts(right)
        sections = {}
        for section in SECTIONS:
            counts, conflicts = compare(left_facts[section], right_facts[section])
            totals[section].update(counts)
            if counts['CONFLICT'] or counts['CANARY_ONLY'] or counts['CRYSTAL_ONLY']:
                sections[section] = dict(sorted(counts.items())) | ({'conflict_keys': conflicts[:20]} if section not in
                                                                    ('keywords', 'voices', 'messages') else {})
        # COMPLEMENTARY: only one-sided facts; CONFLICTING: at least one fact differs between the sources
        row['outcome'] = ('IDENTICAL' if not sections else
                          'CONFLICTING' if any(c.get('CONFLICT') for c in sections.values()) else 'COMPLEMENTARY')
        row['sections'] = sections
        npc_outcomes[row['outcome']] += 1
        rows.append(row)
    report = {
        'schema': SCHEMA, 'evidence': 'OTS_HYPOTHESIS_ONLY', 'policy': 'D2_EQUAL_SOURCES_NO_AUTOMATIC_WINNER',
        'canary_bundles': len(canary), 'crystal_bundles': len(crystal),
        'npc_outcomes': dict(sorted(npc_outcomes.items())),
        'section_totals': {s: dict(sorted(c.items())) for s, c in totals.items()},
        'rows': rows,
    }
    Path(args.out).write_text(json.dumps(report, indent=1, sort_keys=True) + '\n', encoding='utf-8')
    print(json.dumps({k: report[k] for k in ('npc_outcomes', 'section_totals')}, indent=1))


if __name__ == '__main__':
    main()
