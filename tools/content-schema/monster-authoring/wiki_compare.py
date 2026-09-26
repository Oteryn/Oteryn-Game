"""Compare converted Canary monster bundles with TibiaWiki (Fandom) at the 2026-07-28 reference cut.

Evidence tooling only. For each monster page it records the last revision at or before the cut and
the current revision (to expose post-cut edits), the SHA-256 of the cut revision's wikitext and
only the compared infobox facts. No wiki prose is stored. Wiki values are player observations and
never become Game truth by this comparison.

Usage: python wiki_compare.py --canary <Canary checkout> --batch canary-47dfd51f [--cache DIR]
"""
import argparse
import hashlib
import json
import re
import time
import urllib.parse
import urllib.request
from datetime import datetime, timezone
from fractions import Fraction
from pathlib import Path

import canary_batch as cb

ROOT = Path(__file__).resolve().parent
API = 'https://tibia.fandom.com/api.php'
PAGE_URL = 'https://tibia.fandom.com/wiki/'
USER_AGENT = 'OterynEvidenceCollector/0.1 (+https://github.com/Oteryn/Oteryn-Game)'
TARGET_CUT = '2026-07-28'
CUT_TIMESTAMP = '2026-07-29T00:00:00Z'
ELEMENTS = {'physical': 'physicalDmgMod', 'earth': 'earthDmgMod', 'fire': 'fireDmgMod', 'death': 'deathDmgMod',
            'energy': 'energyDmgMod', 'holy': 'holyDmgMod', 'ice': 'iceDmgMod', 'life_drain': 'hpDrainDmgMod',
            'drowning': 'drownDmgMod'}


def api(params):
    query = urllib.parse.urlencode({**params, 'format': 'json', 'formatversion': '2'})
    request = urllib.request.Request(API + '?' + query, headers={'User-Agent': USER_AGENT})
    with urllib.request.urlopen(request, timeout=30) as response:
        return json.load(response)


def revision(title, start=None):
    params = {'action': 'query', 'titles': title, 'prop': 'revisions', 'rvprop': 'ids|timestamp|content',
              'rvslots': 'main', 'rvlimit': 1, 'redirects': 1}
    if start:
        params.update(rvstart=start, rvdir='older')
    page = api(params)['query']['pages'][0]
    if page.get('missing') or not page.get('revisions'):
        return None
    rev = page['revisions'][0]
    return {'page_id': page['pageid'], 'title': page['title'], 'revision_id': rev['revid'],
            'revision_timestamp': rev['timestamp'], 'content': rev['slots']['main']['content']}


def fetch(title, cache):
    path = cache / (cb.slug(title) + '.json')
    if path.exists():
        return json.loads(path.read_text(encoding='utf-8'))
    record = {'cut': revision(title, CUT_TIMESTAMP), 'current': revision(title),
              'retrieved_at': datetime.now(timezone.utc).strftime('%Y-%m-%dT%H:%M:%SZ')}
    path.write_text(json.dumps(record, ensure_ascii=False), encoding='utf-8')
    time.sleep(0.5)
    return record


def infobox(text):
    """Top-level `| key = value` fields of the first Infobox Creature, honouring nested {{ }} and [[ ]]."""
    start = text.find('{{Infobox Creature')
    if start < 0:
        return {}
    depth, i, fields, current = 0, start, {}, None
    buffer = []
    while i < len(text):
        pair = text[i:i + 2]
        if pair in ('{{', '[['):
            depth += 1
            if depth > 1:
                buffer.append(pair)
            i += 2
            continue
        if pair in ('}}', ']]'):
            depth -= 1
            if depth == 0:
                break
            buffer.append(pair)
            i += 2
            continue
        if text[i] == '|' and depth == 1:
            if current is not None:
                fields[current] = ''.join(buffer).strip()
            buffer, current = [], None
            j = text.find('=', i)
            nxt = min(k for k in (text.find('|', i + 1), text.find('}}', i + 1), len(text)) if k >= 0)
            if 0 <= j < nxt:
                current = text[i + 1:j].strip()
                i = j + 1
                continue
        else:
            buffer.append(text[i])
        i += 1
    if current is not None:
        fields[current] = ''.join(buffer).strip()
    return fields


def number(value):
    match = re.match(r'^\s*(-?\d+(?:\.\d+)?)', value or '')
    return Fraction(match.group(1)) if match else None


def wiki_loot(value):
    names = set()
    for item in re.findall(r'\{\{Loot Item\|([^{}]*)\}\}', value or ''):
        parts = [p.strip() for p in item.split('|') if p.strip()]
        candidates = [p for p in parts if not re.fullmatch(r'[\d\-]+', p)]
        if candidates:
            names.add(candidates[0].lower())
    return names


def compare(relative, canary, batch_dir, cache):
    name, source, _ = cb.load_monster(canary / cb.MONSTER_DIR / (relative + '.lua'))
    slug = cb.slug(name)
    monster = json.loads((batch_dir / slug / 'monster.json').read_text(encoding='utf-8'))
    record = fetch(name, cache)
    result = {'monster': slug, 'wiki_title': name, 'page_url': PAGE_URL + urllib.parse.quote(name.replace(' ', '_'))}
    if not record['cut']:
        return {**result, 'status': 'WIKI_PAGE_MISSING', 'rows': []}
    cut, current = record['cut'], record['current']
    fields = infobox(cut['content'])
    result.update({'page_id': cut['page_id'], 'cut_revision_id': cut['revision_id'], 'cut_revision_timestamp': cut['revision_timestamp'],
                   'cut_content_sha256': hashlib.sha256(cut['content'].encode('utf-8')).hexdigest(),
                   'current_revision_id': current['revision_id'], 'current_revision_timestamp': current['revision_timestamp'],
                   'edited_after_cut': current['revision_id'] != cut['revision_id'], 'retrieved_at': record['retrieved_at']})
    c, b = monster['creature'], monster['behavior']
    rows = []

    def row(field, canary_value, wiki_raw, wiki_value, note=None):
        if wiki_raw in (None, '', '?'):
            status = 'WIKI_UNKNOWN'
        elif canary_value == wiki_value:
            status = 'MATCH'
        else:
            status = 'DIFF'
        entry = {'field': field, 'canary': canary_value, 'wiki': wiki_value if wiki_value is not None else wiki_raw, 'wiki_raw': wiki_raw, 'status': status}
        if note:
            entry['note'] = note
        rows.append(entry)

    def as_number(value):
        return None if value is None else (int(value) if value.denominator == 1 else float(value))

    stats = c['stats']
    for field, key, value in (('max_health', 'hp', stats['max_health']), ('experience', 'exp', stats['experience']),
                              ('armor', 'armor', stats['armor']), ('speed', 'speed', stats['speed'])):
        row(field, value, fields.get(key), as_number(number(fields.get(key))),
            'Canary monster.speed is the raw engine value; the wiki lists observed speed.' if field == 'speed' else None)
    mitigation = stats.get('mitigation_percent')
    row('mitigation_percent', float(Fraction(mitigation['numerator'], mitigation['denominator'])) if mitigation else None,
        fields.get('mitigation'), as_number(number(fields.get('mitigation'))))
    resist = {r['damage_type']: Fraction(r['reduction_percent']['numerator'], r['reduction_percent']['denominator']) for r in c['resistances']}
    for element, key in ELEMENTS.items():
        taken = number(fields.get(key))
        row(f'resistance.{element}', as_number(resist.get(element, Fraction(0))), fields.get(key),
            as_number(100 - taken) if taken is not None else None, 'Wiki lists damage taken; resistance = 100 - taken.')
    summoning = c['summoning']
    for field, key, flag in (('summon_mana_cost', 'summon', 'summonable'), ('convince_mana_cost', 'convince', 'convinceable')):
        raw = fields.get(key)
        wiki_value = as_number(number(raw)) if number(raw) is not None else ('--' if raw and raw.strip() in ('--', '-') else None)
        row(field, summoning.get('mana_cost') if summoning[flag] else '--', raw, wiki_value)
    for field, key, value in (('illusionable', 'illusionable', c['flags']['illusionable']),
                              ('pushable', 'pushable', b['movement']['pushable']),
                              ('push_items', 'pushobjects', b['movement']['push_items']),
                              ('sense_invisible', 'senseinvis', b['targeting']['sense_invisible']),
                              ('paralyze_immune', 'paraimmune', 'paralyze' in c['immunities']['conditions'])):
        raw = fields.get(key)
        row(field, value, raw, {'yes': True, 'no': False}.get((raw or '').strip().lower()))
    row('flee_health', b['targeting']['flee_health'], fields.get('runsat'), as_number(number(fields.get('runsat'))))
    bestiary = c.get('bestiary', {})
    for field, key in (('bestiary.class', 'bestiaryclass'), ('bestiary.difficulty', 'bestiarylevel'), ('bestiary.occurrence', 'occurrence')):
        canary_value = bestiary.get(field.split('.')[1])
        raw = fields.get(key)
        wiki_value = raw.strip().lower().replace(' ', '_') if raw else None
        row(field, canary_value.lower() if isinstance(canary_value, str) else canary_value, raw, wiki_value)
    canary_loot = set()
    for entry in source.get('loot', []):
        item_name = entry.get('name') or cb.CONVERTER.names.get(int(entry['id']), f'item {entry["id"]}')
        canary_loot.add(item_name.lower())
    wiki_items = wiki_loot(fields.get('loot'))
    if wiki_items or canary_loot:
        only_canary, only_wiki = canary_loot - wiki_items, wiki_items - canary_loot
        variants = sorted((c_name, w_name) for c_name in only_canary for w_name in only_wiki
                          if re.sub(r'\s*\(.*\)$', '', w_name) == c_name)
        only_canary -= {c_name for c_name, _ in variants}
        only_wiki -= {w_name for _, w_name in variants}
        entry = {'field': 'loot.items', 'status': 'DIFF' if only_canary or only_wiki else 'MATCH',
                 'only_canary': sorted(only_canary), 'only_wiki': sorted(only_wiki),
                 'note': 'Item names only; wiki loot chances are player-reported rarity words and are not compared.'}
        if variants:
            entry['name_variants'] = [{'canary': c_name, 'wiki': w_name} for c_name, w_name in variants]
            entry['note'] += ' Wiki disambiguated names (e.g. "book (grey)") are matched to the Canary base name.'
        rows.append(entry)
    result.update({'status': 'COMPARED', 'rows': rows})
    return result


def main():
    parser = argparse.ArgumentParser(description=__doc__.split('\n')[0])
    parser.add_argument('--canary', required=True, type=Path)
    parser.add_argument('--batch', default=cb.REV, choices=sorted(cb.BATCHES))
    parser.add_argument('--cache', type=Path, default=Path('/tmp/oteryn-wiki-cache'))
    args = parser.parse_args()
    args.cache.mkdir(parents=True, exist_ok=True)
    objects = cb.load_appearance_objects(args.canary / 'data/items/appearances.dat')
    items = cb.load_items_xml(args.canary / 'data/items/items.xml')
    names, index = cb.name_index(objects, items)
    cb.CONVERTER = cb.Converter(args.canary, objects, items, names, index)
    batch_dir = ROOT / 'samples' / args.batch
    results = [compare(relative, args.canary, batch_dir, args.cache) for relative in cb.BATCHES[args.batch]]
    summary = {}
    for result in results:
        for entry in result['rows']:
            summary[entry['status']] = summary.get(entry['status'], 0) + 1
    report = {'source': 'TibiaWiki (Fandom), CC BY-SA; only compared facts are recorded', 'api': API,
              'target_cut': TARGET_CUT, 'cut_rule': f'last revision at or before {CUT_TIMESTAMP}',
              'classification': 'Wiki = player-observed reference evidence; Canary = OTS_HYPOTHESIS_ONLY',
              'row_status_totals': dict(sorted(summary.items())), 'monsters': results}
    out = batch_dir / 'wiki-2026-07-28.json'
    out.write_text(json.dumps(report, ensure_ascii=False, indent=2) + '\n', encoding='utf-8', newline='\n')
    print(json.dumps({'out': str(out), 'totals': report['row_status_totals']}))


if __name__ == '__main__':
    main()
