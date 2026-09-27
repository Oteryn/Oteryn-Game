"""Compare converted Canary monster bundles with TibiaWiki (Fandom) at the 2026-09-27 reference cut.

Evidence tooling only. For each monster page it records the last revision at or before the cut and
the current revision (to expose post-cut edits), the SHA-256 of the cut revision's wikitext and
only the compared infobox facts. No wiki prose is stored. Wiki values are player observations and
never become Game truth by this comparison.

Usage: python wiki_compare.py --canary <Canary checkout> --batch canary-47dfd51f [--cache DIR]
"""
import argparse
import hashlib
import json
import math
import re
import time
import urllib.parse
import urllib.request
from datetime import datetime, timezone
from decimal import Decimal
from fractions import Fraction
from pathlib import Path

import canary_batch as cb

ROOT = Path(__file__).resolve().parent
API = 'https://tibia.fandom.com/api.php'
PAGE_URL = 'https://tibia.fandom.com/wiki/'
USER_AGENT = 'OterynEvidenceCollector/0.1 (+https://github.com/Oteryn/Oteryn-Game)'
TARGET_CUT = '2026-09-27'
LOW_CONFIDENCE_DROPS = 10
CUT_TIMESTAMP = '2026-09-28T00:00:00Z'
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


def cut_cache(cache):
    """The cache directory of the current target date: a record is valid only for the cut it was fetched at."""
    directory = cache / TARGET_CUT
    directory.mkdir(parents=True, exist_ok=True)
    return directory


def fetch(title, cache):
    path = cut_cache(cache) / (cb.slug(title) + '.json')
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


def field_line(content, key):
    """1-based wikitext line of the top-level `| key =` field, or None."""
    for number, line in enumerate(content.splitlines(), 1):
        if re.match(r'^\s*\|\s*' + re.escape(key) + r'\s*=', line):
            return number
    return None


def version_key(version):
    """Tibia client versions are decimal numbers plus build parts: 8.54 < 8.6 < 11.02 < 13.21.14040."""
    parts = re.findall(r'\d+', version.split('e')[0])
    return (Decimal(parts[0] + '.' + (parts[1] if len(parts) > 1 else '0')),) + tuple(int(p) for p in parts[2:])


def subpages(title, cache):
    """Titles `<title> (...)` for a disambiguation page, from the MediaWiki prefix index (cached)."""
    path = cut_cache(cache) / (cb.slug('subpages ' + title) + '.json')
    if path.exists():
        return json.loads(path.read_text(encoding='utf-8'))
    pages = api({'action': 'query', 'list': 'allpages', 'apprefix': title + ' (', 'aplimit': 50})['query']['allpages']
    titles = sorted(p['title'] for p in pages)
    path.write_text(json.dumps(titles, ensure_ascii=False), encoding='utf-8')
    time.sleep(0.5)
    return titles


def item_page(title, cache, variants=True):
    """Item ids declared by the item's own wiki page at the cut (`| itemid =`). A disambiguation page lists
    its `<title> (...)` item pages as variants, each with its ids and `droppedby` creatures."""
    record = fetch(title, cache)
    cut = record['cut']
    if not cut:
        return {'page_title': title, 'status': 'WIKI_PAGE_MISSING'}
    line = field_line(cut['content'], 'itemid')
    raw = cut['content'].splitlines()[line - 1].split('=', 1)[1] if line else ''
    page = {'page_title': cut['title'], 'page_id': cut['page_id'], 'cut_revision_id': cut['revision_id'],
            'cut_content_sha256': hashlib.sha256(cut['content'].encode('utf-8')).hexdigest(),
            'current_revision_id': record['current']['revision_id'], 'retrieved_at': record['retrieved_at'],
            'status': 'COMPARED', 'itemid_line': line, 'item_ids': [int(v) for v in re.findall(r'\d+', raw)]}
    dropped = field_line(cut['content'], 'droppedby')
    if dropped:
        text = cut['content'].splitlines()[dropped - 1]
        page['droppedby_line'] = dropped
        page['dropped_by'] = [n.strip().lower() for n in re.sub(r'.*\{\{Dropped By\|', '', text).rstrip('}').split('|') if n.strip()]
    if variants and not line and '{{disambig}}' in cut['content'].lower():
        page['variants'] = [v for v in (item_page(sub, cache, False) for sub in subpages(cut['title'], cache))
                            if v['status'] == 'COMPARED' and v['item_ids']]
    return page


def loot_statistics(title, wanted, cache):
    """Kill and drop counts of the highest-version block of `Loot Statistics:<title>` at the cut.

    Every item of that block is returned; items in `wanted` (wiki loot missing in Canary) also get
    their item page ids so an ambiguous name can be resolved.
    """
    record = fetch('Loot Statistics:' + title, cache)
    cut, current = record['cut'], record['current']
    if not cut:
        return {'page_title': 'Loot Statistics:' + title, 'status': 'WIKI_PAGE_MISSING'}
    blocks, block = [], None
    for number, line in enumerate(cut['content'].splitlines(), 1):
        if line.lstrip().startswith('{{Loot'):
            # Only {{Loot2}} blocks record kills-with-drop ("times"); older {{Loot}} blocks count items.
            block = {'items': {}, 'loot2': line.lstrip().startswith('{{Loot2')}
            blocks.append(block)
            continue
        if block is None or line.strip() == '}}':
            block = None if line.strip() == '}}' else block
            continue
        match = re.match(r'^\s*\|\s*(version|kills)\s*=\s*(\S+)', line)
        if match:
            block[match.group(1)] = (match.group(2), number)
            continue
        match = re.match(r'^\s*\|\s*([^,|=]+?),\s*times:\s*(\d+),\s*amount:\s*([\d-]+)', line)
        if match:
            block['items'][match.group(1).strip().lower()] = (int(match.group(2)), match.group(3), number, match.group(1).strip())
    latest = max((b for b in blocks if b['loot2'] and 'version' in b and 'kills' in b), key=lambda b: version_key(b['version'][0]))
    items = []
    for name, (times, amount, line, wiki_name) in sorted(latest['items'].items(), key=lambda kv: kv[1][2]):
        item = {'name': name, 'wiki_name': wiki_name, 'times': times, 'amount': amount, 'line': line}
        if name in wanted:
            item['item_page'] = item_page(wiki_name, cache)
        items.append(item)
    return {'page_title': cut['title'], 'page_id': cut['page_id'], 'cut_revision_id': cut['revision_id'],
            'cut_revision_timestamp': cut['revision_timestamp'],
            'cut_content_sha256': hashlib.sha256(cut['content'].encode('utf-8')).hexdigest(),
            'current_revision_id': current['revision_id'], 'edited_after_cut': current['revision_id'] != cut['revision_id'],
            'retrieved_at': record['retrieved_at'], 'status': 'COMPARED',
            'version': latest['version'][0], 'version_line': latest['version'][1],
            'kills': int(latest['kills'][0]), 'kills_line': latest['kills'][1], 'items': items,
            'rule': 'block with the highest game version; probability estimate = times / kills'}


def wilson(times, kills, z=1.959963984540054):
    """95% Wilson score interval of times/kills, in percent."""
    p, n = times / kills, kills
    denominator = 1 + z * z / n
    centre = (p + z * z / (2 * n)) / denominator
    half = z * math.sqrt(p * (1 - p) / n + z * z / (4 * n * n)) / denominator
    return [round(max(0.0, centre - half) * 100, 4), round(min(1.0, centre + half) * 100, 4)]


def loot_chances(source, stats):
    """Compare each Canary loot entry's chance with the wiki estimate of the same item (D15 loot-rate rule)."""
    by_name = {item['name']: item for item in stats['items']}
    entries = source.get('loot', [])
    names = [(e.get('name') or cb.CONVERTER.names.get(int(e['id']), f'item {e["id"]}')).lower() for e in entries]
    rows = []
    for position, (entry, name) in enumerate(zip(entries, names)):
        low, high = entry.get('minCount', 1), entry.get('maxCount', 1)
        row = {'position': position, 'item': name, 'canary_percent': float(cb.percent_from_chance(entry.get('chance', 0))),
               'canary_amount': str(low) if low == high else f'{low}-{high}'}
        wiki = by_name.get(name)
        if wiki is None:
            variants = [i for i in stats['items'] if re.sub(r'\s*\(.*\)$', '', i['name']) == name]
            wiki = variants[0] if len(variants) == 1 else None
        if names.count(name) > 1:
            row['status'] = 'MULTI_ENTRY'
            row['note'] = 'Canary splits this item over several entries; the wiki counts kills with any drop, so they are not compared.'
        elif wiki is not None and not 0 <= wiki['times'] <= stats['kills']:
            row.update(status='INVALID_STATISTICS', times=wiki['times'], kills=stats['kills'],
                       note='Wiki drop count exceeds the kill count of the block.')
        elif wiki is None:
            row.update(status='NOT_OBSERVED', times=0, kills=stats['kills'], interval_95_wilson=wilson(0, stats['kills']),
                       note='Not listed in the highest-version block.')
        else:
            estimate = wiki['times'] * 100 / stats['kills']
            interval = wilson(wiki['times'], stats['kills'])
            row.update(wiki_item=wiki['wiki_name'], wiki_line=wiki['line'], wiki_amount=wiki['amount'], times=wiki['times'],
                       kills=stats['kills'], wiki_percent=round(estimate, 6), interval_95_wilson=interval,
                       confidence='estimate' if wiki['times'] >= LOW_CONFIDENCE_DROPS else 'low_confidence',
                       status='CONSISTENT' if interval[0] <= row['canary_percent'] <= interval[1] else 'DIFF')
        rows.append(row)
    return rows


def number(value):
    """Exact number of a wiki field value, with thousands separators; None when absent, approximate or uncertain."""
    if value is None or re.search(r'[?~]', value):
        return None
    match = re.match(r'^\s*(-?\d{1,3}(?:,\d{3})+|-?\d+)(\.\d+)?', value)
    return Fraction(match.group(1).replace(',', '') + (match.group(2) or '')) if match else None


def wiki_loot(value):
    names = set()
    for item in re.findall(r'\{\{Loot Item\|([^{}]*)\}\}', value or ''):
        parts = [p.strip() for p in item.split('|') if p.strip()]
        candidates = [p for p in parts if not re.fullmatch(r'[\d\-]+', p)]
        if candidates:
            names.add(candidates[0].lower())
    return names


def compare(relative, canary, _batch_dir, cache):
    name, source, _ = cb.load_monster(canary / cb.MONSTER_DIR / (relative + '.lua'), [])
    slug = cb.slug(name)
    # Compare the plain Canary conversion, never a bundle that already carries adopted wiki values.
    cb.CONVERTER.wiki, cb.CONVERTER.pending_definitions = {}, set()
    _, monster, _, _, manifest, _ = cb.CONVERTER.convert(relative)
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

    def row(field, canary_value, wiki_raw, wiki_value, note=None, key=None):
        if wiki_raw in (None, '', '?') or str(wiki_raw).strip().lower() == 'unknown':
            status = 'WIKI_UNKNOWN'
        elif re.search(r'[?~]', str(wiki_raw)):
            status = 'WIKI_UNCERTAIN'
        elif wiki_value is None:
            status = 'WIKI_UNPARSED'
        elif canary_value == wiki_value:
            status = 'MATCH'
        else:
            status = 'DIFF'
        entry = {'field': field, 'canary': canary_value, 'wiki': wiki_value if wiki_value is not None else wiki_raw, 'wiki_raw': wiki_raw, 'status': status}
        if key and field_line(cut['content'], key):
            entry['wiki_line'] = field_line(cut['content'], key)
        if note:
            entry['note'] = note
        rows.append(entry)

    def as_number(value):
        return None if value is None else (int(value) if value.denominator == 1 else float(value))

    stats = c['stats']
    for field, key, value in (('max_health', 'hp', stats['max_health']), ('experience', 'exp', stats['experience']),
                              ('armor', 'armor', stats['armor']), ('speed', 'speed', stats['speed'])):
        row(field, value, fields.get(key), as_number(number(fields.get(key))),
            'Canary monster.speed is the raw engine value; the wiki lists observed speed.' if field == 'speed' else None, key)
    mitigation = stats.get('mitigation_percent')
    row('mitigation_percent', float(Fraction(mitigation['numerator'], mitigation['denominator'])) if mitigation else None,
        fields.get('mitigation'), as_number(number(fields.get('mitigation'))), key='mitigation')
    resist = {r['damage_type']: Fraction(r['reduction_percent']['numerator'], r['reduction_percent']['denominator']) for r in c['resistances']}
    resist.update({damage: Fraction(100) for damage in c['immunities']['damage_types']})
    for element, key in ELEMENTS.items():
        taken = number(fields.get(key))
        row(f'resistance.{element}', as_number(resist.get(element, Fraction(0))), fields.get(key),
            as_number(100 - taken) if taken is not None else None, 'Wiki lists damage taken; resistance = 100 - taken; a Canary damage immunity counts as 100.', key)
    summoning = c['summoning']
    for field, key, flag in (('summon_mana_cost', 'summon', 'summonable'), ('convince_mana_cost', 'convince', 'convinceable')):
        raw = fields.get(key)
        wiki_value = as_number(number(raw)) if number(raw) is not None else ('--' if raw and raw.strip().lower() in ('--', '-', 'no') else None)
        row(field, summoning.get('mana_cost') if summoning[flag] else '--', raw, wiki_value, key=key)
    for field, key, value in (('illusionable', 'illusionable', c['flags']['illusionable']),
                              ('pushable', 'pushable', b['movement']['pushable']),
                              ('push_items', 'pushobjects', b['movement']['push_items']),
                              ('sense_invisible', 'senseinvis', b['targeting']['sense_invisible']),
                              ('paralyze_immune', 'paraimmune', 'paralyze' in c['immunities']['conditions'])):
        raw = fields.get(key)
        row(field, value, raw, {'yes': True, 'no': False}.get((raw or '').strip().lower()), key=key)
    row('flee_health', b['targeting']['flee_health'], fields.get('runsat'), as_number(number(fields.get('runsat'))), key='runsat')
    bestiary = c.get('bestiary', {})
    for field, key in (('bestiary.class', 'bestiaryclass'), ('bestiary.difficulty', 'bestiarylevel'), ('bestiary.occurrence', 'occurrence')):
        canary_value = bestiary.get(field.split('.')[1])
        raw = fields.get(key)
        wiki_value = raw.strip().lower().replace(' ', '_') if raw else None
        row(field, canary_value.lower() if isinstance(canary_value, str) else canary_value, raw, wiki_value, key=key)
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
                 'note': 'Item names only; chances are compared in loot_chances from the Loot Statistics page.'}
        if field_line(cut['content'], 'loot'):
            entry['wiki_line'] = field_line(cut['content'], 'loot')
        stats = loot_statistics(name, sorted(only_wiki), cache)
        result['loot_statistics'] = stats
        if stats['status'] == 'COMPARED' and canary_loot:
            result['loot_chances'] = loot_chances(source, stats)
        if variants:
            entry['name_variants'] = [{'canary': c_name, 'wiki': w_name} for c_name, w_name in variants]
            entry['note'] += ' Wiki disambiguated names (e.g. "book (grey)") are matched to the Canary base name.'
        rows.append(entry)
    result.update({'status': 'COMPARED', 'rows': rows})
    if any('COMBAT_UNDEFINEDDAMAGE' in e.get('resolution', '') for e in manifest['entries']):
        # D25: the converter decides an undefined combat element from these wiki abilities.
        import wiki_scenes
        _, shapes = wiki_scenes.scene_shapes(cache)
        result['abilities'] = [{**{k: v for k, v in a.items() if k != 'tiles'}, 'tiles': sorted(map(list, a.get('tiles', ())))}
                               for a in wiki_scenes.wiki_abilities(cut['content'], shapes, cache, {})]
    return result


def totals(results):
    """Totals over full results (before compact())."""
    summary, chances, confidence, fields = {}, {}, {}, {}
    for result in results:
        for entry in result.get('rows', []):
            summary[entry['status']] = summary.get(entry['status'], 0) + 1
            if entry['status'] == 'DIFF':
                fields[entry['field']] = fields.get(entry['field'], 0) + 1
        for entry in result.get('loot_chances', []):
            chances[entry['status']] = chances.get(entry['status'], 0) + 1
            if 'confidence' in entry:
                confidence[entry['confidence']] = confidence.get(entry['confidence'], 0) + 1
    return (dict(sorted(summary.items())), dict(sorted(chances.items())), dict(sorted(confidence.items())),
            dict(sorted(fields.items(), key=lambda kv: (-kv[1], kv[0]))))


def compact(result):
    """Population form: revisions, per-status counts and only the rows that are not MATCH/CONSISTENT."""
    out = {k: v for k, v in result.items() if k not in ('rows', 'loot_chances', 'loot_statistics')}
    counts = {}
    for entry in result.get('rows', []):
        counts[entry['status']] = counts.get(entry['status'], 0) + 1
    out['row_counts'] = dict(sorted(counts.items()))
    out['rows'] = [r for r in result.get('rows', []) if r['status'] == 'DIFF']
    stats = result.get('loot_statistics')
    if stats:
        # Keep the statistics of wiki-only loot items: canary_batch.py adopts them (D15).
        out['loot_statistics'] = {**{k: v for k, v in stats.items() if k != 'items'},
                                  'items': [i for i in stats.get('items', []) if 'item_page' in i]}
    chance_counts = {}
    for entry in result.get('loot_chances', []):
        chance_counts[entry['status']] = chance_counts.get(entry['status'], 0) + 1
    if chance_counts:
        out['loot_chance_counts'] = dict(sorted(chance_counts.items()))
    # Every loot chance is kept (trimmed): canary_batch.py applies the D15 loot rate rule from it.
    keep = ('position', 'item', 'status', 'confidence', 'canary_percent', 'wiki_percent', 'interval_95_wilson', 'times', 'kills',
            'wiki_line', 'canary_amount', 'wiki_amount', 'note')
    out['loot_chances'] = [{k: c[k] for k in keep if k in c} for c in result.get('loot_chances', [])]
    return out


def main():
    parser = argparse.ArgumentParser(description=__doc__.split('\n')[0])
    parser.add_argument('--canary', required=True, type=Path)
    parser.add_argument('--batch', default=cb.REV, choices=sorted(cb.BATCHES))
    parser.add_argument('--population', action='store_true', help='compare every Canary monster file (compact output)')
    parser.add_argument('--cache', type=Path, default=Path('/tmp/oteryn-wiki-cache'))
    args = parser.parse_args()
    args.cache.mkdir(parents=True, exist_ok=True)
    objects = cb.load_appearance_objects(args.canary / 'data/items/appearances.dat')
    items = cb.load_items_xml(args.canary / 'data/items/items.xml')
    names, index = cb.name_index(objects, items)
    cb.CONVERTER = cb.Converter(args.canary, objects, items, names, index)
    if args.population:
        results, skipped = [], []
        for path in sorted((args.canary / cb.MONSTER_DIR).rglob('*.lua')):
            relative = str(path.relative_to(args.canary / cb.MONSTER_DIR))[:-4]
            try:
                results.append(compare(relative, args.canary, None, args.cache))
            except Exception as exc:  # files the converter cannot convert (see population_census.py)
                skipped.append({'file': relative, 'error': f'{type(exc).__name__}: {str(exc).splitlines()[0][:100]}'})
        out = ROOT / 'samples' / 'wiki-population-2026-09-27.json'
    else:
        results, skipped = [compare(relative, args.canary, None, args.cache) for relative in cb.BATCHES[args.batch]], []
        out = ROOT / 'samples' / args.batch / 'wiki-2026-09-27.json'
    rows, chances, confidence, fields = totals(results)
    statuses = {}
    for result in results:
        statuses[result['status']] = statuses.get(result['status'], 0) + 1
    if args.population:
        results = [compact(result) for result in results]
    report = {'source': 'TibiaWiki (Fandom), CC BY-SA; only compared facts are recorded', 'api': API,
              'target_cut': TARGET_CUT, 'cut_rule': f'last revision at or before {CUT_TIMESTAMP}',
              'classification': 'Wiki = player-observed reference evidence; Canary = OTS_HYPOTHESIS_ONLY',
              'row_status_totals': rows,
              'loot_chance_rule': f'highest-version Loot Statistics block at the cut; estimate = times / kills with a 95% Wilson interval; '
                                  f'fewer than {LOW_CONFIDENCE_DROPS} drops is low_confidence. CONSISTENT = Canary inside the interval',
              'loot_chance_totals': chances, 'loot_chance_confidence': confidence}
    if args.population:
        report.update({'scope': 'Every convertible Canary monster file. Per monster the DIFF rows (the only ones D15 '
                                'adopts) and all loot chances (trimmed) are listed; other statuses are counted.',
                       'monster_status_totals': dict(sorted(statuses.items())), 'diff_fields': fields, 'not_converted': skipped})
    report['monsters'] = results
    if args.population:
        # One monster per line keeps the population file small and its Git diffs readable.
        head = json.dumps({**report, 'monsters': []}, ensure_ascii=False, indent=2)[:-len('\n  "monsters": []\n}')]
        lines = ',\n'.join('    ' + json.dumps(r, ensure_ascii=False, separators=(',', ':')) for r in results)
        out.write_text(head + '\n  "monsters": [\n' + lines + '\n  ]\n}\n', encoding='utf-8', newline='\n')
    else:
        out.write_text(json.dumps(report, ensure_ascii=False, indent=2) + '\n', encoding='utf-8', newline='\n')
    print(json.dumps({'out': str(out), 'totals': rows, 'loot_chances': chances, 'confidence': confidence,
                      **({'monsters': statuses, 'top_diff_fields': dict(list(fields.items())[:12])} if args.population else {})}))


if __name__ == '__main__':
    main()
