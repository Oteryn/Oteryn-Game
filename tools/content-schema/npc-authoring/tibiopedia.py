"""Capture the NPC pages of Tibiopedia (tibiopedia.pl) and keep their trade facts.

Evidence tooling only. Tibiopedia is a Polish, player-observed Tibia reference; each NPC page states
whether the NPC trades ("Handel: tak/nie") and lists what it sells and buys, each item with its price.
It is the third wiki of OTERYN_NPC_AUTHORING_SCHEMA_V1 D13 (with TibiaWiki Fandom and TibiaWiki BR).

`fetch` reads the NPC page list from the site's sitemap, fetches each page and writes the facts file:
page URL and title, the SHA-256 of the fetched page, the trade flag and the trade lists (item name and
each row's price; null when the price is not a plain gold amount, e.g. "5 za 1gp"). No place, profession,
notes or other page text (D3). That file is committed.

Python stdlib only, one request at a time with a pause between pages, neutral User-Agent, no personal
data in any request. The site's robots.txt disallows only /panels/, /core/ and /ajax/ for ordinary
agents; the site asks for a language/layout choice first, which is posted once like a browser does.

Usage:
    python tibiopedia.py fetch --out <facts.json>
    python tibiopedia.py self-test
"""
import argparse
import hashlib
import html
import http.cookiejar
import json
import re
import time
import urllib.error
import urllib.parse
import urllib.request
from datetime import datetime, timezone
from pathlib import Path

SITE = 'https://tibiopedia.pl'
SITEMAP = SITE + '/sitemap.xml'
SETUP = SITE + '/setup/choose'
SETUP_FORM = b'tp_lang=pl&tp_layout=library'
USER_AGENT = 'OterynNpcAuthoring/1.0 (+https://github.com/Oteryn/Oteryn-Game)'
FACTS_SCHEMA = 'OTERYN_NPC_TIBIOPEDIA_FACTS/v1'
LICENSE_NOTE = 'Tibiopedia, player-observed reference data; trade facts only, used under OTERYN_NPC_AUTHORING_SCHEMA_V1 D13.'
PAUSE_SECONDS = 1.5
RETRIES = 4
MAX_PAGES = 5000
# a capture below these bounds is a failed setup, a changed sitemap or changed page markup, never facts
MIN_NPC_URLS = 1000
MIN_NPC_PAGES = 1000
MAX_SKIPPED_SHARE = 0.2
NPC_URL_RE = re.compile(r'<loc>(https://tibiopedia\.pl/npcs/[^<]+)</loc>')
OFFER_RE = re.compile(r'<a[^>]*href="https://tibiopedia\.pl/items/[^"]*">([^<]+)</a>\s*\(([^)]*)\)')
PRICE_RE = re.compile(r'([\d]+(?:[.,]\d+)?)(k*)gp')


def opener():
    jar = urllib.request.build_opener(urllib.request.HTTPCookieProcessor(http.cookiejar.CookieJar()))
    jar.addheaders = [('User-Agent', USER_AGENT)]
    jar.open(SETUP, data=SETUP_FORM, timeout=60).read()
    return jar


def get(jar, url):
    """One page, with retries on transient errors; the path is percent-encoded (names such as Schrödinger)."""
    parts = urllib.parse.urlsplit(url)
    url = urllib.parse.urlunsplit(parts._replace(path=urllib.parse.quote(urllib.parse.unquote(parts.path), safe="/'")))
    delay, last_exc = 5.0, None
    for attempt in range(RETRIES):
        try:
            with jar.open(url, timeout=60) as response:
                body = response.read()
            time.sleep(PAUSE_SECONDS)
            return body
        except (urllib.error.URLError, TimeoutError, OSError) as exc:
            last_exc = exc
            if attempt + 1 < RETRIES:
                time.sleep(delay)
                delay *= 2
    raise SystemExit(f'request failed after {RETRIES} attempts: {url}: {last_exc}')


def price_of(text):
    """A plain gold amount ("35gp", "1.5kgp") as an integer, else None."""
    match = PRICE_RE.fullmatch(text.replace(' ', ''))
    if not match:
        return None
    value = float(match.group(1).replace(',', '.')) * 1000 ** len(match.group(2))
    return int(value) if value == int(value) else None


def offers(block):
    rows = {}
    for match in OFFER_RE.finditer(block):
        rows.setdefault(html.unescape(match.group(1)).strip(), []).append(price_of(html.unescape(match.group(2))))
    return rows


def page_facts(page):
    """Title, trade flag and trade lists of one NPC page; None for a page with no trade flag (not an NPC)."""
    flag = re.search(r'Handel:\s*<strong>(.*?)</strong>', page, re.S)
    if flag is None:
        return None
    title = re.search(r'<title>(.*?) - Tibia', page, re.S)
    title = html.unescape(title.group(1)).strip() if title else None
    sells, buys = '', ''
    if 'id="npcTradeTab1Holder"' in page:
        sells = page.split('id="npcTradeTab1Holder"', 1)[1].split('id="npcTradeTab2Holder"', 1)[0]
    if 'id="npcTradeTab2Holder"' in page:
        buys = page.split('id="npcTradeTab2Holder"', 1)[1].split('<script', 1)[0]
    return {'title': title, 'name': re.sub(r'^NPC:\s*', '', title or '') or None,
            'trade': re.sub(r'<[^>]+>', '', flag.group(1)).strip(),
            'trades': {'SellToPlayer': offers(sells), 'BuyFromPlayer': offers(buys)}}


def cmd_fetch(args):
    fetched_at = datetime.now(timezone.utc).strftime('%Y-%m-%dT%H:%M:%SZ')
    jar = opener()
    urls = sorted(set(NPC_URL_RE.findall(get(jar, SITEMAP).decode('utf-8'))) - {SITE + '/npcs/search'})
    if not MIN_NPC_URLS <= len(urls) <= MAX_PAGES:
        raise SystemExit(f'{len(urls)} NPC URLs in the sitemap, outside {MIN_NPC_URLS}..{MAX_PAGES}')
    pages, skipped = [], 0
    for index, url in enumerate(urls):
        body = get(jar, url)
        facts = page_facts(body.decode('utf-8', 'replace'))
        if facts is None:
            skipped += 1
            continue
        pages.append({'url': url, 'sha256': hashlib.sha256(body).hexdigest(), **facts})
        if index % 100 == 0:
            print(f'{index}/{len(urls)} {url}', flush=True)
    if len(pages) < MIN_NPC_PAGES or skipped > MAX_SKIPPED_SHARE * len(urls):
        raise SystemExit(f'{len(pages)} NPC pages, {skipped} without a trade flag: the capture is incomplete')
    pages.sort(key=lambda p: p['url'])
    digest = hashlib.sha256(''.join(p['url'] + p['sha256'] for p in pages).encode('utf-8')).hexdigest()
    result = {'schema': FACTS_SCHEMA, 'site': SITE, 'fetched_at': fetched_at, 'license': LICENSE_NOTE,
              'counts': {'sitemap_urls': len(urls), 'npc_pages': len(pages), 'not_npc_pages': skipped,
                         'trading': sum(p['trade'] == 'tak' for p in pages)},
              'snapshot_pages_digest': digest, 'pages': pages}
    Path(args.out).write_text(json.dumps(result, ensure_ascii=False, indent=1, sort_keys=True) + '\n', encoding='utf-8')
    print(json.dumps(result['counts']))


def cmd_self_test(_args):
    page = ('<title>NPC: Ahmet - Tibia</title> Handel: <strong>tak</strong>'
            '<div id="npcTradeTab1Holder"><a href="https://tibiopedia.pl/items/Machete">Machete</a> (35gp)'
            '<a class="x" href="https://tibiopedia.pl/items/Crusher">Crusher</a> (1.5kgp)'
            '<a href="https://tibiopedia.pl/items/Blueberry">Blueberry</a> (5 za 1gp)</div>'
            '<div id="npcTradeTab2Holder"><a href="https://tibiopedia.pl/items/Machete">Machete</a> (6gp)'
            '<a href="https://tibiopedia.pl/items/Machete">Machete</a> (7 gp)</div><script>x</script>')
    facts = page_facts(page)
    assert facts['name'] == 'Ahmet' and facts['trade'] == 'tak', facts
    assert facts['trades']['SellToPlayer'] == {'Machete': [35], 'Crusher': [1500], 'Blueberry': [None]}, facts
    assert facts['trades']['BuyFromPlayer'] == {'Machete': [6, 7]}, facts
    assert page_facts('<title>Ab\'Dendriel - Tibia</title>') is None
    assert price_of('1,5kgp') == 1500 and price_of('12gp') == 12 and price_of('darmo') is None
    print('self-test ok')


def main():
    parser = argparse.ArgumentParser(description=__doc__.split('\n')[0])
    commands = parser.add_subparsers(dest='command', required=True)
    fetch_parser = commands.add_parser('fetch', help='fetch every NPC page and write the facts file')
    fetch_parser.add_argument('--out', required=True)
    fetch_parser.set_defaults(func=cmd_fetch)
    commands.add_parser('self-test', help='offline parser checks').set_defaults(func=cmd_self_test)
    args = parser.parse_args()
    args.func(args)


if __name__ == '__main__':
    main()
