"""Capture the NPC pages of TibiaWiki BR (tibiawiki.com.br) as an exact-revision wikitext snapshot.

Evidence tooling only. TibiaWiki BR is a player-observed reference source; its NPC pages carry the
outfit, position and in-game dialogue of Tibia Global NPCs, including recent ones that the Fandom
wiki only stubs. The snapshot records each page's id, exact revision, timestamp, the SHA-256 of its
wikitext and the wikitext itself, so any later comparison can be re-checked against the same bytes.
Tibia NPC text is reference data under OTERYN_NPC_AUTHORING_SCHEMA_V1 D9; the snapshot is a CI
artifact, not a committed corpus.

Pages: every namespace-0 member of `Categoria:NPCs no Tibia` and its subcategories, plus every
namespace-0 subpage (`<NPC>/...`) of those pages. Python stdlib only, <= 2 requests/s, neutral
User-Agent, no personal data in any request.

The site answers this repository's runners (the G4 non-Item capture used the same API); it may
refuse other networks.

Usage:
    python wiki_br.py fetch --out <dir>/tibiawiki-br-npc-snapshot.json
    python wiki_br.py self-test
"""
import argparse
import hashlib
import json
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
                   'captured': len(pages), 'missing': len(npc_pages) + len(sub_pages) - len(pages)},
        'pages_digest': digest, 'pages': pages,
    }


def cmd_fetch(args):
    npc_pages, categories = category_pages()
    if not npc_pages or len(npc_pages) > MAX_PAGES:
        raise SystemExit(f'unexpected NPC page count {len(npc_pages)}')
    sub_pages = {pageid: title for pageid, title in subpages(set(npc_pages.values())).items()
                 if pageid not in npc_pages}
    records = fetch_revisions(set(npc_pages) | set(sub_pages))
    snapshot = build_snapshot(npc_pages, sub_pages, records, categories,
                              datetime.now(timezone.utc).strftime('%Y-%m-%dT%H:%M:%SZ'))
    out = Path(args.out)
    out.parent.mkdir(parents=True, exist_ok=True)
    out.write_text(json.dumps(snapshot, ensure_ascii=False, indent=1, sort_keys=True) + '\n', encoding='utf-8')
    print(json.dumps({'out': str(out), 'counts': snapshot['counts'], 'pages_digest': snapshot['pages_digest']}))


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
    again = build_snapshot({7: 'Goldro', 10: 'Gone'}, {9: 'Goldro/Diálogos'}, {7: record, 9: sub}, [ROOT_CATEGORY], 'T')
    assert again == snapshot
    print('wiki_br self-test: PASS')


def main():
    parser = argparse.ArgumentParser(description=__doc__.split('\n')[0])
    sub = parser.add_subparsers(dest='command', required=True)
    fetch_parser = sub.add_parser('fetch', help='capture the NPC pages into a snapshot')
    fetch_parser.add_argument('--out', required=True)
    fetch_parser.set_defaults(func=cmd_fetch)
    sub.add_parser('self-test', help='offline checks, no network').set_defaults(func=cmd_self_test)
    args = parser.parse_args()
    args.func(args)


if __name__ == '__main__':
    main()
