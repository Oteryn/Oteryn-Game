"""Compare the Canary/Crystal player spell census with TibiaWiki (Fandom) at the reference cut.

Evidence tooling only. TibiaWiki is a player-observed reference source (CC BY-SA); the committed
facts file keeps only short allowlisted infobox values plus page id, immutable revision id and the
SHA-256 of that revision's wikitext, never article prose. Nothing here becomes Game truth by
comparison alone; adoption is an owner decision (monster D15/D25 precedent).

Owner rule (2026-09-27): the wiki is read as of the fetch day, not at a historical cut, because the
wikis carry the most current Reference data. `fetch --cut YYYY-MM-DD` (default: today, UTC) pins the
last revision at or before the end of that day; the facts file records every revision id.
Pages: every article embedding `Template:Infobox Spell` (instant and conjuring spells) and every
member of `Category:Runes` (rune items, `Infobox Object`, matched to rune spells by item id).

Usage:
    python wiki_spells.py fetch --cache <dir>            # network; writes <dir>/fandom-spells-cut.json
    python wiki_spells.py facts --cache <dir> --out samples/wiki-spell-facts-fandom-<cut>.json
    python wiki_spells.py compare --facts samples/wiki-spell-facts-fandom-<cut>.json \
        --census samples/spell-census-canary-47dfd51f-crystal-ff7ede5.json \
        --out samples/wiki-spell-compare-fandom-<cut>.json
    python wiki_spells.py self-test
"""
import argparse
import hashlib
import json
import re
import sys
import time
import urllib.error
import urllib.parse
import urllib.request
from collections import Counter
from datetime import datetime, timedelta, timezone
from pathlib import Path

API = 'https://tibia.fandom.com/api.php'
USER_AGENT = 'OterynSpellAuthoring/1.0 (+https://github.com/Oteryn/Oteryn-Game)'
LICENSE_NOTE = ('TibiaWiki (Fandom), CC BY-SA; only short allowlisted infobox facts with page and revision ids '
                'are recorded, never article prose.')
FORMULAE_PAGE = 'Formulae'
SPELL_TEMPLATE = 'Template:Infobox Spell'
RUNE_CATEGORY = 'Category:Runes'
THROTTLE_SECONDS = 0.5
RETRIES = 4
SPELL_FIELDS = ('name', 'spellid', 'type', 'subclass', 'secondarygroup', 'runegroup', 'damagetype', 'words',
                'premium', 'mana', 'soul', 'amount', 'levelrequired', 'cooldown', 'cooldown2', 'cooldown3',
                'cooldowngroup', 'cooldowngroup2', 'voc', 'basepower', 'promotion', 'partyspell',
                'passivespell', 'wheelspell', 'implemented', 'status', 'learnfrom', 'spellcost')
RUNE_FIELDS = ('name', 'actualname', 'itemid', 'objectclass', 'primarytype', 'words', 'levelrequired',
               'mlrequired', 'vocrequired', 'damagetype', 'basepower', 'implemented', 'status')
MAX_FIELD_LENGTH = 400
BASE_VOCATION = {'druid': 'druid', 'elder druid': 'druid', 'sorcerer': 'sorcerer', 'master sorcerer': 'sorcerer',
                 'knight': 'knight', 'elite knight': 'knight', 'paladin': 'paladin', 'royal paladin': 'paladin',
                 'monk': 'monk', 'exalted monk': 'monk'}
DAMAGE_TYPE = {'COMBAT_PHYSICALDAMAGE': 'physical', 'COMBAT_ENERGYDAMAGE': 'energy', 'COMBAT_EARTHDAMAGE': 'earth',
               'COMBAT_FIREDAMAGE': 'fire', 'COMBAT_LIFEDRAIN': 'life drain', 'COMBAT_MANADRAIN': 'mana drain',
               'COMBAT_HEALING': 'healing', 'COMBAT_DROWNDAMAGE': 'drowning', 'COMBAT_ICEDAMAGE': 'ice',
               'COMBAT_HOLYDAMAGE': 'holy', 'COMBAT_DEATHDAMAGE': 'death', 'COMBAT_AGONYDAMAGE': 'agony',
               'COMBAT_NEUTRALDAMAGE': 'neutral'}


# ------------------------------------------------------------------------------------------------
# Fandom access
# ------------------------------------------------------------------------------------------------

def api(params):
    query = urllib.parse.urlencode({**params, 'format': 'json', 'formatversion': '2'})
    delay, last = 1.0, None
    for attempt in range(RETRIES):
        try:
            request = urllib.request.Request(API + '?' + query, headers={'User-Agent': USER_AGENT})
            with urllib.request.urlopen(request, timeout=30) as response:
                return json.load(response)
        except (urllib.error.URLError, TimeoutError, OSError) as exc:
            last = exc
            if attempt + 1 < RETRIES:
                time.sleep(delay)
                delay *= 2
    raise RuntimeError(f'Fandom API request failed after {RETRIES} attempts: {last}') from last


def listing(params, key):
    titles = []
    while True:
        data = api(params)
        titles.extend(p['title'] for p in data.get('query', {}).get(key, []))
        time.sleep(THROTTLE_SECONDS)
        if not data.get('continue'):
            return titles
        params = {**params, **data['continue']}


def cut_timestamp(cut):
    day = datetime.strptime(cut, '%Y-%m-%d').replace(tzinfo=timezone.utc)
    return (day + timedelta(days=1)).strftime('%Y-%m-%dT%H:%M:%SZ')


def cut_revision(title, cut):
    data = api({'action': 'query', 'prop': 'revisions', 'titles': title, 'rvlimit': 1, 'rvstart': cut_timestamp(cut),
                'rvdir': 'older', 'rvprop': 'ids|timestamp|content', 'rvslots': 'main'})
    time.sleep(THROTTLE_SECONDS)
    page = data['query']['pages'][0]
    revisions = page.get('revisions') or []
    if page.get('missing') or not revisions:
        return {'title': title, 'page_id': page.get('pageid'), 'status': 'no_revision_at_cut'}
    rev = revisions[0]
    content = rev['slots']['main']['content']
    return {'title': page['title'], 'page_id': page['pageid'], 'revision_id': rev['revid'],
            'timestamp': rev['timestamp'], 'content_sha256': hashlib.sha256(content.encode('utf-8')).hexdigest(),
            'content': content}


def fetch(cache, cut):
    spells = listing({'action': 'query', 'list': 'embeddedin', 'eititle': SPELL_TEMPLATE, 'einamespace': 0,
                      'eilimit': 500}, 'embeddedin')
    runes = listing({'action': 'query', 'list': 'categorymembers', 'cmtitle': RUNE_CATEGORY, 'cmnamespace': 0,
                     'cmlimit': 500}, 'categorymembers')
    pages = {}
    for title in sorted(set(spells) | set(runes) | {FORMULAE_PAGE}):
        pages[title] = cut_revision(title, cut)
    snapshot = {'api': API, 'target_cut': cut, 'cut_rule': f'last revision at or before {cut_timestamp(cut)}',
                'spell_titles': sorted(spells), 'rune_titles': sorted(runes), 'pages': pages}
    cache.mkdir(parents=True, exist_ok=True)
    (cache / 'fandom-spells-cut.json').write_text(json.dumps(snapshot, ensure_ascii=False), encoding='utf-8')
    print(f'{len(spells)} spell pages, {len(runes)} rune category pages')


# ------------------------------------------------------------------------------------------------
# Wikitext
# ------------------------------------------------------------------------------------------------

def top_level_fields(text, start):
    """`| key = value` fields of the template starting at text[start:start+2] == '{{' (nesting-aware)."""
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
    match = re.search(r'\{\{\s*' + re.escape(template) + r'\s*[|}]', text, re.IGNORECASE)
    return top_level_fields(text, match.start()) if match else {}


def level_curve(content):
    """The two <math> formulas of the Formulae page 'Damage and Healing' section (S5), verbatim."""
    start = content.find('The step size')
    return re.findall(r'<math>(.*?)</math>', content[start:start + 800]) if start >= 0 else []


def facts(snapshot):
    """Allowlisted infobox facts per page (no prose)."""
    out = []
    for title, page in sorted(snapshot['pages'].items()):
        row = {k: page[k] for k in ('title', 'page_id', 'revision_id', 'timestamp', 'content_sha256') if k in page}
        if page.get('status'):
            row['status'] = page['status']
            out.append(row)
            continue
        if title == FORMULAE_PAGE:
            row['template'] = None
            row['level_curve'] = level_curve(page['content'])
            out.append(row)
            continue
        spell = infobox(page['content'], 'Infobox Spell')
        rune = infobox(page['content'], 'Infobox Object')
        if spell:
            row['template'] = 'Infobox Spell'
            row['fields'] = {k: spell[k][:MAX_FIELD_LENGTH] for k in SPELL_FIELDS if spell.get(k)}
        elif rune and title in snapshot['rune_titles']:
            row['template'] = 'Infobox Object'
            row['fields'] = {k: rune[k][:MAX_FIELD_LENGTH] for k in RUNE_FIELDS if rune.get(k)}
        else:
            row['template'] = None
        out.append(row)
    return {'schema': 'OTERYN_SPELL_WIKI_FACTS/v1', 'api': snapshot['api'], 'license': LICENSE_NOTE,
            'target_cut': snapshot['target_cut'], 'cut_rule': snapshot['cut_rule'], 'pages': out}


# ------------------------------------------------------------------------------------------------
# Normalisation
# ------------------------------------------------------------------------------------------------

def plain(value):
    value = re.sub(r'\[\[(?:[^|\]]*\|)?([^\]]*)\]\]', r'\1', value or '')
    value = re.sub(r'<[^>]+>', '', value)
    return re.sub(r'\s+', ' ', value).strip()


def wiki_number(value):
    text = plain(value).replace(',', '')
    return int(text) if re.fullmatch(r'\d+', text) else None


def wiki_seconds_ms(value):
    text = plain(value)
    if re.fullmatch(r'\d+(?:\.\d+)?', text):
        return int(round(float(text) * 1000))
    return None


def wiki_vocations(value):
    text = plain(value).lower()
    found = set()
    for name in sorted(BASE_VOCATION, key=len, reverse=True):
        pattern = r'\b' + re.escape(name) + r's?\b'
        if re.search(pattern, text):
            found.add(BASE_VOCATION[name])
            text = re.sub(pattern, ' ', text)
    return sorted(found)


def words_key(value):
    value = re.sub(r"''.*?''", ' ', value or '')
    return re.sub(r'\s+', ' ', plain(value).lower().replace('"', ' ').replace("'", '')).strip()


def words_match(registrar, wiki_words):
    """Wiki words may append the parameter (`exura sio "name`); the spoken words are its prefix."""
    source = words_key(registrar.get('words', ''))
    if source == wiki_words:
        return True
    takes_parameter = registrar.get('hasParams') or registrar.get('hasPlayerNameParam')
    return bool(takes_parameter and wiki_words.startswith(source + ' '))


def source_vocations(registrar):
    names = [str(v).split(';')[0].strip().lower() for v in registrar.get('vocation', [])]
    return sorted({BASE_VOCATION.get(n, n) for n in names})


def as_list(value):
    if value is None:
        return []
    return value if isinstance(value, list) else [value]


def source_damage_type(record):
    types = {c['parameters'].get('COMBAT_PARAM_TYPE') for c in record['combats']} - {None}
    types = {DAMAGE_TYPE.get(t, str(t)) for t in types}
    return sorted(types)


def compare_field(stats, rows, field, source_value, wiki_value, parsed=True):
    counts = stats.setdefault(field, Counter())
    if not parsed:
        counts['wiki_unparsed'] += 1
    elif wiki_value is None:
        counts['wiki_absent'] += 1
    elif source_value is None:
        counts['source_absent'] += 1
        rows[field] = {'source': None, 'wiki': wiki_value}
    elif source_value == wiki_value:
        counts['match'] += 1
    else:
        counts['diff'] += 1
        rows[field] = {'source': source_value, 'wiki': wiki_value}


def compare_spell(record, wiki, stats):
    reg, fields, rows = record['registrar'], wiki['fields'], {}
    wiki_words = words_key(fields.get('words', '')) or None
    source_words = words_key(reg.get('words', '')) or None
    compare_field(stats, rows, 'words', wiki_words if wiki_words and words_match(reg, wiki_words) else source_words,
                  wiki_words)
    for field, source in (('level', reg.get('level')), ('mana', reg.get('mana')), ('soul', reg.get('soul'))):
        wiki_field = {'level': 'levelrequired'}.get(field, field)
        raw = fields.get(wiki_field)
        value = wiki_number(raw) if raw is not None else None
        compare_field(stats, rows, field, source if source is not None else (0 if field == 'soul' else None),
                      value if raw is not None else (0 if field == 'soul' else None), raw is None or value is not None)
    raw = fields.get('premium')
    compare_field(stats, rows, 'premium', bool(reg.get('isPremium', False)),
                  None if raw is None else plain(raw).lower() == 'yes', True)
    raw = fields.get('cooldown')
    value = wiki_seconds_ms(raw) if raw is not None else None
    compare_field(stats, rows, 'cooldown_ms', reg.get('cooldown'), value, raw is None or value is not None)
    group_cd = as_list(reg.get('groupCooldown'))
    for index, key in enumerate(('cooldowngroup', 'cooldowngroup2')):
        raw = fields.get(key)
        value = wiki_seconds_ms(raw) if raw is not None else None
        if value == 0:
            raw = value = None  # the wiki writes 0 for "no secondary group cooldown"
        compare_field(stats, rows, f'group_cooldown_{index + 1}_ms', group_cd[index] if index < len(group_cd) else None,
                      value, raw is None or value is not None)
    groups = [str(g).lower() for g in as_list(reg.get('group'))]
    raw = fields.get('subclass')
    compare_field(stats, rows, 'group', groups[0] if groups else None, plain(raw).lower() if raw else None)
    raw = fields.get('secondarygroup')
    compare_field(stats, rows, 'secondary_group', groups[1] if len(groups) > 1 else None,
                  plain(raw).lower().replace(' ', '').replace("'", '') if raw else None)
    raw = fields.get('voc')
    compare_field(stats, rows, 'vocations', source_vocations(reg) or None, wiki_vocations(raw) if raw else None)
    raw = fields.get('basepower')
    value = wiki_number(raw) if raw is not None else None
    compare_field(stats, rows, 'base_power', reg.get('basePower'), value, raw is None or value is not None)
    compare_damage_type(stats, rows, record, fields.get('damagetype'))
    if record['cast']['tier'] == 'conjure':
        raw = fields.get('amount')
        value = wiki_number(raw) if raw is not None else None
        compare_field(stats, rows, 'conjure_amount', (record['cast'].get('conjure') or {}).get('count'), value,
                      raw is None or value is not None)
    return rows


def compare_rune(record, wiki, stats):
    reg, fields, rows = record['registrar'], wiki['fields'], {}
    for field, key in (('level', 'levelrequired'), ('magicLevel', 'mlrequired'), ('basePower', 'basepower')):
        raw = fields.get(key)
        value = wiki_number(raw) if raw is not None else None
        compare_field(stats, rows, field, reg.get(field), value, raw is None or value is not None)
    compare_damage_type(stats, rows, record, fields.get('damagetype'))
    return rows


def compare_damage_type(stats, rows, record, raw):
    """A source spell with several Combats (stances, variants) matches when the wiki type is one of them."""
    types = source_damage_type(record)
    wiki = plain(raw).lower() if raw else None
    if len(types) > 1 and wiki in types:
        stats.setdefault('damage_type', Counter())['match_one_of_several'] += 1
        return
    compare_field(stats, rows, 'damage_type', types[0] if len(types) == 1 else (types or None), wiki)


def compare(facts_doc, census):
    spell_pages = [p for p in facts_doc['pages'] if p.get('template') == 'Infobox Spell']
    by_words, by_name = {}, {}
    for page in spell_pages:
        words = words_key(page['fields'].get('words', ''))
        if words:
            by_words.setdefault(words, []).append(page)
        by_name[plain(page['fields'].get('name', page['title'])).lower()] = page
    rune_pages = {}
    for page in facts_doc['pages']:
        if page.get('template') == 'Infobox Object':
            item = wiki_number(page['fields'].get('itemid', ''))
            if item is not None:
                rune_pages.setdefault(item, page)
    result = {}
    for source in ('canary', 'crystal'):
        stats = {}
        matched, unmatched, rows = set(), [], []
        for record in census[source]:
            reg = record['registrar']
            if str(reg.get('words', '')).startswith('#'):
                unmatched.append({'name': record['name'], 'spell_type': record['spell_type'],
                                  'reason': 'monster-only registration (unspeakable words)'})
                continue
            if record['spell_type'] == 'rune':
                page = rune_pages.get(reg.get('runeId'))
                if page is None:
                    unmatched.append({'name': record['name'], 'spell_type': 'rune', 'rune_item_id': reg.get('runeId')})
                    continue
                local = {}
                diff = compare_rune(record, page, local)
            else:
                candidates = by_words.get(words_key(reg.get('words', '')), [])
                page = candidates[0] if len(candidates) == 1 else by_name.get(str(record['name']).lower())
                if page is None:
                    unmatched.append({'name': record['name'], 'spell_type': 'instant', 'words': reg.get('words')})
                    continue
                local = {}
                diff = compare_spell(record, page, local)
            matched.add(page['title'])
            for field, counter in local.items():
                stats.setdefault(field, Counter()).update(counter)
            if diff:
                rows.append({'name': record['name'], 'spell_type': record['spell_type'], 'file': record['file'],
                             'wiki_title': page['title'], 'wiki_revision_id': page.get('revision_id'),
                             'differences': diff})
        wiki_only = [{'title': p['title'], **{k: plain(p['fields'][k]) for k in ('status', 'implemented', 'type')
                                              if k in p['fields']}}
                     for p in sorted(spell_pages, key=lambda p: p['title']) if p['title'] not in matched]
        result[source] = {'compared': len(census[source]) - len(unmatched), 'unmatched_source': unmatched,
                          'field_counts': {k: dict(v) for k, v in sorted(stats.items())},
                          'wiki_spell_pages_without_source': wiki_only, 'differences': rows}
    return result


def self_test():
    text = ('{{Infobox Spell|List={{{1|}}}\n| name = Light Healing\n| words = exura\n| premium = no\n| mana = 20\n'
            '| levelrequired = 8\n| cooldown = 1\n| cooldowngroup = 1\n| voc = [[Paladin]]s, [[Druid]]s, [[Sorcerer]]s, '
            '[[Monk]]s\n| subclass = Healing\n| damagetype = Healing\n| basepower = 40\n| effect = x [[HP|health]]\n}}')
    fields = infobox(text, 'Infobox Spell')
    assert fields['words'] == 'exura' and fields['levelrequired'] == '8', fields
    assert wiki_vocations(fields['voc']) == ['druid', 'monk', 'paladin', 'sorcerer'], wiki_vocations(fields['voc'])
    record = {'spell_type': 'instant', 'name': 'Light Healing', 'file': 'x', 'cast': {'tier': 'plain_combat'},
              'combats': [{'parameters': {'COMBAT_PARAM_TYPE': 'COMBAT_HEALING'}}],
              'registrar': {'words': 'exura', 'mana': 20, 'level': 8, 'cooldown': 1000, 'groupCooldown': 1000,
                            'group': 'healing', 'vocation': ['druid;true', 'elder druid;true', 'paladin;true',
                                                             'sorcerer;true', 'monk;true'], 'basePower': 40}}
    stats = {}
    rows = compare_spell(record, {'fields': {k: v for k, v in fields.items()}}, stats)
    assert rows == {}, rows
    record['registrar']['mana'] = 25
    rows = compare_spell(record, {'fields': fields}, {})
    assert rows == {'mana': {'source': 25, 'wiki': 20}}, rows
    assert wiki_seconds_ms('2') == 2000 and wiki_seconds_ms('?') is None
    assert wiki_vocations('[[Druid]]s and [[Sorcerer]]s') == ['druid', 'sorcerer']
    assert wiki_vocations('[[Elite Knight]]s') == ['knight']
    assert words_match({'words': 'exura sio', 'hasParams': True}, words_key('exura sio "\'\'name\'\'"'))
    assert words_match({'words': 'exura sio', 'hasParams': True}, 'exura sio name')
    assert not words_match({'words': 'exura sio'}, 'exura sio name')
    print('wiki_spells self-test: ok')
    return 0


def main(argv=None):
    parser = argparse.ArgumentParser(description=__doc__.splitlines()[0])
    parser.add_argument('command', choices=('fetch', 'facts', 'compare', 'self-test'))
    parser.add_argument('--cache', type=Path)
    parser.add_argument('--facts', type=Path)
    parser.add_argument('--census', type=Path)
    parser.add_argument('--out', type=Path)
    parser.add_argument('--cut', default=datetime.now(timezone.utc).strftime('%Y-%m-%d'),
                        help='fetch: read each page as of the end of this UTC day (default: today)')
    args = parser.parse_args(argv)
    if args.command == 'self-test':
        return self_test()
    if args.command == 'fetch':
        fetch(args.cache, args.cut)
        return 0
    if args.command == 'facts':
        snapshot = json.loads((args.cache / 'fandom-spells-cut.json').read_text(encoding='utf-8'))
        document = facts(snapshot)
        write_lines(args.out, document, 'pages')
        return 0
    facts_doc = json.loads(args.facts.read_text(encoding='utf-8'))
    census = json.loads(args.census.read_text(encoding='utf-8'))
    result = compare(facts_doc, census)
    document = {'schema': 'OTERYN_SPELL_WIKI_COMPARE/v1', 'license': LICENSE_NOTE, 'target_cut': facts_doc['target_cut'],
                'census_sources': census['sources'],
                'note': 'Rows list differences only; field_counts count every compared field.'}
    for source, value in result.items():
        document[source + '_summary'] = {k: v for k, v in value.items() if k != 'differences'}
        document[source + '_differences'] = value['differences']
    write_lines(args.out, document, None)
    print(json.dumps({s: {'compared': result[s]['compared'], 'unmatched': len(result[s]['unmatched_source']),
                          'with_differences': len(result[s]['differences']),
                          'field_counts': result[s]['field_counts']} for s in result}, indent=1))
    return 0


def write_lines(out, document, _unused):
    """One list element per line so a regenerated file diffs by row."""
    lines = ['{']
    keys = list(document)
    for i, key in enumerate(keys):
        value = document[key]
        tail = ',' if i + 1 < len(keys) else ''
        if isinstance(value, list):
            lines.append(f'  {json.dumps(key)}: [')
            for j, item in enumerate(value):
                lines.append('    ' + json.dumps(item, ensure_ascii=False, sort_keys=True) + (',' if j + 1 < len(value) else ''))
            lines.append('  ]' + tail)
        else:
            lines.append(f'  {json.dumps(key)}: ' + json.dumps(value, ensure_ascii=False, sort_keys=True) + tail)
    lines.append('}')
    out.parent.mkdir(parents=True, exist_ok=True)
    out.write_text('\n'.join(lines) + '\n', encoding='utf-8', newline='\n')


if __name__ == '__main__':
    sys.exit(main())
