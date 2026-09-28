"""Spell facts from tibiopedia.pl, the third player-observed reference (owner decision S12).

Evidence tooling only. tibiopedia.pl is "all rights reserved": the committed facts file keeps single
facts (level, mana, soul, cooldowns, base power, ...) with the page URL and the SHA-256 of the fetched
page, never descriptions, comments or images. Field names follow the Fandom infobox names so the
three sources compare field by field (`verify_spells.py`).

Usage:
    python tibiopedia_spells.py fetch --cache <dir>             # network; one page per second
    python tibiopedia_spells.py facts --cache <dir> --out samples/tibiopedia-spell-facts-<date>.json
    python tibiopedia_spells.py self-test
"""
import argparse
import hashlib
import html
import http.cookiejar
import json
import re
import sys
import time
import urllib.error
import urllib.parse
import urllib.request
from datetime import datetime, timezone
from pathlib import Path

from wiki_spells import write_lines

BASE = 'https://tibiopedia.pl'
LIST_URL = BASE + '/spells/all'
USER_AGENT = 'OterynSpellAuthoring/1.0 (+https://github.com/Oteryn/Oteryn-Game)'
THROTTLE_SECONDS = 1.0
RETRIES = 4
LICENSE_NOTE = ('tibiopedia.pl, all rights reserved; only single facts with the page URL and the SHA-256 of '
                'the fetched page are recorded (owner decision S12), never descriptions, comments or images.')
GROUP = {'atak': 'Attack', 'leczenie': 'Healing', 'wsparcie': 'Support', 'specjalne': 'Special',
         'skupienie': 'Focus', 'postawa': 'Stance', 'przywołanie': 'Summon', 'drużyna': 'Party'}
DAMAGE = {'fizyczne': 'Physical', 'ogień': 'Fire', 'lód': 'Ice', 'energia': 'Energy', 'ziemia': 'Earth',
          'śmierć': 'Death', 'święte': 'Holy', 'leczenie': 'Healing', 'wyssanie życia': 'Life Drain',
          'wyssanie many': 'Mana Drain', 'utonięcie': 'Drowning', 'agonia': 'Agony'}
YES_NO = {'tak': 'yes', 'nie': 'no'}
ALL_VOCATIONS = 'Druid, Sorcerer, Knight, Paladin, Monk'


SETUP_URL = BASE + '/setup/choose'
# The site first serves a language/layout chooser (/setup) and keeps the answer in its session cookie.
OPENER = urllib.request.build_opener(urllib.request.HTTPCookieProcessor(http.cookiejar.CookieJar()))


def get(url, data=None):
    for attempt in range(RETRIES):
        try:
            request = urllib.request.Request(url, data=data, headers={'User-Agent': USER_AGENT})
            with OPENER.open(request, timeout=60) as response:
                return response.read()
        except (urllib.error.URLError, TimeoutError) as error:
            if attempt + 1 == RETRIES:
                raise
            print(f'retry {url}: {error}', file=sys.stderr)
            time.sleep(2 ** (attempt + 1))
    raise RuntimeError(url)


def spell_links(list_html):
    """Detail page links of the /spells/all list, in page order, without duplicates."""
    seen, links = set(), []
    for href in re.findall(r'<caption>.*?<a href="(https://tibiopedia\.pl/spells/[^"#?]+)"', list_html, re.S):
        if href not in seen:
            seen.add(href)
            links.append(href)
    return links


def fetch(cache):
    cache.mkdir(parents=True, exist_ok=True)
    fetched = datetime.now(timezone.utc).strftime('%Y-%m-%dT%H:%M:%SZ')
    get(SETUP_URL, urllib.parse.urlencode({'tp_lang': 'pl', 'tp_layout': 'library'}).encode())
    listing = get(LIST_URL).decode('utf-8', errors='replace')
    links = spell_links(listing)
    if not links:
        raise SystemExit('tibiopedia: the spell list has no spell links (setup page or layout change)')
    pages = []
    for url in links:
        time.sleep(THROTTLE_SECONDS)
        body = get(url)
        pages.append({'url': url, 'sha256': hashlib.sha256(body).hexdigest(),
                      'html': body.decode('utf-8', errors='replace')})
        print(f'{len(pages)}/{len(links)} {url}', file=sys.stderr)
    snapshot = {'fetched': fetched, 'list_url': LIST_URL, 'pages': pages}
    (cache / 'tibiopedia-spells.json').write_text(json.dumps(snapshot, ensure_ascii=False), encoding='utf-8')


def cell_text(cell):
    """Cell text; an icon cell reads as its alt text (group icons carry the group name)."""
    alts = re.findall(r'<img[^>]*\balt="([^"]*)"', cell)
    text = html.unescape(re.sub(r'<[^>]+>', ' ', re.sub(r'<br\s*/?>', ' ', cell)))
    text = re.sub(r'\s+', ' ', text).strip()
    return text, [html.unescape(a).strip() for a in alts]


def tables(page_html):
    """(caption, rows) per table of the spell detail block; each row is a list of (text, alts)."""
    start = page_html.find('spellDetailsDiv')
    end = page_html.find('id="search"', start)
    block = re.sub(r'<script.*?</script>', '', page_html[start:end], flags=re.S)
    out = []
    for table in re.findall(r'<table\b.*?</table>', block, re.S):
        caption = re.search(r'<caption[^>]*>(.*?)</caption>', table, re.S)
        caption_text = cell_text(caption.group(1))[0] if caption else ''
        rows = [[cell_text(c) for c in re.findall(r'<t[hd]\b[^>]*>(.*?)</t[hd]>', row, re.S)]
                for row in re.findall(r'<tr\b.*?</tr>', table, re.S)]
        out.append((caption_text, [r for r in rows if r]))
    return out


def seconds(value):
    """'2s', '1min', '1min 30s', '2h' -> whole seconds as the wiki writes them ('2'); None if absent."""
    text = value.strip().lower()
    total, matched = 0, False
    for amount, unit in re.findall(r'(\d+(?:[.,]\d+)?)\s*(h|min|s)\b', text):
        total += float(amount.replace(',', '.')) * {'h': 3600, 'min': 60, 's': 1}[unit]
        matched = True
    if not matched:
        return None
    return str(int(total)) if total == int(total) else str(total)


def cooldowns(rows):
    """Own cooldown, then each group cooldown with its group, from a Cooldown table."""
    own, groups = None, []
    for row in rows[1:]:
        if len(row) < 2:
            continue
        label, (value, alts) = row[0][0].lower(), row[1]
        if label in ('czaru', 'runy'):
            own = seconds(value)
        elif label.startswith('grup'):
            groups.append((seconds(value), GROUP.get(alts[0].lower(), alts[0]) if alts else None))
    return own, groups


def header_row(rows, *names):
    """Values of the first data row under a header row starting with `names`."""
    for i, row in enumerate(rows[:-1]):
        heads = [cell[0] for cell in row]
        if heads[:len(names)] == list(names):
            return {h: rows[i + 1][j] for j, h in enumerate(heads) if j < len(rows[i + 1])}
    return None


def page_facts(url, page_html):
    """Canonical (Fandom-named) facts of one detail page: a spell row, plus a rune row for a rune page."""
    spell, rune = {}, {}
    cooldown_tables = []
    for caption, rows in tables(page_html):
        head = header_row(rows, 'Grupa zaklęcia')
        if head and caption != 'Informacje o runie':
            alts = head['Grupa zaklęcia'][1]
            if alts:
                spell['subclass'] = GROUP.get(alts[0].lower(), alts[0])
            if 'Premium' in head:
                spell['premium'] = YES_NO.get(head['Premium'][0].lower(), head['Premium'][0])
            if 'Rodzaj obrażeń' in head and head['Rodzaj obrażeń'][0] not in ('', '-'):
                spell['damagetype'] = DAMAGE.get(head['Rodzaj obrażeń'][0].lower(), head['Rodzaj obrażeń'][0])
        head = header_row(rows, 'Lvl', 'Profesja', 'Formuła')
        if head:
            spell['levelrequired'] = head['Lvl'][0]
            spell['voc'] = head['Profesja'][0]
            spell['words'] = head['Formuła'][0]
            if 'Sp' in head:
                spell['soul'] = head['Sp'][0]
            if 'Mana' in head:
                spell['mana'] = head['Mana'][0]
        head = header_row(rows, 'Bazowa Moc')
        if head:
            if head['Bazowa Moc'][0] not in ('', '-'):
                spell['basepower'] = head['Bazowa Moc'][0]
            if head.get('Rodzaj obrażeń', ('',))[0] not in ('', '-'):
                spell['damagetype'] = DAMAGE.get(head['Rodzaj obrażeń'][0].lower(), head['Rodzaj obrażeń'][0])
            if head.get('Zasięg', ('',))[0] not in ('', '-'):
                spell['spellrange'] = head['Zasięg'][0]
        if rows and rows[0] and rows[0][0][0] == 'Cooldown':
            cooldown_tables.append(cooldowns(rows))
        if caption == 'Informacje o runie':
            head = header_row(rows, 'Wygląd') or {}
            if head.get('Rodzaj obrażeń', ('',))[0] not in ('', '-'):
                rune['damagetype'] = DAMAGE.get(head['Rodzaj obrażeń'][0].lower(), head['Rodzaj obrażeń'][0])
            charges = next((v for k, v in head.items() if k.startswith('Liczba')), None)
            if charges and charges[0] not in ('', '-'):
                rune['charges'] = charges[0]
        head = header_row(rows, 'Lvl', 'Mlvl')
        if head:
            rune['levelrequired'] = head['Lvl'][0]
            rune['mlrequired'] = head['Mlvl'][0]
            voc = head.get('Profesja', ('',))[0]
            if voc:
                rune['vocrequired'] = ALL_VOCATIONS if voc.lower() == 'każda' else voc
        text = re.sub(r'\s+', ' ', html.unescape(re.sub(r'<[^>]+>', ' ', page_html)))
        added = re.search(r'Dodane w wersji\s+(\d+(?:\.\d+)+)', text)
        if added:
            spell['implemented'] = added.group(1)
    if cooldown_tables:
        own, groups = cooldown_tables[0]
        if own is not None:
            spell['cooldown'] = own
        for index, (value, group) in enumerate(groups[:2]):
            suffix = '' if index == 0 else '2'
            if value is not None:
                spell['cooldowngroup' + suffix] = value
            if index == 1 and group:
                spell['secondarygroup'] = group
    name_match = re.search(r'<caption>\s*<img[^>]*alt="([^"]*)"', page_html[page_html.find('spellDetailsDiv'):])
    name = html.unescape(name_match.group(1)).strip() if name_match else url.rsplit('/', 1)[-1].replace('_', ' ')
    spell['name'] = name
    rows = [{'template': 'Infobox Spell', 'title': name, 'url': url, 'fields': spell}]
    if rune:
        if len(cooldown_tables) > 1 and cooldown_tables[1][0] is not None:
            rune['cooldown'] = cooldown_tables[1][0]
        rune['name'] = name
        spell['type'] = 'Rune'
        rows.append({'template': 'Infobox Object', 'title': name, 'url': url, 'fields': rune})
    return rows


def facts(snapshot):
    pages = []
    for page in snapshot['pages']:
        for row in page_facts(page['url'], page['html']):
            pages.append({**row, 'content_sha256': page['sha256']})
    return {'schema': 'OTERYN_SPELL_WIKI_FACTS/v1', 'wiki': 'tibiopedia', 'api': snapshot['list_url'],
            'license': LICENSE_NOTE, 'target_cut': snapshot['fetched'][:10],
            'cut_rule': 'pages as served at the fetch time ' + snapshot['fetched'], 'pages': pages}


SAMPLE = '''<div class="spellDetailsDiv"><table class="spell"><caption><img src="x" alt="Sudden Death Rune" />
Sudden Death Rune</caption><tr><th>Grupa zaklęcia</th><th>Premium</th></tr><tr><td><img alt="wsparcie" /></td>
<td>nie</td></tr></table><table class="spell"><tr><th>Lvl</th><th>Profesja</th><th>Formuła</th><th>Sp</th>
<th>Mana</th></tr><tr><td>45</td><td>Sorcerer</td><td><i>adori gran mort</i></td><td>5</td><td>985</td></tr></table>
<table class="spell"><tr><th>Bazowa Moc</th><th>Skalowanie</th><th>Rodzaj obrażeń</th><th>Zasięg</th></tr><tr>
<td>150</td><td>Magic Level</td><td>śmierć</td><td>7</td></tr></table><table class="spell"><tr>
<th colspan="2">Cooldown</th></tr><tr><td> Czaru </td><td> 2s </td></tr><tr><td> Grupy </td><td> 2s
(<img alt="wsparcie" />) </td></tr></table><table class="spell"><tr><th>Opis</th></tr><tr><td>prose</td></tr>
</table><table class="spell"><caption>Informacje o runie</caption><tr><th>Wygląd</th><th>Grupa zaklęcia</th>
<th>Rodzaj obrażeń</th><th>Liczba<br />ładunków</th></tr><tr><td><img alt="runa" /></td><td><img alt="atak" />
</td><td>śmierć</td><td>3</td></tr></table><table class="spell"><tr><th colspan="4">Wymagania</th></tr><tr>
<th>Lvl</th><th>Mlvl</th><th>Mana</th><th>Profesja</th></tr><tr><td>45</td><td>15</td><td>0</td><td>Każda</td>
</tr></table><table class="spell"><tr><th colspan="2">Cooldown</th></tr><tr><td> Runy </td><td> 2s </td></tr>
<tr><td> Grupy </td><td> 2s (<img alt="atak" />) </td></tr></table></div><div id="search"></div>'''


def self_test():
    spell, rune = page_facts('https://tibiopedia.pl/spells/Sudden_Death_Rune', SAMPLE)
    assert spell['fields'] == {'subclass': 'Support', 'premium': 'no', 'levelrequired': '45', 'voc': 'Sorcerer',
                               'words': 'adori gran mort', 'soul': '5', 'mana': '985', 'basepower': '150',
                               'damagetype': 'Death', 'spellrange': '7', 'cooldown': '2', 'cooldowngroup': '2',
                               'name': 'Sudden Death Rune', 'type': 'Rune'}, spell['fields']
    assert rune['fields'] == {'damagetype': 'Death', 'charges': '3', 'levelrequired': '45', 'mlrequired': '15',
                              'vocrequired': ALL_VOCATIONS, 'cooldown': '2', 'name': 'Sudden Death Rune'}, rune
    assert 'prose' not in json.dumps([spell, rune])
    assert seconds('2s') == '2' and seconds('1min 30s') == '90' and seconds('2h') == '7200'
    assert seconds('-') is None and seconds('0.5s') == '0.5'
    listing = ('<caption><img alt="A" /><a href="https://tibiopedia.pl/spells/Ice_Strike">Ice Strike</a></caption>'
               '<caption><a href="https://tibiopedia.pl/spells/Ice_Strike">x</a></caption>')
    assert spell_links(listing) == ['https://tibiopedia.pl/spells/Ice_Strike']
    print('tibiopedia_spells self-test: ok')
    return 0


def main(argv=None):
    parser = argparse.ArgumentParser(description=__doc__.splitlines()[0])
    parser.add_argument('command', choices=('fetch', 'facts', 'self-test'))
    parser.add_argument('--cache', type=Path)
    parser.add_argument('--out', type=Path)
    args = parser.parse_args(argv)
    if args.command == 'self-test':
        return self_test()
    if args.command == 'fetch':
        fetch(args.cache)
        return 0
    snapshot = json.loads((args.cache / 'tibiopedia-spells.json').read_text(encoding='utf-8'))
    write_lines(args.out, facts(snapshot), 'pages')
    return 0


if __name__ == '__main__':
    sys.exit(main())
