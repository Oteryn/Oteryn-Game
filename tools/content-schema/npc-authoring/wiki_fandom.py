"""Compare converted Canary/Crystal NPC bundles with TibiaWiki (Fandom) infobox facts.

Evidence tooling only: TibiaWiki is a player-observed reference source (CC BY-SA); this tool
records only the compared facts (positions, travel prices, trade prices) and their page/revision
ids, never article prose or dialogue. Nothing here becomes Game truth by comparison alone.

Style mirrors monster-authoring/wiki_compare.py and wiki_scenes.py (same Fandom API, throttling
and licensing conventions), but this tool is self-contained: Python stdlib only (urllib), no
dependency on the Canary/Crystal converters.

Usage:
    python wiki_fandom.py fetch --cache <dir>
    python wiki_fandom.py compare --snapshot <cache>/fandom-npc-snapshot.json \
        --bundles <out/canary/bundles> [--bundles <out/crystal/bundles>] --out <report.json>
    python wiki_fandom.py self-test
"""
import argparse
import hashlib
import json
import re
import time
import unicodedata
import urllib.error
import urllib.parse
import urllib.request
from collections import Counter, defaultdict
from datetime import datetime, timezone
from pathlib import Path

API = 'https://tibia.fandom.com/api.php'
USER_AGENT = 'OterynNpcAuthoring/1.0 (+https://github.com/Oteryn/Oteryn-Game)'
LICENSE_NOTE = 'TibiaWiki (Fandom), CC BY-SA; only compared facts (positions, travel and trade prices) are recorded, never article prose or dialogue.'
NPC_TEMPLATE = 'Template:Infobox NPC'
ITEM_TEMPLATE = 'Template:Infobox Object'
THROTTLE_SECONDS = 0.5  # <= 2 requests/s
RETRIES = 4
NEAR_TILES = 3
SNAPSHOT_SCHEMA = 'OTERYN_NPC_FANDOM_SNAPSHOT/v1'
COMPARE_SCHEMA = 'OTERYN_NPC_FANDOM_COMPARE/v1'


# --------------------------------------------------------------------------------------------
# Low-level Fandom API access
# --------------------------------------------------------------------------------------------

def api(params, retries=RETRIES):
    """One MediaWiki API call with retries on transient errors. No personal data ever leaves in headers."""
    query = urllib.parse.urlencode({**params, 'format': 'json', 'formatversion': '2'})
    url = API + '?' + query
    delay, last_exc = 1.0, None
    for attempt in range(retries):
        try:
            request = urllib.request.Request(url, headers={'User-Agent': USER_AGENT})
            with urllib.request.urlopen(request, timeout=30) as response:
                return json.load(response)
        except (urllib.error.URLError, TimeoutError, OSError) as exc:
            last_exc = exc
            if attempt + 1 < retries:
                time.sleep(delay)
                delay *= 2
    raise RuntimeError(f'Fandom API request failed after {retries} attempts: {last_exc}') from last_exc


def list_titles(template_title, limit=500):
    """Every page title that embeds `template_title` (list=embeddedin; cheap, no content), throttled."""
    titles = []
    params = {'action': 'query', 'list': 'embeddedin', 'eititle': template_title, 'einamespace': 0, 'eilimit': limit}
    while True:
        data = api(params)
        titles.extend(page['title'] for page in data.get('query', {}).get('embeddedin', []))
        time.sleep(THROTTLE_SECONDS)
        cont = data.get('continue')
        if not cont:
            break
        params.update(cont)
    return titles


def fetch_pages(template_title, batch=50):
    """Yield every page {pageid, title, revisions:[...]} that embeds `template_title`, throttled.

    Two steps: list every title with the cheap `list=embeddedin` (paginates cleanly), then fetch
    content in bounded `titles=a|b|...` batches. A single combined `generator=embeddedin` +
    `prop=revisions` query is not used: with a generator supplying multiple pages, `rvcontinue`
    can come back pinned to the same first page batch (a known multi-page revision-continuation
    quirk), which would loop forever instead of advancing.
    """
    titles = list_titles(template_title)
    for i in range(0, len(titles), batch):
        chunk = titles[i:i + batch]
        data = api({'action': 'query', 'titles': '|'.join(chunk), 'prop': 'revisions',
                    'rvprop': 'ids|content', 'rvslots': 'main'})
        for page in data.get('query', {}).get('pages', []):
            yield page
        time.sleep(THROTTLE_SECONDS)


def page_content(page):
    """(revid, wikitext) of a query-result page's single fetched revision, or (None, None) if missing/deleted."""
    revisions = page.get('revisions') or []
    if page.get('missing') or not revisions:
        return None, None
    rev = revisions[0]
    return rev.get('revid'), rev['slots']['main']['content']


# --------------------------------------------------------------------------------------------
# Generic wikitext parsing (mirrors the brace/bracket-depth conventions of wiki_compare.py /
# wiki_scenes.py, generalised to any top-level infobox template and any nested template call).
# --------------------------------------------------------------------------------------------

def top_level_fields(text, start):
    """`| key = value` fields of the template beginning at `start` (text[start:start+2] == '{{'),
    honouring nested {{ }} and [[ ]] the same way wiki_compare.infobox() does."""
    depth, i, fields, current, buffer = 0, start, {}, None, []
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
            candidates = [k for k in (text.find('|', i + 1), text.find('}}', i + 1)) if k >= 0]
            nxt = min(candidates) if candidates else len(text)
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


def infobox(text, template):
    """Top-level fields of the first `{{<template>|...}}` in `text`, or {} if it is not there
    (a page can be listed as embedding a template only transitively, via a subpage it transcludes)."""
    match = re.search(r'\{\{\s*' + re.escape(template) + r'\b', text, re.IGNORECASE)
    return top_level_fields(text, match.start()) if match else {}


def split_top_level(text, sep):
    """Split `text` on `sep` characters outside any {{ }} or [[ ]] nesting."""
    parts, depth, buffer = [], 0, ''
    i = 0
    while i < len(text):
        pair = text[i:i + 2]
        if pair in ('{{', '[['):
            depth += 1
            buffer += pair
            i += 2
            continue
        if pair in ('}}', ']]'):
            depth -= 1
            buffer += pair
            i += 2
            continue
        if text[i] == sep and depth == 0:
            parts.append(buffer)
            buffer = ''
        else:
            buffer += text[i]
        i += 1
    parts.append(buffer)
    return parts


def extract_balanced(text, start):
    """Body of the `{{...}}` template beginning at `start` (name + params, braces stripped) and the
    index right after its closing `}}`."""
    depth, i, n = 0, start, len(text)
    while i < n:
        pair = text[i:i + 2]
        if pair == '{{':
            depth += 1
            i += 2
            continue
        if pair == '}}':
            depth -= 1
            i += 2
            if depth == 0:
                return text[start + 2:i - 2], i
            continue
        i += 1
    return text[start + 2:], n


def split_params(body):
    """(positional, named) of a template body (its own name as first part, discarded)."""
    parts = split_top_level(body, '|')
    positional, named = [], {}
    for part in parts[1:]:
        match = re.match(r'^\s*([A-Za-z_][A-Za-z0-9_ ]*)\s*=\s*(.*)$', part, re.S)
        if match and '{{' not in match.group(1) and '[[' not in match.group(1):
            named[match.group(1).strip().lower()] = match.group(2).strip()
        else:
            positional.append(part.strip())
    return positional, named


def find_templates(text, name):
    """Every `{{name|...}}` call anywhere in `text` (any nesting depth), as [(positional, named), ...].
    Used to pull `{{TransportCell|...}}` calls out of an infobox `notes` field, which itself sits
    inside a `{{TransportList|...}}` wrapper."""
    results, lname, i, n = [], name.strip().lower(), 0, len(text)
    while i < n:
        if text[i:i + 2] == '{{':
            k = i + 2
            while k < n and text[k] not in '|}\n':
                k += 1
            if text[i + 2:k].strip().lower() == lname:
                body, end = extract_balanced(text, i)
                results.append(split_params(body))
                i = end
                continue
        i += 1
    return results


def strip_wiki_markup(text):
    """A short informational string with the common wiki markup removed (links, refs, bold/italic,
    templates, comments). Not used for anything that is compared, only for snapshot readability."""
    if not text:
        return ''
    text = re.sub(r'<!--.*?-->', '', text, flags=re.S)
    text = re.sub(r'<ref[^>]*/>', '', text)
    text = re.sub(r'<ref[^>]*>.*?</ref>', '', text, flags=re.S)
    text = re.sub(r'\[\[[^\]|]*\|([^\]]*)\]\]', r'\1', text)
    text = re.sub(r'\[\[([^\]]*)\]\]', r'\1', text)
    text = re.sub(r"'{2,}", '', text)
    text = re.sub(r'\{\{[^{}]*\}\}', '', text)
    return re.sub(r'\s+', ' ', text).strip()


def parse_price(value):
    """First integer in `value` (thousands separators stripped), or None (missing/unparsable/'--'/'-')."""
    if value is None:
        return None
    match = re.search(r'-?\d[\d,]*', value)
    return int(match.group(0).replace(',', '')) if match else None


def parse_int(value):
    if value is None:
        return None
    match = re.search(r'-?\d+', value)
    return int(match.group(0)) if match else None


def parse_coord(value):
    """TibiaWiki posx/posy: `sector.offset` (x = sector*256 + offset) or a plain absolute coordinate."""
    if value is None:
        return None
    v = value.strip()
    if not v or v in ('?', '-', '--'):
        return None
    match = re.fullmatch(r'(\d+)\.(\d+)', v)
    if match:
        return int(match.group(1)) * 256 + int(match.group(2))
    match = re.fullmatch(r'\d+', v)
    return int(v) if match else None


def normalize_name(value):
    """Case/whitespace/apostrophe-insensitive join key for NPC, destination and item names."""
    if not value:
        return ''
    value = unicodedata.normalize('NFKC', value)
    value = value.replace('’', "'").replace('‘', "'").replace('´', "'").replace('`', "'")
    return re.sub(r'\s+', ' ', value.strip()).lower()


def parse_npc_price_list(value):
    """`buyfrom`/`sellto` field: comma-separated NPC names, each optionally `Name: price` overriding the
    page's generic price. Returns ([(name, override_price_or_None), ...], unparsable_fragment_count)."""
    entries, unparsed = [], 0
    if value is None:
        return entries, unparsed
    v = value.strip()
    if v.lower() in ('', '-', '--', 'none', 'no', 'n/a'):
        return entries, unparsed
    for part in split_top_level(v, ','):
        part = part.strip()
        if not part:
            continue
        name_part, _, price_part = part.partition(':')
        name = strip_wiki_markup(name_part)
        name = re.sub(r'[\*†‡\d]+$', '', name).strip()  # trailing footnote markers
        if not name or not re.search(r'[A-Za-z]', name):  # e.g. a stray "-"/"--" list fragment
            unparsed += 1
            continue
        price = None
        if price_part:
            price = parse_price(price_part)
            if price is None:
                unparsed += 1
        entries.append((name, price))
    return entries, unparsed


def parse_transport(notes):
    """`{{TransportCell|Destination|Price|optional condition}}` calls inside an NPC's `notes` field."""
    out = []
    for positional, _named in find_templates(notes or '', 'TransportCell'):
        if len(positional) < 2:
            continue
        destination = strip_wiki_markup(positional[0])
        if not destination:
            continue
        price = parse_price(positional[1])
        condition = positional[2].strip() if len(positional) > 2 else ''
        out.append({'destination': destination, 'price': price, 'condition_present': bool(condition)})
    return out


# --------------------------------------------------------------------------------------------
# fetch
# --------------------------------------------------------------------------------------------

def build_npc_record(page):
    revid, content = page_content(page)
    if content is None:
        return None
    fields = infobox(content, 'Infobox NPC')
    if not fields:
        return None  # page embeds the template only transitively (e.g. via a "<city> NPCs" subpage)
    name = strip_wiki_markup(fields.get('name')) or page['title']
    jobs = [strip_wiki_markup(fields.get(key)) for key in ('job', 'job2', 'job3')]
    jobs = [job for job in jobs if job]
    x, y, z = parse_coord(fields.get('posx')), parse_coord(fields.get('posy')), parse_int(fields.get('posz'))
    position = {'x': x, 'y': y, 'z': z} if None not in (x, y, z) else None
    return {'pageid': page['pageid'], 'revid': revid, 'title': page['title'], 'name': name, 'jobs': jobs,
            'city': strip_wiki_markup(fields.get('city')), 'location': strip_wiki_markup(fields.get('location'))[:300],
            'position': position, 'transport': parse_transport(fields.get('notes'))}


def build_item_record(page, trade_by_npc, unparsed_counter):
    revid, content = page_content(page)
    if content is None:
        return None
    fields = infobox(content, 'Infobox Object')
    if not fields:
        return None
    title = page['title']
    generic_buy, generic_sell = parse_price(fields.get('npcprice')), parse_price(fields.get('npcvalue'))
    buy_entries, buy_unparsed = parse_npc_price_list(fields.get('buyfrom'))
    sell_entries, sell_unparsed = parse_npc_price_list(fields.get('sellto'))
    unparsed_counter[0] += buy_unparsed + sell_unparsed
    for name, override in buy_entries:
        price = override if override is not None else generic_buy
        if price is None:
            continue
        row = trade_by_npc[normalize_name(name)].setdefault(title.lower(), {'item': title, 'buy_price': None, 'sell_price': None})
        row['buy_price'] = price
    for name, override in sell_entries:
        price = override if override is not None else generic_sell
        if price is None:
            continue
        row = trade_by_npc[normalize_name(name)].setdefault(title.lower(), {'item': title, 'buy_price': None, 'sell_price': None})
        row['sell_price'] = price
    return {'pageid': page['pageid'], 'revid': revid, 'title': title}


def cmd_fetch(args):
    cache = Path(args.cache)
    cache.mkdir(parents=True, exist_ok=True)
    npcs, npc_pages_seen = [], 0
    for page in fetch_pages(NPC_TEMPLATE):
        npc_pages_seen += 1
        record = build_npc_record(page)
        if record:
            npcs.append(record)
    npcs.sort(key=lambda n: n['title'])

    trade_by_npc = defaultdict(dict)
    unparsed_counter = [0]
    item_pages, item_pages_seen = [], 0
    for page in fetch_pages(ITEM_TEMPLATE):
        item_pages_seen += 1
        record = build_item_record(page, trade_by_npc, unparsed_counter)
        if record:
            item_pages.append(record)
    item_pages.sort(key=lambda i: i['title'])
    trade = {npc: sorted(items.values(), key=lambda i: i['item']) for npc, items in sorted(trade_by_npc.items())}

    snapshot = {
        'schema': SNAPSHOT_SCHEMA, 'license': LICENSE_NOTE, 'api': API,
        'fetched_at': datetime.now(timezone.utc).strftime('%Y-%m-%dT%H:%M:%SZ'),
        'counts': {'npc_pages_seen': npc_pages_seen, 'npc_pages': len(npcs),
                   'item_pages_seen': item_pages_seen, 'item_pages': len(item_pages),
                   'trade_npcs': len(trade), 'unparsable_trade_fragments': unparsed_counter[0]},
        'npcs': npcs, 'trade': trade, 'item_pages': item_pages,
    }
    out = cache / 'fandom-npc-snapshot.json'
    out.write_text(json.dumps(snapshot, ensure_ascii=False, indent=1, sort_keys=True), encoding='utf-8')
    print(json.dumps({'out': str(out), 'counts': snapshot['counts']}))


# --------------------------------------------------------------------------------------------
# compare
# --------------------------------------------------------------------------------------------

def classify_position(wiki_position, placements):
    if wiki_position is None:
        return {'status': 'NO_WIKI_POSITION'}
    if not placements:
        return {'status': 'NO_PLACEMENT', 'wiki': wiki_position}
    best, fallback = None, None
    for placement in placements:
        pos = placement.get('position') or {}
        if pos.get('x') is None or pos.get('y') is None or pos.get('z') is None:
            continue
        fallback = fallback or pos
        if pos == wiki_position:
            return {'status': 'MATCH', 'wiki': wiki_position, 'source': pos}
        if pos['z'] == wiki_position['z']:
            distance = max(abs(pos['x'] - wiki_position['x']), abs(pos['y'] - wiki_position['y']))
            if distance <= NEAR_TILES and (best is None or distance < best[0]):
                best = (distance, pos)
    if best is not None:
        return {'status': 'NEAR', 'wiki': wiki_position, 'source': best[1], 'chebyshev_distance': best[0]}
    return {'status': 'MISMATCH', 'wiki': wiki_position, 'source': fallback}


def compare_travel(wiki_transport, bundle_travel):
    wiki_map, source_map = {}, {}
    for entry in wiki_transport:
        if entry.get('price') is None:
            continue
        key = normalize_name(entry['destination'])
        if key:
            wiki_map.setdefault(key, {'name': entry['destination'], 'price': entry['price']})
    for entry in bundle_travel:
        key = normalize_name(entry.get('keyword') or '')
        if key and entry.get('price') is not None:
            source_map.setdefault(key, {'name': entry.get('keyword'), 'price': entry['price']})
    rows = []
    for key in sorted(set(wiki_map) | set(source_map)):
        wiki, source = wiki_map.get(key), source_map.get(key)
        if wiki and source:
            status = 'MATCH' if wiki['price'] == source['price'] else 'MISMATCH'
            rows.append({'destination': key, 'status': status, 'wiki_price': wiki['price'], 'source_price': source['price']})
        elif wiki:
            rows.append({'destination': key, 'status': 'WIKI_ONLY', 'wiki_price': wiki['price']})
        else:
            rows.append({'destination': key, 'status': 'SOURCE_ONLY', 'source_price': source['price']})
    return rows


def compare_trade(wiki_items, offers):
    wiki_map = {normalize_name(item['item']): item for item in wiki_items}
    source_map = {normalize_name(offer['item_name']): offer for offer in offers if offer.get('item_name')}
    buy_rows, sell_rows = [], []
    for key in sorted(set(wiki_map) | set(source_map)):
        wiki, source = wiki_map.get(key), source_map.get(key)
        name = (wiki or {}).get('item') or (source or {}).get('item_name') or key
        wiki_buy, wiki_sell = (wiki or {}).get('buy_price'), (wiki or {}).get('sell_price')
        source_buy, source_sell = (source or {}).get('buy_price'), (source or {}).get('sell_price')
        for rows, wiki_price, source_price in ((buy_rows, wiki_buy, source_buy), (sell_rows, wiki_sell, source_sell)):
            if wiki_price is None and source_price is None:
                continue
            if wiki_price is not None and source_price is not None:
                status = 'MATCH' if wiki_price == source_price else 'MISMATCH'
                rows.append({'item': name, 'status': status, 'wiki_price': wiki_price, 'source_price': source_price})
            elif wiki_price is not None:
                rows.append({'item': name, 'status': 'WIKI_ONLY', 'wiki_price': wiki_price})
            else:
                rows.append({'item': name, 'status': 'SOURCE_ONLY', 'source_price': source_price})
    return buy_rows, sell_rows


def source_label(bundles_dir):
    path = Path(bundles_dir)
    return path.parent.name if path.name == 'bundles' else path.name


def cmd_compare(args):
    snapshot_bytes = Path(args.snapshot).read_bytes()
    snapshot = json.loads(snapshot_bytes)
    wiki_npcs = {normalize_name(npc['title']): npc for npc in snapshot['npcs']}
    trade = snapshot.get('trade', {})

    report_sources = {}
    for bundles_dir in args.bundles:
        label = source_label(bundles_dir)
        joined, source_only, matched = [], [], set()
        for path in sorted(Path(bundles_dir).glob('*.json')):
            bundle = json.loads(path.read_text(encoding='utf-8'))
            definition = bundle.get('definition') or {}
            name = definition.get('name') or definition.get('display_name') or path.stem
            key_norm = normalize_name(name)
            wiki = wiki_npcs.get(key_norm)
            if wiki is None:
                source_only.append({'name': name, 'key': bundle.get('key')})
                continue
            matched.add(key_norm)
            services = bundle.get('services') or {}
            trade_service = services.get('trade') or {}
            position = classify_position(wiki.get('position'), bundle.get('placements') or [])
            travel_rows = compare_travel(wiki.get('transport') or [], services.get('travel') or [])
            buy_rows, sell_rows = compare_trade(trade.get(key_norm, []), trade_service.get('offers') or [])
            joined.append({'name': name, 'key': bundle.get('key'), 'position': position,
                            'travel': travel_rows, 'trade_buy': buy_rows, 'trade_sell': sell_rows})
        wiki_only = [{'name': wiki_npcs[key]['name'], 'title': wiki_npcs[key]['title']}
                     for key in sorted(set(wiki_npcs) - matched)]
        report_sources[label] = {
            'joined': len(joined), 'wiki_only_npcs': len(wiki_only), 'source_only_npcs': len(source_only),
            'position_totals': dict(sorted(Counter(row['position']['status'] for row in joined).items())),
            'travel_totals': dict(sorted(Counter(t['status'] for row in joined for t in row['travel']).items())),
            'trade_buy_totals': dict(sorted(Counter(t['status'] for row in joined for t in row['trade_buy']).items())),
            'trade_sell_totals': dict(sorted(Counter(t['status'] for row in joined for t in row['trade_sell']).items())),
            'npcs': [{'name': row['name'], 'key': row['key'], 'position': row['position'],
                      'travel_issues': [t for t in row['travel'] if t['status'] != 'MATCH'],
                      'trade_buy_issues': [t for t in row['trade_buy'] if t['status'] != 'MATCH'],
                      'trade_sell_issues': [t for t in row['trade_sell'] if t['status'] != 'MATCH']}
                     for row in joined],
            'wiki_only': wiki_only, 'source_only': source_only,
        }

    report = {'schema': COMPARE_SCHEMA, 'license': LICENSE_NOTE, 'api': API,
              'snapshot': {'file': Path(args.snapshot).name, 'sha256': hashlib.sha256(snapshot_bytes).hexdigest(),
                           'fetched_at': snapshot.get('fetched_at'), 'counts': snapshot.get('counts')},
              'generated_at': datetime.now(timezone.utc).strftime('%Y-%m-%dT%H:%M:%SZ'),
              'position_rule': f'MATCH = exact placement; NEAR = same z, Chebyshev distance <= {NEAR_TILES}; else MISMATCH.',
              'sources': report_sources}
    out = Path(args.out)
    out.parent.mkdir(parents=True, exist_ok=True)
    out.write_text(json.dumps(report, ensure_ascii=False, indent=1, sort_keys=True) + '\n', encoding='utf-8')
    print(json.dumps({'out': str(out), 'sources': {k: {kk: vv for kk, vv in v.items() if kk.endswith('_totals') or kk in ('joined', 'wiki_only_npcs', 'source_only_npcs')} for k, v in report_sources.items()}}))


# --------------------------------------------------------------------------------------------
# self-test (offline; no network)
# --------------------------------------------------------------------------------------------

def cmd_self_test(_args):
    checks = 0

    def check(label, actual, expected):
        nonlocal checks
        checks += 1
        assert actual == expected, f'{label}: expected {expected!r}, got {actual!r}'

    check('coord sector.offset', parse_coord('126.54'), 32310)
    check('coord sector.offset y', parse_coord('125.199'), 32199)
    check('coord plain integer', parse_coord('32310'), 32310)
    check('coord unknown', parse_coord('?'), None)
    check('coord empty', parse_coord(''), None)

    bluebear_notes = ("Captain Bluebear will transport players by ship to:\n"
                       "{{TransportList \n |{{TransportCell|Ab'Dendriel|130}}\n |{{TransportCell|Carlin|110}}\n"
                       " |{{TransportCell|Targuna|0|After paying 5,000 or providing a [[Sail Pass]]}}\n}}")
    transport = parse_transport(bluebear_notes)
    by_dest = {t['destination']: t for t in transport}
    check('transport count', len(transport), 3)
    check('transport simple price', by_dest["Ab'Dendriel"]['price'], 130)
    check('transport simple price 2', by_dest['Carlin']['price'], 110)
    check('transport condition price', by_dest['Targuna']['price'], 0)
    check('transport condition flag (with condition)', by_dest['Targuna']['condition_present'], True)
    check('transport condition flag (without condition)', by_dest['Carlin']['condition_present'], False)
    check('transport link stripped from condition-bearing destination', "'" in by_dest["Ab'Dendriel"]['destination'], True)

    axe_buyfrom = "Baltim, Brengus, Cedrik, Coltrayne: 20, Esrik, [[Sam]], Obi: 20, Willard"
    entries, unparsed = parse_npc_price_list(axe_buyfrom)
    names = {n: p for n, p in entries}
    check('buyfrom entry count', len(entries), 8)
    check('buyfrom no-override is None', names['Baltim'], None)
    check('buyfrom override parsed', names['Coltrayne'], 20)
    check('buyfrom wikilink stripped', names['Sam'], None)
    check('buyfrom override after wikilink', names['Obi'], 20)
    check('buyfrom no unparsable fragments', unparsed, 0)

    sellto_with_dash = "Baltim, Brengus, H.L.: 6, --"
    entries2, unparsed2 = parse_npc_price_list(sellto_with_dash)
    names2 = {n: p for n, p in entries2}
    check('sellto dotted-name override', names2['H.L.'], 6)
    check('sellto trailing dash is unparsable, not crashing', unparsed2, 1)

    check('none/dash list yields nothing', parse_npc_price_list('--')[0], [])
    check('thousands separator price', parse_price('25,000'), 25000)
    check('missing price', parse_price('-'), None)

    fields = infobox('{{Infobox NPC|List={{{1|}}}|GetValue={{{GetValue|}}}\n| name = Sam\n| job = Artisan\n'
                      '| posx = 126.104\n| posy = 125.199\n| posz = 7\n| notes = Sells things.\n}}', 'Infobox NPC')
    check('infobox top-level field extraction', fields.get('name'), 'Sam')
    check('infobox posx field extraction', fields.get('posx'), '126.104')
    check('missing template returns empty', infobox('no template here', 'Infobox NPC'), {})

    check('normalize_name apostrophe/case/whitespace', normalize_name("  Ab’Dendriel  "), normalize_name("ab'dendriel"))

    position_match = classify_position({'x': 100, 'y': 100, 'z': 7}, [{'position': {'x': 100, 'y': 100, 'z': 7}}])
    check('position exact match', position_match['status'], 'MATCH')
    position_near = classify_position({'x': 100, 'y': 100, 'z': 7}, [{'position': {'x': 102, 'y': 100, 'z': 7}}])
    check('position near', position_near['status'], 'NEAR')
    position_far = classify_position({'x': 100, 'y': 100, 'z': 7}, [{'position': {'x': 200, 'y': 100, 'z': 7}}])
    check('position mismatch', position_far['status'], 'MISMATCH')
    check('position no wiki', classify_position(None, [{'position': {'x': 1, 'y': 1, 'z': 1}}])['status'], 'NO_WIKI_POSITION')
    check('position no placement', classify_position({'x': 1, 'y': 1, 'z': 1}, [])['status'], 'NO_PLACEMENT')

    travel_rows = compare_travel([{'destination': 'Carlin', 'price': 110}, {'destination': 'Targuna', 'price': None}],
                                  [{'keyword': 'carlin', 'price': 110}, {'keyword': 'krailos', 'price': 230}])
    travel_by_dest = {r['destination']: r for r in travel_rows}
    check('travel match', travel_by_dest['carlin']['status'], 'MATCH')
    check('travel source only', travel_by_dest['krailos']['status'], 'SOURCE_ONLY')
    check('travel unparsable wiki price excluded from wiki side', 'targuna' not in travel_by_dest, True)

    buy_rows, sell_rows = compare_trade([{'item': 'Axe', 'buy_price': 20, 'sell_price': 7}],
                                         [{'item_name': 'axe', 'buy_price': 20, 'sell_price': 5}])
    check('trade buy match', buy_rows[0]['status'], 'MATCH')
    check('trade sell mismatch', sell_rows[0]['status'], 'MISMATCH')

    print(json.dumps({'status': 'OK', 'checks': checks}))


# --------------------------------------------------------------------------------------------

def main():
    parser = argparse.ArgumentParser(description=__doc__.split('\n')[0])
    sub = parser.add_subparsers(dest='command', required=True)

    fetch_parser = sub.add_parser('fetch', help='fetch NPC and item infobox facts from TibiaWiki into a cached snapshot')
    fetch_parser.add_argument('--cache', required=True)
    fetch_parser.set_defaults(func=cmd_fetch)

    compare_parser = sub.add_parser('compare', help='compare bundles against a fetched snapshot')
    compare_parser.add_argument('--snapshot', required=True)
    compare_parser.add_argument('--bundles', required=True, action='append', help='repeatable; a bundles directory per source')
    compare_parser.add_argument('--out', required=True)
    compare_parser.set_defaults(func=cmd_compare)

    self_test_parser = sub.add_parser('self-test', help='offline parser unit checks, no network')
    self_test_parser.set_defaults(func=cmd_self_test)

    args = parser.parse_args()
    args.func(args)


if __name__ == '__main__':
    main()
