"""Capture the NPC pages of TibiaWiki BR (tibiawiki.com.br) and extract their compared facts.

Evidence tooling only. TibiaWiki BR is a player-observed reference source; its NPC pages carry the
position, trade lists and in-game dialogue transcripts of Tibia Global NPCs, including recent ones
that the Fandom wiki only stubs.

`fetch` writes the raw snapshot: each page's id, exact revision, timestamp, the SHA-256 of its
wikitext and the wikitext itself. It stays a CI artifact and is never committed (wiki prose is not
bulk-copied, OTERYN_WORLD_PROJECT_SOURCE_PROFILE_V2_DECISION).

`facts` reduces a snapshot to the facts that are compared (OTERYN_NPC_AUTHORING_SCHEMA_V1 D3): page and
revision ids, the SHA-256 of each raw page, infobox name, `implemented` and `removed` versions, map
positions, trade lists (item name and explicit price) and the lines the NPC itself speaks in the
transcript, which are Tibia NPC text kept as reference data (D9). No notes, descriptions or other wiki
prose. That file is committed.

Pages: every namespace-0 member of `Categoria:NPCs no Tibia` and its subcategories, plus every
namespace-0 subpage (`<NPC>/...`) of those pages. Python stdlib only, <= 2 requests/s, neutral
User-Agent, no personal data in any request.

The site answers this repository's runners (the G4 non-Item capture used the same API); it may
refuse other networks.

Usage:
    python wiki_br.py fetch --out <dir>/tibiawiki-br-npc-snapshot.json
    python wiki_br.py facts --snapshot <dir>/tibiawiki-br-npc-snapshot.json --out <facts.json>
    python wiki_br.py self-test
"""
import argparse
import hashlib
import json
import re
import time
import urllib.error
import urllib.parse
import urllib.request
from datetime import datetime, timezone
from pathlib import Path

API = 'https://www.tibiawiki.com.br/api.php'
USER_AGENT = 'OterynNpcAuthoring/1.0 (+https://github.com/Oteryn/Oteryn-Game)'
ROOT_CATEGORY = 'Categoria:NPCs no Tibia'
SNAPSHOT_SCHEMA = 'OTERYN_NPC_TIBIAWIKI_BR_SNAPSHOT/v1'
FACTS_SCHEMA = 'OTERYN_NPC_TIBIAWIKI_BR_FACTS/v1'
LICENSE_NOTE = 'TibiaWiki BR, player-observed reference data; used under OTERYN_NPC_AUTHORING_SCHEMA_V1 D9.'
THROTTLE_SECONDS = 0.5  # <= 2 requests/s
RETRIES = 4
BATCH = 50
MAX_CATEGORIES = 500
MAX_PAGES = 20000


def api(params, retries=RETRIES):
    """One MediaWiki API call with retries on transient errors."""
    url = API + '?' + urllib.parse.urlencode({**params, 'format': 'json', 'formatversion': '2'})
    delay, last_exc = 1.0, None
    for attempt in range(retries):
        try:
            request = urllib.request.Request(url, headers={'User-Agent': USER_AGENT})
            with urllib.request.urlopen(request, timeout=60) as response:
                data = json.load(response)
            time.sleep(THROTTLE_SECONDS)
            if 'error' in data:
                raise SystemExit(f'API error: {data["error"]}')
            return data
        except (urllib.error.URLError, TimeoutError, OSError) as exc:
            last_exc = exc
            if attempt + 1 < retries:
                time.sleep(delay)
                delay *= 2
    raise SystemExit(f'API request failed after {retries} attempts: {last_exc}')


def continued(params, key):
    """Every list item of a continued query."""
    params = dict(params)
    while True:
        data = api(params)
        yield from data.get('query', {}).get(key, [])
        if 'continue' not in data:
            return
        params.update(data['continue'])


def category_pages():
    """Namespace-0 pages of the root category tree, as {pageid: title}."""
    pages, seen, queue = {}, {ROOT_CATEGORY}, [ROOT_CATEGORY]
    while queue:
        category = queue.pop(0)
        for member in continued({'action': 'query', 'list': 'categorymembers', 'cmtitle': category,
                                 'cmtype': 'page|subcat', 'cmlimit': 500}, 'categorymembers'):
            if member['ns'] == 14 and member['title'] not in seen:
                seen.add(member['title'])
                queue.append(member['title'])
                if len(seen) > MAX_CATEGORIES:
                    raise SystemExit('category tree exceeds the category bound')
            elif member['ns'] == 0:
                pages[member['pageid']] = member['title']
    return pages, sorted(seen)


def subpages(parents):
    """Namespace-0 pages titled `<parent>/...` for a known parent title, as {pageid: title}."""
    found = {}
    for page in continued({'action': 'query', 'list': 'allpages', 'apnamespace': 0, 'aplimit': 500},
                          'allpages'):
        title = page['title']
        if '/' in title and title.split('/', 1)[0] in parents:
            found[page['pageid']] = title
    return found


def fetch_revisions(page_ids):
    """Current revision of each page id: {pageid: record}."""
    records = {}
    ids = sorted(page_ids)
    for start in range(0, len(ids), BATCH):
        chunk = ids[start:start + BATCH]
        data = api({'action': 'query', 'pageids': '|'.join(map(str, chunk)), 'prop': 'revisions',
                    'rvprop': 'ids|timestamp|content', 'rvslots': 'main'})
        for page in data.get('query', {}).get('pages', []):
            record = page_record(page)
            if record is not None:
                records[record['pageid']] = record
    return records


def page_record(page):
    """A snapshot record of one API page, or None for a missing page."""
    revisions = page.get('revisions') or []
    if page.get('missing') or len(revisions) != 1:
        return None
    revision = revisions[0]
    text = revision['slots']['main']['content']
    return {'pageid': page['pageid'], 'title': page['title'], 'revid': revision['revid'],
            'timestamp': revision['timestamp'], 'sha256': hashlib.sha256(text.encode('utf-8')).hexdigest(),
            'wikitext': text}


def build_snapshot(npc_pages, sub_pages, records, categories, fetched_at):
    enumerated = {**npc_pages, **sub_pages}
    missing = [{'pageid': pageid, 'title': enumerated[pageid]} for pageid in sorted(enumerated) if pageid not in records]
    pages = []
    for pageid in sorted(records):
        record = dict(records[pageid])
        record['role'] = 'npc' if pageid in npc_pages else 'subpage'
        pages.append(record)
    digest = hashlib.sha256('\n'.join(f'{p["pageid"]}:{p["revid"]}:{p["sha256"]}' for p in pages).encode()).hexdigest()
    return {
        'schema': SNAPSHOT_SCHEMA, 'license': LICENSE_NOTE, 'api': API, 'root_category': ROOT_CATEGORY,
        'fetched_at': fetched_at, 'categories': categories,
        'counts': {'categories': len(categories), 'npc_pages': len(npc_pages), 'subpages': len(sub_pages),
                   'captured': len(pages), 'missing': len(missing)},
        'pages_digest': digest, 'pages': pages, 'missing_pages': missing,
    }


def cmd_fetch(args):
    npc_pages, categories = category_pages()
    if not npc_pages or len(npc_pages) > MAX_PAGES:
        raise SystemExit(f'unexpected NPC page count {len(npc_pages)}')
    sub_pages = {pageid: title for pageid, title in subpages(set(npc_pages.values())).items()
                 if pageid not in npc_pages}
    records = fetch_revisions(set(npc_pages) | set(sub_pages))
    missing = (set(npc_pages) | set(sub_pages)) - set(records)
    if missing:  # a page can lose its revision between enumeration and fetch; retry once, then record it
        records.update(fetch_revisions(missing))
    snapshot = build_snapshot(npc_pages, sub_pages, records, categories,
                              datetime.now(timezone.utc).strftime('%Y-%m-%dT%H:%M:%SZ'))
    out = Path(args.out)
    out.parent.mkdir(parents=True, exist_ok=True)
    out.write_text(json.dumps(snapshot, ensure_ascii=False, indent=1, sort_keys=True) + '\n', encoding='utf-8')
    print(json.dumps({'out': str(out), 'counts': snapshot['counts'], 'pages_digest': snapshot['pages_digest']}))


# -- facts ---------------------------------------------------------------------------------------
MAPA = re.compile(r'\{\{\s*[Mm]apa\s*\|\s*(\d+)\s*,\s*(\d+)\s*,\s*(\d+)')
TRADES = re.compile(r'\{\{\s*Trades/(Buy|Sell)(.*?)\}\}', re.S)
LINK = re.compile(r'\[\[(?:[^\]|]*\|)?([^\]]*)\]\]')
# after links and bold/italic markup are removed, a transcript line is `Speaker: text`; the speaker may be
# written plain, bold, italic or as a link, before or around the colon, and may contain an apostrophe
SPEAKER = re.compile(r"\s*([^:<>{}|]{1,60}?)\s*:\s*(.*)$")
BREAK = re.compile(r'<\s*/?\s*br\s*/?\s*>', re.I)
# structural wiki/HTML markup that is never part of what the NPC says (a closing infobox, spoiler, paragraph ...)
STRUCTURE = re.compile(r'</?\s*(?:spoiler|p|div|span|small|big|center|noinclude|includeonly|onlyinclude|nowiki|ref|s|u|b|i)\b[^>]*>'
                       r'|\}\}\s*$', re.I)
PRICE = re.compile(r"^(?:''')?\s*(\d{1,3}(?:[ .]\d{3})+|\d+)\b")
FIELD = re.compile(r'^\|\s*([a-z0-9_]+)\s*=(.*)$', re.M)


def infobox_field(wikitext, name):
    for match in FIELD.finditer(wikitext):
        if match.group(1) == name:
            return match.group(2).strip()
    return ''


def price_of(field):
    """An explicit price such as `200`, `1 000`, `1.000`, `'''1 000'''` or `200 gp`; None when the field has none."""
    match = PRICE.match(field.strip())
    return int(re.sub(r'[ .]', '', match.group(1))) if match else None


def unmarked(text):
    return LINK.sub(r'\1', text).replace("'''", '').replace("''", '')


def fold(text):
    return re.sub(r'\s+', ' ', text).strip().casefold()


def page_facts(page):
    wikitext = page['wikitext']
    name = infobox_field(wikitext, 'name') or page['title']
    trades = {'BuyFromPlayer': {}, 'SellToPlayer': {}}
    for kind, body in TRADES.findall(wikitext):
        direction = 'SellToPlayer' if kind == 'Sell' else 'BuyFromPlayer'
        for part in LINK.sub(r'\1', body).split('|')[1:]:  # links first, so a piped link is not a separator
            fields = [field.strip() for field in part.split(',')]
            # `Blood;Vial of Blood` names the content page, then the offer's in-game name
            fields[0] = fields[0].rsplit(';', 1)[-1].strip()
            if fields[0]:
                trades[direction].setdefault(fields[0], price_of(fields[1]) if len(fields) > 1 else None)
    # a qualified page (`Hyacinth (NPC)`) labels its turns with the plain name
    speakers = {fold(re.sub(r'\s*\([^()]*\)$', '', label)) for label in (page['title'], name)}
    speakers |= {fold(page['title']), fold(name)}
    lines = []
    for raw in wikitext.split('\n'):
        # one physical line can hold several turns separated by <br>; a segment without a speaker
        # continues the turn before it on the same line
        speaker = None
        for segment in BREAK.split(unmarked(raw)):
            segment = re.sub(r'\s+', ' ', STRUCTURE.sub('', segment)).strip()
            match = SPEAKER.match(segment)
            if match:
                speaker, text = fold(match.group(1)), match.group(2).strip()
                repeated = SPEAKER.match(text)
                while repeated and fold(repeated.group(1)) == speaker:  # `Name: Name: text` repeats the label
                    text = repeated.group(2).strip()
                    repeated = SPEAKER.match(text)
                if speaker in speakers and text:
                    lines.append(text)
            elif segment and speaker in speakers and lines:
                lines[-1] = f'{lines[-1]} {segment}'
    return {'pageid': page['pageid'], 'revid': page['revid'], 'timestamp': page['timestamp'],
            'sha256': page['sha256'], 'title': page['title'], 'role': page['role'], 'name': name,
            'implemented': infobox_field(wikitext, 'implemented'), 'removed': infobox_field(wikitext, 'removed'),
            'positions': sorted({(int(x), int(y), int(z)) for x, y, z in MAPA.findall(wikitext)}),
            'trades': trades, 'npc_lines': lines}


def build_facts(snapshot, snapshot_sha256):
    """`snapshot_sha256` is the SHA-256 of the raw snapshot file; it binds every copied value (titles, timestamps,
    roles, the missing-page inventory) to the capture artifact, beyond what `pages_digest` covers."""
    if snapshot['schema'] != SNAPSHOT_SCHEMA:
        raise SystemExit('not a TibiaWiki BR snapshot')
    missing = snapshot.get('missing_pages', [])
    counts = snapshot['counts']
    if (counts['captured'] != len(snapshot['pages']) or counts['missing'] != len(missing)
            or {p['pageid'] for p in missing} & {p['pageid'] for p in snapshot['pages']}):
        raise SystemExit('snapshot counts or missing-page inventory are inconsistent')
    for page in snapshot['pages']:
        if hashlib.sha256(page['wikitext'].encode('utf-8')).hexdigest() != page['sha256']:
            raise SystemExit(f'page {page["pageid"]} wikitext does not match its sha256')
    ordered = sorted(snapshot['pages'], key=lambda p: p['pageid'])
    digest = hashlib.sha256('\n'.join(f'{p["pageid"]}:{p["revid"]}:{p["sha256"]}' for p in ordered).encode()).hexdigest()
    if digest != snapshot['pages_digest']:
        raise SystemExit('snapshot pages do not match its pages_digest')
    pages = [page_facts(page) for page in sorted(snapshot['pages'], key=lambda p: p['pageid'])]
    return {'schema': FACTS_SCHEMA, 'license': LICENSE_NOTE, 'api': snapshot['api'],
            'fetched_at': snapshot['fetched_at'], 'snapshot_pages_digest': snapshot['pages_digest'],
            'snapshot_sha256': snapshot_sha256, 'missing_pages': missing,
            'counts': {'pages': len(pages), 'with_positions': sum(1 for p in pages if p['positions']),
                       'with_trades': sum(1 for p in pages if any(p['trades'].values())),
                       'with_npc_lines': sum(1 for p in pages if p['npc_lines']),
                       'removed': sum(1 for p in pages if p['removed'])},
            'pages': pages}


def cmd_facts(args):
    data = Path(args.snapshot).read_bytes()
    facts = build_facts(json.loads(data), hashlib.sha256(data).hexdigest())
    out = Path(args.out)
    out.parent.mkdir(parents=True, exist_ok=True)
    out.write_text(json.dumps(facts, ensure_ascii=False, indent=1, sort_keys=True) + '\n', encoding='utf-8')
    print(json.dumps({'out': str(out), 'counts': facts['counts']}))


def cmd_self_test(_args):
    page = {'pageid': 7, 'title': 'Goldro', 'revisions': [
        {'revid': 11, 'timestamp': '2026-09-01T00:00:00Z', 'slots': {'main': {'content': 'Olá'}}}]}
    record = page_record(page)
    assert record['sha256'] == hashlib.sha256('Olá'.encode('utf-8')).hexdigest(), record
    assert page_record({'pageid': 8, 'title': 'X', 'missing': True}) is None
    sub = page_record({'pageid': 9, 'title': 'Goldro/Diálogos', 'revisions': [
        {'revid': 12, 'timestamp': '2026-09-01T00:00:00Z', 'slots': {'main': {'content': 'a'}}}]})
    snapshot = build_snapshot({7: 'Goldro', 10: 'Gone'}, {9: 'Goldro/Diálogos'}, {9: sub, 7: record},
                              [ROOT_CATEGORY], 'T')
    assert [p['pageid'] for p in snapshot['pages']] == [7, 9]
    assert [p['role'] for p in snapshot['pages']] == ['npc', 'subpage']
    assert snapshot['counts'] == {'categories': 1, 'npc_pages': 2, 'subpages': 1, 'captured': 2, 'missing': 1}
    assert snapshot['missing_pages'] == [{'pageid': 10, 'title': 'Gone'}]
    again = build_snapshot({7: 'Goldro', 10: 'Gone'}, {9: 'Goldro/Diálogos'}, {7: record, 9: sub}, [ROOT_CATEGORY], 'T')
    assert again == snapshot
    text = ("{{Infobox_NPC\n| name = Goldro\n| implemented = 15.30\n| removed = \n"
            "| location = [[Salgadora]] ({{Mapa|34055,32503,7:2|aqui}}).\n| notes = Long wiki prose.\n"
            "| sells = {{Trades/Sell\n| Bread,4\n| [[Cheese]]\n| Cot, 200 [[Gold Coins|gp]]\n| Fire Sword, '''1 000'''\n| Blood;Vial of Blood\n| Beer; Mug of Beer, 3}}\n"
            "| falas = \n''Jogador:'' '''Hi'''</br>\n"
            "'''Goldro:''' Hello, ''Jogador''. Ask about [[Salgadora|the town]].</br>\n"
            "'''Goldro''': Bold name, colon outside.</br>\n'''[[Goldro]]:''' Linked name.</br>\n"
            "[[Goldro]]: Link form.</br>\nGoldro: Plain form.</br>\n''Goldro:'' Italic form.</br>\n"
            "[[Other]]: Not mine.\n'''Goldro:''' One.<br>Jogador: Accident<br>'''Goldro:''' Two.<br>still two.\n"
            "'''Goldro:''' Goldro: Repeated label.\nGoldro: Bye.</spoiler></p></noinclude>}}")
    facts = page_facts({**page_record({'pageid': 7, 'title': 'Goldro', 'revisions': [
        {'revid': 11, 'timestamp': 'T', 'slots': {'main': {'content': text}}}]}), 'role': 'npc'})
    assert facts['positions'] == [(34055, 32503, 7)], facts
    assert facts['trades'] == {'BuyFromPlayer': {}, 'SellToPlayer': {
        'Bread': 4, 'Cheese': None, 'Cot': 200, 'Fire Sword': 1000, 'Vial of Blood': None, 'Mug of Beer': 3}}, facts
    assert facts['npc_lines'] == ['Hello, Jogador. Ask about the town.', 'Bold name, colon outside.',
                                  'Linked name.', 'Link form.', 'Plain form.', 'Italic form.', 'One.', 'Two. still two.',
                                  'Repeated label.', 'Bye.'], facts
    apostrophe = page_facts({**page_record({'pageid': 8, 'title': "Lee'Delle", 'revisions': [
        {'revid': 1, 'timestamp': 'T', 'slots': {'main': {'content': "'''Lee'Delle:''' Welcome."}}}]}), 'role': 'npc'})
    assert apostrophe['npc_lines'] == ['Welcome.'], apostrophe
    qualified = page_facts({**page_record({'pageid': 9, 'title': 'Hyacinth (NPC)', 'revisions': [
        {'revid': 1, 'timestamp': 'T', 'slots': {'main': {'content': "[[Hyacinth (NPC)|Hyacinth]]: Greetings."}}}]}),
        'role': 'npc'})
    assert qualified['npc_lines'] == ['Greetings.'], qualified
    assert build_facts(snapshot, 'f' * 64)['snapshot_sha256'] == 'f' * 64
    dropped = {**snapshot, 'pages': snapshot['pages'][:1], 'counts': {**snapshot['counts'], 'captured': 1}}
    inconsistent = {**snapshot, 'missing_pages': []}
    for tampered in (dropped, inconsistent):
        try:
            build_facts(tampered, 'f' * 64)
        except SystemExit:
            pass
        else:
            raise AssertionError('a snapshot that does not match its digest or its own counts must be rejected')
    assert 'prose' not in json.dumps(facts), facts
    print('wiki_br self-test: PASS')


def main():
    parser = argparse.ArgumentParser(description=__doc__.split('\n')[0])
    sub = parser.add_subparsers(dest='command', required=True)
    fetch_parser = sub.add_parser('fetch', help='capture the NPC pages into a snapshot')
    fetch_parser.add_argument('--out', required=True)
    fetch_parser.set_defaults(func=cmd_fetch)
    facts_parser = sub.add_parser('facts', help='reduce a snapshot to its compared facts')
    facts_parser.add_argument('--snapshot', required=True)
    facts_parser.add_argument('--out', required=True)
    facts_parser.set_defaults(func=cmd_facts)
    sub.add_parser('self-test', help='offline checks, no network').set_defaults(func=cmd_self_test)
    args = parser.parse_args()
    args.func(args)


if __name__ == '__main__':
    main()
