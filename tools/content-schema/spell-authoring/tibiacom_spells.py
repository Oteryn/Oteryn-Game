"""Spell facts from the official tibia.com spell library (owner decision S15).

Evidence tooling only. tibia.com answers a Cloudflare browser check from the build container, so the
capture runs on a hosted runner in a real (headless) Chromium through Playwright, one page per second,
with no user-agent spoofing or challenge bypass: if the site does not serve the page, the capture fails.
The artifact keeps the table cells of each page (cut to 200 characters) with the page URL and the
SHA-256 of the served HTML, never the raw page; `facts` maps the cells to Fandom field names.

Usage (hosted runner; needs `pip install playwright` and `python -m playwright install chromium`):
    python tibiacom_spells.py fetch --out <dir>/tibiacom-spell-tables.json
    python tibiacom_spells.py facts --artifact tibiacom-spell-tables.json \
        --out samples/tibiacom-spell-facts-<date>.json
    python tibiacom_spells.py self-test
"""
import argparse
import hashlib
import html
import json
import re
import sys
import time
import urllib.parse
from datetime import datetime, timezone
from pathlib import Path

LIST_URL = 'https://www.tibia.com/library/?subtopic=spells'
THROTTLE_SECONDS = 1.0
MAX_CELL = 200
CHALLENGE_WAIT_SECONDS = 60
LICENSE_NOTE = ('tibia.com (CipSoft GmbH); only single facts from the spell library tables with the page URL '
                'and the SHA-256 of the served page are recorded (owner decision S15), never page text or images.')
# In-page extraction: every table as rows of trimmed cell texts; nothing else leaves the browser.
TABLES_JS = '''() => Array.from(document.querySelectorAll('table')).map(t =>
    Array.from(t.rows).map(r => Array.from(r.cells).map(c => (c.innerText || '').replace(/\\s+/g, ' ').trim())))'''
LINKS_JS = '''() => Array.from(document.querySelectorAll('a[href*="subtopic=spells"][href*="spell="]')).map(a => a.href)'''


def spell_url(href):
    """Canonical detail URL (spell= parameter only), or None for list/filter links."""
    query = urllib.parse.parse_qs(urllib.parse.urlparse(href).query)
    spell = query.get('spell', [''])[0]
    if not spell or query.get('subtopic', [''])[0] != 'spells':
        return None
    return f'{LIST_URL}&spell={urllib.parse.quote(spell)}'


def cut_tables(tables):
    return [[[cell[:MAX_CELL] for cell in row] for row in table if any(row)] for table in tables]


def fetch(out):
    from playwright.sync_api import sync_playwright  # hosted runner only

    fetched = datetime.now(timezone.utc).strftime('%Y-%m-%dT%H:%M:%SZ')
    pages = []
    with sync_playwright() as playwright:
        browser = playwright.chromium.launch()
        page = browser.new_page()

        def load(url):
            page.goto(url, wait_until='domcontentloaded', timeout=90_000)
            deadline = time.monotonic() + CHALLENGE_WAIT_SECONDS
            while 'just a moment' in page.title().lower():
                if time.monotonic() > deadline:
                    raise SystemExit(f'tibia.com did not serve {url} (browser check not passed)')
                page.wait_for_timeout(1000)
            page.wait_for_load_state('load')
            body = page.content().encode('utf-8')
            return {'url': url, 'sha256': hashlib.sha256(body).hexdigest(),
                    'tables': cut_tables(page.evaluate(TABLES_JS))}

        listing = load(LIST_URL)
        links = sorted({u for u in (spell_url(h) for h in page.evaluate(LINKS_JS)) if u})
        if not links:
            raise SystemExit('tibia.com: the spell library lists no spells (layout change?)')
        pages.append(listing)
        for index, url in enumerate(links, 1):
            time.sleep(THROTTLE_SECONDS)
            pages.append(load(url))
            print(f'{index}/{len(links)} {url}', file=sys.stderr)
        browser.close()
    document = {'schema': 'OTERYN_TIBIACOM_SPELL_TABLES/v1', 'list_url': LIST_URL, 'fetched': fetched,
                'license': LICENSE_NOTE, 'pages': pages}
    out.parent.mkdir(parents=True, exist_ok=True)
    out.write_text(json.dumps(document, ensure_ascii=False, indent=1) + '\n', encoding='utf-8')


# ------------------------------------------------------------------------------------------------
# Facts: table cells -> Fandom field names (labels as the library prints them)
# ------------------------------------------------------------------------------------------------
LABELS = {'name': 'name', 'formula': 'words', 'words': 'words', 'vocation': 'voc', 'group': 'group',
          'type': 'type', 'cooldown': 'cooldown', 'soul points': 'soul', 'amount': 'amount',
          'damage type': 'damagetype', 'magic type': 'damagetype', 'exp lvl': 'levelrequired',
          'level': 'levelrequired', 'mana': 'mana', 'premium': 'premium', 'price': 'spellcost',
          'magic level': 'mlrequired', 'mlvl': 'mlrequired', 'rune': 'rune', 'charges': 'charges',
          'range': 'spellrange'}


def seconds(value):
    """'2s' / '2 s' / '1m 30s' / '2h' -> whole seconds as text ('2'); None when no duration."""
    total, matched = 0.0, False
    for amount, unit in re.findall(r'(\d+(?:[.,]\d+)?)\s*(h|m|min|s)\b', value.lower()):
        total += float(amount.replace(',', '.')) * {'h': 3600, 'm': 60, 'min': 60, 's': 1}[unit]
        matched = True
    if not matched:
        return None
    return str(int(total)) if total == int(total) else str(total)


def cooldown_fields(value):
    """'2s (Group: Attack 2s)' style cells: own cooldown, then each 'group time' pair."""
    fields = {}
    own = re.split(r'\(', value, maxsplit=1)[0]
    if seconds(own) is not None:
        fields['cooldown'] = seconds(own)
    groups = re.findall(r'([A-Za-z][A-Za-z ]*?)\s*:?\s*(\d+(?:[.,]\d+)?\s*(?:h|min|m|s))\b', value[len(own):])
    for index, (group, amount) in enumerate(groups[:2]):
        suffix = '' if index == 0 else '2'
        fields['cooldowngroup' + suffix] = seconds(amount)
        name = re.sub(r'^(?:group|groups)\s*', '', group.strip(), flags=re.I).strip()
        if name:
            fields['subclass' if index == 0 else 'secondarygroup'] = name
    return fields


def label_value_rows(tables):
    """(label, value) from two-cell rows 'Label:' | 'value' of every table."""
    for table in tables:
        for row in table:
            if len(row) == 2 and row[0].endswith(':'):
                yield row[0][:-1].strip().lower(), row[1].strip()


def page_facts(page):
    """Spell row, plus a rune row when the page carries rune information."""
    spell, rune, in_rune = {}, {}, False
    for table in page['tables']:
        header = ' '.join(table[0]).lower() if table and table[0] else ''
        in_rune = in_rune or 'rune information' in header
        target = rune if 'rune information' in header else spell
        for label, value in label_value_rows([table]):
            field = LABELS.get(label)
            if not field or not value or value in ('-', '--'):
                continue
            if field == 'cooldown':
                target.update(cooldown_fields(value))
            elif field == 'group':
                target.setdefault('subclass', value)
            else:
                target[field] = value
    if 'words' in spell:
        spell['words'] = html.unescape(spell['words']).strip('"\' ')
    rows = [{'template': 'Infobox Spell', 'title': spell.get('name', page['url']), 'url': page['url'],
             'content_sha256': page['sha256'], 'fields': spell}]
    if rune:
        rune.setdefault('name', spell.get('name', ''))
        rows.append({'template': 'Infobox Object', 'title': rune['name'], 'url': page['url'],
                     'content_sha256': page['sha256'], 'fields': rune})
    return rows


def facts(artifact):
    pages = []
    for page in artifact['pages']:
        if page['url'] == artifact['list_url']:
            continue
        pages.extend(page_facts(page))
    return {'schema': 'OTERYN_SPELL_WIKI_FACTS/v1', 'wiki': 'tibiacom', 'api': artifact['list_url'],
            'license': LICENSE_NOTE, 'target_cut': artifact['fetched'][:10],
            'cut_rule': 'pages as served at the fetch time ' + artifact['fetched'], 'pages': pages}


def self_test():
    assert spell_url('https://www.tibia.com/library/?subtopic=spells&spell=icestrike') == LIST_URL + '&spell=icestrike'
    assert spell_url('https://www.tibia.com/library/?subtopic=spells&vocation=druid') is None
    assert seconds('2s') == '2' and seconds('1m 30s') == '90' and seconds('none') is None
    assert cooldown_fields('2s (Group: Attack 2s)') == {'cooldown': '2', 'cooldowngroup': '2', 'subclass': 'Attack'}
    page = {'url': LIST_URL + '&spell=suddendeathrune', 'sha256': 'x', 'tables': [
        [['Spell Information'], ['Name:', 'Sudden Death Rune'], ['Formula:', 'adori gran mort'],
         ['Vocation:', 'Sorcerer'], ['Group:', 'Support'], ['Cooldown:', '2s'], ['Soul Points:', '5'],
         ['Amount:', '3'], ['Exp Lvl:', '45'], ['Mana:', '985'], ['Premium:', 'no'], ['Description:', 'prose']],
        [['Rune Information'], ['Vocation:', 'all'], ['Group:', 'Attack'], ['Magic Type:', 'Death'],
         ['Exp Lvl:', '45'], ['Magic Level:', '15']]]}
    spell, rune = page_facts(page)
    assert spell['fields'] == {'name': 'Sudden Death Rune', 'words': 'adori gran mort', 'voc': 'Sorcerer',
                               'subclass': 'Support', 'cooldown': '2', 'soul': '5', 'amount': '3',
                               'levelrequired': '45', 'mana': '985', 'premium': 'no'}, spell['fields']
    assert rune['fields'] == {'voc': 'all', 'subclass': 'Attack', 'damagetype': 'Death', 'levelrequired': '45',
                              'mlrequired': '15', 'name': 'Sudden Death Rune'}, rune['fields']
    assert 'prose' not in json.dumps([spell, rune])
    assert cut_tables([[['x' * 300], []]]) == [[['x' * 200]]]
    print('tibiacom_spells self-test: ok')
    return 0


def main(argv=None):
    parser = argparse.ArgumentParser(description=__doc__.splitlines()[0])
    parser.add_argument('command', choices=('fetch', 'facts', 'self-test'))
    parser.add_argument('--artifact', type=Path)
    parser.add_argument('--out', type=Path)
    args = parser.parse_args(argv)
    if args.command == 'self-test':
        return self_test()
    if args.command == 'fetch':
        fetch(args.out)
        return 0
    from wiki_spells import write_lines
    write_lines(args.out, facts(json.loads(args.artifact.read_text(encoding='utf-8'))), 'pages')
    return 0


if __name__ == '__main__':
    sys.exit(main())
