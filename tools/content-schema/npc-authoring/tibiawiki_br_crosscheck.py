"""Cross-check every admitted NPC against the committed TibiaWiki BR NPC facts.

Evidence only: reports where the admitted NPC facts agree or disagree with TibiaWiki BR. Nothing is
changed, admitted or held by this report.

Per admitted NPC (the NPC declarations of the pinned admission evidence), matched to a BR page by name
(case-insensitive page title, then the infobox `name`; a name variant such as `Name (1)`, `Name (Day)` or
`Name Init` falls back to its base name's page, and a plain name also matches a BR page whose title or
name carries a qualifier, such as `Name (NPC)`, when exactly one does). A name that still has no page
matches an alias only when exactly one BR page is both a close name (an article dropped, at most
ALIAS_EDITS character edits, or one name a whole-word prefix of the other) and placed within CLOSE_TILES
of an admitted placement on the same floor; the row then records `match: POSITION_ALIAS`:
- removed: the BR infobox `removed` version, when BR says the NPC left the game;
- position: the nearest admitted placement to any BR map position. MATCH is the same tile, NEAR the
  same floor within NEAR_TILES tiles, CLOSE the same floor within CLOSE_TILES, OTHER_FLOOR within
  CLOSE_TILES on another floor, otherwise MISMATCH (BR map markers are approximate);
- trade: admitted offers against the BR sell/buy lists by item name (a BR name with a parenthesised
  qualifier also matches the plain name, and a filled fluid container such as `vial of blood` matches
  the container), with the explicit BR price when BR gives one (BR omits the
  price when it is the item's usual price). BR_ONLY is an NPC whose BR trade list has no admitted
  offers at all;
- dialogue: how many admitted Dialogue texts appear, as a full line, among the lines the NPC speaks in
  its BR transcript, with the same normalization and |PLAYERNAME| wildcard as rule D10 and the in-game
  `{keyword}` highlight braces removed (transcripts show plain text). A text that does not match but is
  at least NEAR_RATIO similar to a transcript line is counted as near (a small wording difference).

Inputs are committed files only, so the report is reproducible byte for byte:
    python tibiawiki_br_crosscheck.py --out samples/tibiawiki-br-crosscheck-v1.json
"""
import argparse
import hashlib
import json
import re
import sys
from collections import Counter
from difflib import SequenceMatcher
from pathlib import Path

ROOT = Path(__file__).resolve().parents[3]
FACTS = ROOT / 'imports/tibiawiki/npc-br/2026-09-28/tibiawiki-br-npc-facts.json'
CANDIDATES = ROOT / 'tools/content-schema/npc-authoring/samples/promotion-candidates-v1.json'
DIALOGUES = ROOT / 'docs/agents/evidence/OTV2-20260928-npc-dialogue-wave-a-staged.json'
ADMISSION = ROOT / 'docs/agents/evidence/OTV2-20260927-npc-admission-wave-a-staged.json'
ITEMS = ROOT / 'content/items/definitions'
SCHEMA = 'OTERYN_NPC_TIBIAWIKI_BR_CROSSCHECK/v1'
NEAR_TILES = 3
CLOSE_TILES = 10
NEAR_RATIO = 0.9
VARIANT = re.compile(r'^(.*?)(?: \([^()]*\)| Init| Vampires Lair| Back)$')
QUALIFIER = re.compile(r'^(.*?) \([^()]*\)$')
CONTAINER = re.compile(r'^(.*?) of .+$')  # `vial of blood`: the admitted offer is the container, its fluid a sub type
FLUID_CONTAINERS = {'vial', 'mug', 'cup', 'bottle', 'flask', 'green flask', 'rum flask', 'bucket', 'jug', 'bowl',
                    'waterskin', 'amphora', 'large amphora', 'pitcher', 'goblet', 'elven vase'}
ALIAS_EDITS = 2  # at most this many single-character edits between an unmatched name and a BR name


def normalize(text):
    return re.sub(r'\s+', ' ', text).strip().casefold()


def br_index(facts):
    by_title, by_name, by_base = {}, {}, {}
    for page in facts['pages']:
        by_title.setdefault(normalize(page['title']), page)
        by_name.setdefault(normalize(page['name']), []).append(page)
        for label in {page['title'], page['name']}:
            qualified = QUALIFIER.match(label)
            if qualified:
                by_base.setdefault(normalize(qualified.group(1)), set()).add(page['pageid'])
    pages = {page['pageid']: page for page in facts['pages']}
    by_base = {key: pages[next(iter(ids))] for key, ids in by_base.items() if len(ids) == 1}
    return by_title, by_name, by_base


def edit_distance(a, b):
    previous = list(range(len(b) + 1))
    for i, ca in enumerate(a, 1):
        current = [i]
        for j, cb in enumerate(b, 1):
            current.append(min(previous[j] + 1, current[j - 1] + 1, previous[j - 1] + (ca != cb)))
        previous = current
    return previous[-1]


def close_names(a, b):
    a, b = (re.sub(r'^(?:a|an|the) ', '', x) for x in (a, b))
    if a == b or a.startswith(b + ' ') or b.startswith(a + ' '):
        return True
    return min(len(a), len(b)) >= 8 and edit_distance(a, b) <= ALIAS_EDITS


def alias_page(name, placements, facts):
    key = normalize(name)
    matches = []
    for page in facts['pages']:
        if not any(close_names(key, normalize(label)) for label in {page['title'], page['name']}):
            continue
        near = any(placement['position']['z'] == z
                   and max(abs(placement['position']['x'] - x), abs(placement['position']['y'] - y)) <= CLOSE_TILES
                   for placement in placements for x, y, z in page['positions'])
        if near:
            matches.append(page)
    return matches[0] if len(matches) == 1 else None


def find_page(name, index):
    by_title, by_name, by_base = index
    key = normalize(name)
    if key in by_title:
        return by_title[key]
    pages = by_name.get(key, [])
    if len(pages) == 1:
        return pages[0]
    variant = VARIANT.match(name)
    if variant:
        return find_page(variant.group(1), index)
    return by_base.get(key)


def plain(text):
    return normalize(text.replace('{', '').replace('}', ''))


def text_matches(text, lines):
    pattern = '.+?'.join(re.escape(part) for part in plain(text).split('|playername|'))
    regex = re.compile(pattern)
    return any(regex.fullmatch(line) for line in lines)


def text_near(text, lines):
    ours = plain(text).replace('|playername|', '')
    return any(SequenceMatcher(None, ours, line, autojunk=False).ratio() >= NEAR_RATIO for line in lines)


def dialogue_texts(declaration):
    texts = []
    for key in ('greet', 'farewell', 'walkaway', 'send_trade'):
        texts.extend(declaration.get(key) or [])

    def walk(nodes):
        for node in nodes:
            texts.extend(node.get('reply') or [])
            walk(node.get('children') or [])

    walk(declaration.get('keywords') or [])
    return texts


def item_names():
    names = {}
    for path in sorted(ITEMS.glob('items-*.json')):
        for record in json.loads(path.read_text(encoding='utf-8'))['records']:
            definition = record['definition']
            name = definition.get('semantics', {}).get('presentation', {})
            if name.get('state') == 'KNOWN' and name['value']['name'].get('state') == 'KNOWN':
                names[definition['identity']['key']] = normalize(name['value']['name']['value'])
    return names


def check_position(placements, positions):
    if not positions:
        return {'status': 'NO_BR_POSITION'}
    if not placements:
        return {'status': 'NO_PLACEMENT', 'br': [list(p) for p in positions]}
    best = None
    for placement in placements:
        ours = placement['position']
        for x, y, z in positions:
            distance = max(abs(ours['x'] - x), abs(ours['y'] - y))
            rank = (abs(ours['z'] - z), distance)
            if best is None or rank < best[0]:
                best = (rank, [ours['x'], ours['y'], ours['z']], [x, y, z])
    (floors, distance), ours, theirs = best
    if floors == 0 and distance <= CLOSE_TILES:
        status = 'MATCH' if distance == 0 else 'NEAR' if distance <= NEAR_TILES else 'CLOSE'
    else:
        status = 'OTHER_FLOOR' if distance <= CLOSE_TILES else 'MISMATCH'
    row = {'status': status}
    if status != 'MATCH':
        row.update({'ours': ours, 'br': theirs})
    return row


def check_trade(offers, trades, names):
    has_br = any(trades.values())
    if not offers and not has_br:
        return None
    if not has_br:
        return {'status': 'NO_BR_TRADE', 'offers': len(offers)}
    if not offers:
        return {'status': 'BR_ONLY', 'br_offers': sum(len(names) for names in trades.values())}
    # several offers can share one item name (four music sheets); both sides keep every offer
    ours = {'SellToPlayer': {}, 'BuyFromPlayer': {}}
    uncomparable = []
    for offer in offers:
        name = names.get(offer['item']['key'])
        if name is None:  # an Item without a known name cannot be compared by name; it is reported, not dropped
            uncomparable.append(f'{offer["direction"]}:{offer["item"]["key"]}')
            continue
        ours[offer['direction']].setdefault(name, []).append(offer['unit_price'])
    row = {'matched': 0, 'only_ours': [], 'only_br': [], 'price_mismatch': []}
    for direction in ('SellToPlayer', 'BuyFromPlayer'):
        mine, theirs = ours[direction], {}
        for name, price in trades[direction].items():
            for form in (QUALIFIER, CONTAINER):
                base = form.match(name)
                if (name not in mine and base and base.group(1) in mine
                        and (form is QUALIFIER or base.group(1) in FLUID_CONTAINERS)):
                    name = base.group(1)
            theirs.setdefault(name, []).append(price)
        for name in sorted(set(mine) | set(theirs)):
            label = f'{direction}:{name}'
            mine_prices, br_prices = sorted(mine.get(name, [])), theirs.get(name, [])
            row['matched'] += min(len(mine_prices), len(br_prices))
            row['only_ours'].extend([label] * max(0, len(mine_prices) - len(br_prices)))
            row['only_br'].extend([label] * max(0, len(br_prices) - len(mine_prices)))
            if mine_prices:
                unmatched = list(mine_prices)
                explicit = sorted(price for price in br_prices if price is not None)
                missing = []
                for price in explicit:
                    if price in unmatched:
                        unmatched.remove(price)
                    else:
                        missing.append(price)
                if missing:
                    row['price_mismatch'].append({'offer': label, 'ours': mine_prices, 'br': explicit})
    row['uncomparable'] = sorted(uncomparable)
    if row['only_ours'] or row['only_br'] or row['price_mismatch']:
        row['status'] = 'DIFFER'
    else:
        row['status'] = 'INCOMPLETE' if uncomparable else 'AGREE'
    return {key: value for key, value in row.items() if value not in ([], 0) or key in ('status', 'matched')}


def crosscheck(facts, candidates, dialogues, admission, names):
    index = br_index(facts)
    admitted = {d['identity']['key'] for d in admission['declarations'] if d['kind'] == 'NPC'}
    dialogue_by_npc = {entry['npc']: entry['declaration'] for entry in dialogues['dialogues']}
    rows, totals = [], Counter()
    for candidate in sorted(candidates['candidates'], key=lambda c: c['identity']['key']):
        key, name = candidate['identity']['key'], candidate['name']
        if key not in admitted:
            continue
        page = find_page(name, index)
        row = {'npc': key, 'name': name}
        if page is None:
            page = alias_page(name, candidate['placements'], facts)
            if page is not None:
                row['match'] = 'POSITION_ALIAS'
                totals['matched_by_position_alias'] += 1
        if page is None:
            row['br'] = None
            totals['no_br_page'] += 1
            rows.append(row)
            continue
        row['br'] = {'pageid': page['pageid'], 'revid': page['revid'], 'title': page['title']}
        if page['removed']:
            row['removed'] = page['removed']
            totals['removed_on_br'] += 1
        row['position'] = check_position(candidate['placements'], [tuple(p) for p in page['positions']])
        totals[f'position:{row["position"]["status"]}'] += 1
        trades = {direction: {normalize(item): price for item, price in items.items()}
                  for direction, items in page['trades'].items()}
        trade = check_trade((candidate['trade_service'] or {}).get('offers', []), trades, names)
        if trade is not None:
            row['trade'] = trade
            totals[f'trade:{trade["status"]}'] += 1
        lines = [normalize(line) for line in page['npc_lines']]
        declaration = dialogue_by_npc.get(key)
        if declaration is not None and lines:
            texts = dialogue_texts(declaration)
            matched = [text for text in texts if text_matches(text, lines)]
            near = sum(1 for text in texts if text not in matched and text_near(text, lines))
            row['dialogue'] = {'texts': len(texts), 'matched': len(matched), 'near': near, 'br_lines': len(lines)}
            totals['dialogue:checked'] += 1
            totals['dialogue:texts'] += len(texts)
            totals['dialogue:matched'] += len(matched)
            totals['dialogue:near'] += near
            if texts and not matched and not near:
                totals['dialogue:no_text_matched'] += 1
        elif lines:
            totals['dialogue:br_only'] += 1
        rows.append(row)
    totals['npcs'] = len(rows)
    return {
        'schema': SCHEMA, 'evidence': 'REFERENCE_CROSSCHECK_ONLY',
        'inputs': {
            'snapshot_pages_digest': facts['snapshot_pages_digest'],
            'facts_sha256': hashlib.sha256(FACTS.read_bytes()).hexdigest(),
            'candidates_sha256': hashlib.sha256(CANDIDATES.read_bytes()).hexdigest(),
            'dialogues_sha256': hashlib.sha256(DIALOGUES.read_bytes()).hexdigest(),
            'admission_sha256': hashlib.sha256(ADMISSION.read_bytes()).hexdigest(),
        },
        'near_tiles': NEAR_TILES, 'close_tiles': CLOSE_TILES, 'near_ratio': NEAR_RATIO,
        'totals': dict(sorted(totals.items())),
        'npcs': rows,
    }


def main():
    parser = argparse.ArgumentParser(description=__doc__.split('\n')[0])
    parser.add_argument('--out', type=Path, required=True)
    args = parser.parse_args()
    report = crosscheck(json.loads(FACTS.read_text(encoding='utf-8')),
                        json.loads(CANDIDATES.read_text(encoding='utf-8')),
                        json.loads(DIALOGUES.read_text(encoding='utf-8')),
                        json.loads(ADMISSION.read_text(encoding='utf-8')), item_names())
    args.out.write_text(json.dumps(report, indent=1, sort_keys=True, ensure_ascii=False) + '\n', encoding='utf-8')
    print(json.dumps(report['totals'], indent=1))
    return 0


if __name__ == '__main__':
    sys.exit(main())
