"""Cross-check every admitted NPC against the committed TibiaWiki BR NPC facts.

Evidence only: reports where the admitted NPC facts agree or disagree with TibiaWiki BR. Nothing is
changed, admitted or held by this report.

Per NPC, matched to a BR page by name (case-insensitive page title, then the infobox `name`; a name
variant such as `Name (1)`, `Name (Day)` or `Name Init` falls back to its base name's page):
- removed: the BR infobox `removed` version, when BR says the NPC left the game;
- position: the nearest admitted placement to any BR map position. MATCH is the same tile, NEAR the
  same floor within NEAR_TILES tiles, CLOSE the same floor within CLOSE_TILES, OTHER_FLOOR within
  CLOSE_TILES on another floor, otherwise MISMATCH (BR map markers are approximate);
- trade: admitted offers against the BR sell/buy lists by item name (a BR name with a parenthesised
  qualifier also matches the plain name), with the explicit BR price when BR gives one (BR omits the
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
ITEMS = ROOT / 'content/items/definitions'
SCHEMA = 'OTERYN_NPC_TIBIAWIKI_BR_CROSSCHECK/v1'
NEAR_TILES = 3
CLOSE_TILES = 10
NEAR_RATIO = 0.9
VARIANT = re.compile(r'^(.*?)(?: \([^()]*\)| Init| Vampires Lair| Back)$')
QUALIFIER = re.compile(r'^(.*?) \([^()]*\)$')


def normalize(text):
    return re.sub(r'\s+', ' ', text).strip().casefold()


def br_index(facts):
    by_title, by_name = {}, {}
    for page in facts['pages']:
        by_title.setdefault(normalize(page['title']), page)
        by_name.setdefault(normalize(page['name']), []).append(page)
    return by_title, by_name


def find_page(name, by_title, by_name):
    key = normalize(name)
    if key in by_title:
        return by_title[key]
    pages = by_name.get(key, [])
    if len(pages) == 1:
        return pages[0]
    variant = VARIANT.match(name)
    return find_page(variant.group(1), by_title, by_name) if variant else None


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
    ours = {'SellToPlayer': {}, 'BuyFromPlayer': {}}
    unnamed = 0
    for offer in offers:
        name = names.get(offer['item']['key'])
        if name is None:
            unnamed += 1
            continue
        ours[offer['direction']].setdefault(name, offer['unit_price'])
    row = {'matched': 0, 'only_ours': [], 'only_br': [], 'price_mismatch': []}
    for direction in ('SellToPlayer', 'BuyFromPlayer'):
        mine, theirs = ours[direction], {}
        for name, price in trades[direction].items():
            qualified = QUALIFIER.match(name)
            if name not in mine and qualified and qualified.group(1) in mine:
                name = qualified.group(1)
            theirs.setdefault(name, price)
        for name in sorted(set(mine) | set(theirs)):
            label = f'{direction}:{name}'
            if name not in theirs:
                row['only_ours'].append(label)
            elif name not in mine:
                row['only_br'].append(label)
            else:
                row['matched'] += 1
                if theirs[name] is not None and theirs[name] != mine[name]:
                    row['price_mismatch'].append({'offer': label, 'ours': mine[name], 'br': theirs[name]})
    if unnamed:
        row['unnamed_offers'] = unnamed
    row['status'] = 'AGREE' if not (row['only_ours'] or row['only_br'] or row['price_mismatch']) else 'DIFFER'
    return {key: value for key, value in row.items() if value not in ([], 0) or key in ('status', 'matched')}


def crosscheck(facts, candidates, dialogues, names):
    by_title, by_name = br_index(facts)
    dialogue_by_npc = {entry['npc']: entry['declaration'] for entry in dialogues['dialogues']}
    rows, totals = [], Counter()
    for candidate in sorted(candidates['candidates'], key=lambda c: c['identity']['key']):
        key, name = candidate['identity']['key'], candidate['name']
        page = find_page(name, by_title, by_name)
        row = {'npc': key, 'name': name}
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
                        json.loads(DIALOGUES.read_text(encoding='utf-8')), item_names())
    args.out.write_text(json.dumps(report, indent=1, sort_keys=True, ensure_ascii=False) + '\n', encoding='utf-8')
    print(json.dumps(report['totals'], indent=1))
    return 0


if __name__ == '__main__':
    sys.exit(main())
