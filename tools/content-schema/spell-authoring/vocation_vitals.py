"""Candidate maximum hitpoints, mana, capacity and soul per vocation and level (decision SPELL-D5).

Evidence tooling only. SPELL-D5 (#162 comment 5867161696) applies the spell source rule to vitals: the wiki
decides what it states (S3, S11, S13), Canary and Crystal fill only what it does not (S4), every value keeps
its provenance, and a remaining conflict goes to the owner. TibiaWiki (Fandom) `Formulae` states per
vocation the gain per level and the total from level 8 for hitpoints, mana and capacity; below level 8
every character has the Rookie values. `Soul Point` states the soul maximum per account type and the soul
regeneration rate of regular and promoted characters. Canary and Crystal add hitpoint and mana
regeneration. Owner decisions of 2026-09-28: the soul maximum follows the account type as the wiki states
(D5a; the account type is Platform-owned), and where Canary and Crystal differ on regeneration the Canary
15.30 branch decides (D5b). Only these facts are recorded, with each Fandom revision id and content SHA-256.

Usage:
    python vocation_vitals.py fetch --cache <dir>        # network: current Fandom Formulae and Soul Point
    python vocation_vitals.py build --cache <dir> --canary <canary@99902524> --crystal <crystalserver@ff7ede5> \
        --out samples/vocation-vitals-candidate.json
    python vocation_vitals.py self-test
"""
import argparse
import hashlib
import json
import re
import subprocess
import sys
import urllib.parse
import urllib.request
from pathlib import Path

from wiki_spells import USER_AGENT, write_lines

API = 'https://tibia.fandom.com/api.php'
PAGES = {'formulae': 'Formulae', 'soul': 'Soul Point'}
ROW_VOCATIONS = {'Knights': ['knight', 'elite_knight'], 'Monks': ['monk', 'exalted_monk'],
                 'Paladins': ['paladin', 'royal_paladin'],
                 'Sorcerers and Druids': ['sorcerer', 'master_sorcerer', 'druid', 'elder_druid'], 'Rookies': ['none']}
SOURCE_NAMES = {'None': 'none', 'Sorcerer': 'sorcerer', 'Druid': 'druid', 'Paladin': 'paladin', 'Knight': 'knight',
                'Master Sorcerer': 'master_sorcerer', 'Elder Druid': 'elder_druid', 'Royal Paladin': 'royal_paladin',
                'Elite Knight': 'elite_knight', 'Monk': 'monk', 'Exalted Monk': 'exalted_monk'}
STATS = ('hitpoints', 'mana', 'capacity')
SOURCE_GAIN = {'hitpoints': 'gainhp', 'mana': 'gainmana', 'capacity': 'gaincap'}
REGEN_FIELDS = ('gainhpticks', 'gainhpamount', 'gainmanaticks', 'gainmanaamount')
PROMOTED = {'elite_knight', 'royal_paladin', 'master_sorcerer', 'elder_druid', 'exalted_monk'}
ROOKIE_UNTIL_LEVEL = 8
# The same pins as the spell census (S14); `build` refuses a checkout at another commit.
SOURCE_REVISIONS = {'canary': ('opentibiabr/canary', '99902524e052f37574194466c2949c576e4ab269'),
                    'crystal': ('zimbadev/crystalserver', 'ff7ede593c69d4c658b382c97443e8155926924a')}
SOUL_MAX = re.compile(r'Free Account\]\] characters have a maximum of (\d+) soul points and \[\[Premium Account\]\] '
                      r'characters have a maximum of (\d+)')
SOUL_REGEN = re.compile(r'Regular characters regenerate Soul Points at a rate of 1 Soul Point every (\d+) minutes?, '
                        r'while \[\[Promotion\|Promoted\]\] characters\' Soul Points regenerate at a rate of 1 Soul '
                        r'Point every (\d+) seconds')


def fetch(cache):
    cache.mkdir(parents=True, exist_ok=True)
    for key, title in PAGES.items():
        query = urllib.parse.urlencode({'action': 'query', 'redirects': 1, 'prop': 'revisions', 'rvslots': 'main',
                                        'rvprop': 'content|ids|timestamp', 'format': 'json', 'formatversion': 2,
                                        'titles': title})
        request = urllib.request.Request(f'{API}?{query}', headers={'User-Agent': USER_AGENT})
        with urllib.request.urlopen(request, timeout=60) as response:
            page = json.load(response)['query']['pages'][0]
        revision = page['revisions'][0]
        (cache / f'fandom-{key}.json').write_text(json.dumps({
            'title': page['title'], 'page_id': page['pageid'], 'revision_id': revision['revid'],
            'timestamp': revision['timestamp'], 'content': revision['slots']['main']['content']}), encoding='utf-8')


def wiki_source(page):
    return {'kind': 'mediawiki', 'api': API, 'title': page['title'], 'page_id': page['page_id'],
            'revision_id': page['revision_id'],
            'content_sha256': hashlib.sha256(page['content'].encode('utf-8')).hexdigest()}


def soul_facts(content):
    maximum, regen = SOUL_MAX.search(content), SOUL_REGEN.search(content)
    if not maximum or not regen:
        raise ValueError('Soul Point: the soul maximum or regeneration sentence changed')
    return {'max': {'free_account': int(maximum.group(1)), 'premium_account': int(maximum.group(2))},
            'regen_ms': {'regular': int(regen.group(1)) * 60_000, 'promoted': int(regen.group(2)) * 1000}}


def linear(expression):
    """'5\\left(3lvl + 13\\right)' -> (15, 65): total = a * level + b."""
    text = expression.replace('\\left', '').replace('\\right', '').replace(' ', '')
    match = re.fullmatch(r'(\d+)\((\d*)lvl([+-]\d+)\)', text)
    if not match:
        raise ValueError(f'unexpected formula {expression!r}')
    factor, per, offset = int(match.group(1)), int(match.group(2) or 1), int(match.group(3))
    return factor * per, factor * offset


def fandom_table(content):
    """{row name: {stat: (per_level, total_a, total_b)}} from the Hitpoints, Mana, and Capacity table."""
    section = re.search(r'== Hitpoints, Mana, and Capacity ==\n(.*?)\n\|\}', content, re.S)
    if not section:
        raise ValueError('Formulae: the Hitpoints, Mana, and Capacity table is missing')
    rows = {}
    for block in section.group(1).split('\n|-')[1:]:
        lines = [line.strip() for line in block.strip().splitlines() if line.strip()]
        if not lines or not lines[0].startswith('!') or len(lines) != 7:
            continue
        name = re.sub(r'\[\[(?:[^|\]]*\|)?([^\]]*)\]\]', r'\1', lines[0].lstrip('! ').strip())
        cells = [re.sub(r'</?math>', '', line.lstrip('| ').strip()) for line in lines[1:]]
        rows[name] = {}
        for index, stat in enumerate(STATS):
            per_level = int(cells[2 * index])
            a, b = linear(cells[2 * index + 1])
            if a != per_level:
                raise ValueError(f'{name} {stat}: the total formula does not grow by its per-level gain')
            rows[name][stat] = (per_level, a, b)
    return rows


def source_vocations(root, name):
    head = subprocess.run(['git', '-C', str(root), 'rev-parse', 'HEAD'], capture_output=True, text=True,
                          check=True).stdout.strip()
    if head != SOURCE_REVISIONS[name][1]:
        raise SystemExit(f'{name}: {root} is at {head}, expected {SOURCE_REVISIONS[name][1]}')
    text = (root / 'data' / 'XML' / 'vocations.xml').read_text(encoding='utf-8', errors='replace')
    out = {}
    for match in re.finditer(r'<vocation ([^>]*)>', text):
        attrs = dict(re.findall(r'(\w+)="([^"]*)"', match.group(1)))
        key = SOURCE_NAMES.get(attrs.get('name'))
        if key:
            out[key] = attrs
    return out


def build(formulae, soul_page, sources):
    table = fandom_table(formulae['content'])
    soul = soul_facts(soul_page['content'])
    rookie = table['Rookies']
    vocations, conflicts = {}, []
    by_vocation = {v: row for row, keys in ROW_VOCATIONS.items() for v in keys}
    for vocation, row in sorted(by_vocation.items()):
        stats = table[row]
        entry = {'rookie_until_level': ROOKIE_UNTIL_LEVEL}
        for stat in STATS:
            per_level, a, b = stats[stat]
            entry[stat] = {'per_level': per_level, 'total': {'level_factor': a, 'offset': b},
                           'rookie_total': {'level_factor': rookie[stat][1], 'offset': rookie[stat][2]},
                           'provenance': 'wiki:Formulae'}
            for name, attrs in sources.items():
                gain = attrs.get(vocation, {}).get(SOURCE_GAIN[stat])
                if gain is not None and int(gain) != per_level:
                    conflicts.append({'vocation': vocation, 'field': stat + '.per_level', 'wiki': per_level,
                                      name: int(gain), 'resolution': 'S3: the wiki decides.'})
        regen_ms = soul['regen_ms']['promoted' if vocation in PROMOTED else 'regular']
        # Owner D5a (2026-09-28): the wiki decides; the maximum follows the account type, not promotion.
        entry['soul'] = {'regen_ms': regen_ms, 'max': soul['max'], 'provenance': 'wiki:Soul Point (owner D5a)'}
        for name, attrs in sources.items():
            ticks = attrs.get(vocation, {}).get('gainsoulticks')
            if ticks is not None and int(ticks) != regen_ms:
                conflicts.append({'vocation': vocation, 'field': 'soul.regen_ms', 'wiki': regen_ms, name: int(ticks),
                                  'resolution': 'S3: the wiki decides.'})
        regen = {}
        for field in REGEN_FIELDS:
            present = {name: int(attrs[vocation][field]) for name, attrs in sources.items()
                       if field in attrs.get(vocation, {})}
            if len(set(present.values())) == 1:
                regen[field] = next(iter(present.values()))
            elif present:
                # Owner D5b (2026-09-28): where the sources differ, the Canary 15.30 branch decides (S14 tie rule).
                regen[field] = present['canary']
                conflicts.append({'vocation': vocation, 'field': field, **present,
                                  'resolution': 'Owner D5b: Canary 15.30 decides.'})
        entry['regeneration'] = {**regen, 'provenance': 'sources: equal values, else Canary 15.30 (owner D5b)'}
        vocations[vocation] = entry
    conflicts.append({'vocation': '*', 'field': 'soul.max', 'wiki': soul['max'],
                      'sources': {v: {name: int(attrs[v]['soulmax']) for name, attrs in sources.items()
                                      if 'soulmax' in attrs.get(v, {})} for v in sorted(vocations)},
                      'resolution': 'Owner D5a: the wiki decides; the maximum follows the account type, which '
                                    'Platform owns.'})
    return {'schema': 'OTERYN_VOCATION_VITALS_CANDIDATE/v1',
            'decision': 'SPELL-D5 (#162 comment 5867161696); owner D5a and D5b of 2026-09-28',
            'rule': 'total(level) = rookie_total below rookie_until_level, else level_factor * level + offset',
            'wiki_sources': [wiki_source(formulae), wiki_source(soul_page)],
            'source_revisions': {name: f'{repo}@{rev} data/XML/vocations.xml'
                                 for name, (repo, rev) in SOURCE_REVISIONS.items()}, 'vocations': vocations, 'conflicts': conflicts}


def total(stat_entry, level, rookie_until):
    formula = stat_entry['total'] if level >= rookie_until else stat_entry['rookie_total']
    return formula['level_factor'] * level + formula['offset']


SAMPLE = '''== Hitpoints, Mana, and Capacity ==

{|class="wikitable"
! rowspan="2" | [[Vocation]]
|-
! Per Level
|-
! [[Knights]]
| <math>15</math>
| <math>5\\left(3lvl + 13\\right)</math>
| <math>5</math>
| <math>5\\left(lvl + 10\\right)</math>
| <math>25</math>
| <math>5\\left(5lvl + 54\\right)</math>
|-
! [[Rookie]]s
| <math>5</math>
| <math>5\\left(lvl + 29\\right)</math>
| <math>5</math>
| <math>5\\left(lvl + 10\\right)</math>
| <math>10</math>
| <math>10\\left(lvl + 39\\right)</math>
|}
'''


def self_test():
    table = fandom_table(SAMPLE)
    assert table['Knights']['hitpoints'] == (15, 15, 65), table
    assert table['Rookies']['capacity'] == (10, 10, 390), table
    assert linear('5\\left(6lvl - 30\\right)') == (30, -150)
    entry = {'total': {'level_factor': 15, 'offset': 65}, 'rookie_total': {'level_factor': 5, 'offset': 145}}
    soul = soul_facts("[[Free Account]] characters have a maximum of 100 soul points and [[Premium Account]] "
                      "characters have a maximum of 200. Regular characters regenerate Soul Points at a rate of 1 Soul "
                      "Point every 2 minutes, while [[Promotion|Promoted]] characters' Soul Points regenerate at a "
                      "rate of 1 Soul Point every 16 seconds")
    assert soul == {'max': {'free_account': 100, 'premium_account': 200},
                    'regen_ms': {'regular': 120000, 'promoted': 16000}}, soul
    assert total(entry, 8, 8) == 185 and total(entry, 7, 8) == 180 and total(entry, 100, 8) == 1565
    print('vocation_vitals self-test: ok')
    return 0


def main(argv=None):
    parser = argparse.ArgumentParser(description=__doc__.splitlines()[0])
    parser.add_argument('command', choices=('fetch', 'build', 'self-test'))
    parser.add_argument('--cache', type=Path)
    parser.add_argument('--canary', type=Path)
    parser.add_argument('--crystal', type=Path)
    parser.add_argument('--out', type=Path)
    args = parser.parse_args(argv)
    if args.command == 'self-test':
        return self_test()
    if args.command == 'fetch':
        fetch(args.cache)
        return 0
    formulae, soul = (json.loads((args.cache / f'fandom-{key}.json').read_text(encoding='utf-8')) for key in PAGES)
    sources = {'canary': source_vocations(args.canary, 'canary'), 'crystal': source_vocations(args.crystal, 'crystal')}
    document = build(formulae, soul, sources)
    write_lines(args.out, document, None)
    print(json.dumps({'vocations': len(document['vocations']), 'conflicts': len(document['conflicts'])}))
    return 0


if __name__ == '__main__':
    sys.exit(main())
