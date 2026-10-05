"""Check every converted Spell bundle against TibiaWiki Fandom, TibiaWiki BR and tibiopedia.pl.

Evidence tooling only. For each spell and field the report puts our converted value next to the three
references and classifies it:

- `agree`: every reference that states the field equals our value;
- `ours_differs`: the references agree with each other but not with our value (a likely data defect,
  unless an official change in `official-changes.json` explains it: S11);
- `sources_disagree`: the references disagree; the row names the references our value follows;
- `no_source`: no reference states the field (counted, not listed).

Instant and conjuring spells join the references by spoken words. Runes prefer a unique item ID,
then an unambiguous normalized name when an ID is absent. Conflicting IDs and ambiguous pages
are reported instead of silently taking the first page. These comparisons are evidence checks,
not proof of runtime readiness.

Usage:
    python convert_spells.py --canary <dir> --crystal <dir> --out <bundles>
    python verify_spells.py --bundles <bundles> --out samples/spell-verify-3-sources-<date>.json
    python verify_spells.py self-test
"""
import argparse
import json
import re
import sys
from collections import Counter
from pathlib import Path

import wiki_spells as ws

HERE = Path(__file__).resolve().parent
SAMPLES = HERE / 'samples'
FACTS = {'fandom': SAMPLES / 'wiki-spell-facts-fandom-2026-09-27.json',
         'br': SAMPLES / 'wiki-spell-facts-br-2026-09-27.json',
         'tibiopedia': SAMPLES / 'tibiopedia-spell-facts-2026-09-28.json'}
READINESS = SAMPLES / 'spell-readiness-p2.json'
OFFICIAL = HERE / 'official-changes.json'
REFERENCES = tuple(FACTS)
SPELL_FIELDS = ('levelrequired', 'mana', 'soul', 'premium', 'voc', 'cooldown', 'cooldowngroup', 'cooldowngroup2',
                'subclass', 'secondarygroup', 'basepower', 'amount', 'spellrange')
RUNE_FIELDS = ('itemid', 'levelrequired', 'mlrequired', 'basepower', 'charges', 'vocrequired',
               'cooldown', 'cooldowngroup', 'cooldowngroup2', 'subclass', 'secondarygroup', 'spellrange')
GROUP_FIELDS = ('subclass', 'secondarygroup')
# Page categories that are not cooldown groups: conjuring, familiar, party and stance spells cool down in the
# Support group (wiki_spells.BR_PRIMARY_GROUP); "Party" as a second BR category names no cooldown group.
PRIMARY_GROUP = {'conjure': 'support', 'supply': 'support', 'summon': 'support', 'party': 'support',
                 'stance': 'support'}
SECONDARY_GROUP = {'party': None, 'virtude': 'virtue'}
VARIES = ('różnie', 'różni', 'zmienna')  # tibiopedia.pl for a value that varies (party spells)
# Explicit source spelling/name differences, not fuzzy or category-derived matches.
RUNE_ALIASES = {'antidote rune': 'cure poison rune', 'desintegrate rune': 'disintegrate rune',
                'energybomb rune': 'energy bomb rune', 'firebomb rune': 'fire bomb rune',
                'paralyze rune': 'paralyse rune'}


def rune_name(value):
    name = re.sub(r'\s+\(item\)$', '', ws.plain(value).lower())
    return RUNE_ALIASES.get(name, name)


def rune_item_id(spell):
    """Only known legacy numeric item-key namespaces carry a Tibia item ID."""
    key = spell.get('rune', {}).get('item', {}).get('key', '')
    match = re.fullmatch(r'(?:candidate:item/|legacyitem:)([1-9][0-9]*)', key)
    return int(match[1]) if match else None


def group_key(value):
    """'Ultimate Strikes' / 'ultimatestrikes' -> 'ultimatestrikes' (S9 group keys)."""
    return re.sub(r'[^a-z]', '', ws.plain(str(value)).lower()) or None


def reference_value(field, raw, fields=None):
    if raw is None or ws.plain(str(raw)) in ('', '-', '?'):
        return None
    if field == 'subclass':
        key = group_key(raw)
        return PRIMARY_GROUP.get(key, key)
    if field == 'secondarygroup':
        key = group_key(raw)
        return SECONDARY_GROUP.get(key, key)
    if ws.plain(str(raw)).lower().rstrip('.') in VARIES:
        return 'varies'
    if field == 'cooldowngroup2' and not reference_value('secondarygroup', (fields or {}).get('secondarygroup')):
        return None  # Fandom writes a second group time (often 0) on spells without a second group
    if field in ('charges', 'itemid'):
        return ws.wiki_number(raw)
    return ws.crosswalk_value(field, raw)


def our_spell_values(spell):
    """Our bundle in reference terms (the Fandom field names and crosswalk units)."""
    req, costs, groups = spell['requirements'], spell['costs'], spell.get('groups', [])
    values = {'levelrequired': req.get('level'), 'soul': costs.get('soul'),
              'premium': 'yes' if req.get('premium') else 'no',
              'voc': sorted({ws.BASE_VOCATION.get(v.replace('_', ' '), v) for v in req.get('vocations', [])}) or None,
              'cooldown': spell.get('cooldown_ms'), 'basepower': spell.get('base_power')}
    mana = costs.get('mana')
    values['mana'] = mana if isinstance(mana, int) else 'varies' if mana is not None else None
    native = spell.get('execution', {}).get('native_behavior', {})
    if native.get('key') == 'party_buff':
        party_mana = native.get('parameters', {}).get('mana', {})
        if party_mana.get('mode') == 'scaled':
            values['mana'] = 'varies'  # S27 C.3: mana scales with the members.
        elif party_mana.get('mode') == 'fixed':
            values['mana'] = party_mana.get('base')  # Native execution charges this cost itself.
    values['spellrange'] = spell.get('targeting', {}).get('range_tiles')
    for index, group in enumerate(groups[:2]):
        values['cooldowngroup' + ('2' if index else '')] = group.get('cooldown_ms')
        values['subclass' if index == 0 else 'secondarygroup'] = group_key(group.get('group'))
    conjure = spell.get('execution', {}).get('conjure')
    if conjure:
        values['amount'] = conjure.get('count')
    return values


def our_rune_values(spell):
    rune = spell.get('rune', {})
    values = our_spell_values(spell)
    values.update(itemid=rune_item_id(spell), mlrequired=rune.get('magic_level'),
                  charges=rune.get('charges'), vocrequired=values['voc'])
    return values


class References:
    def __init__(self, documents):
        self.spells, self.runes, self.rune_ids = {}, {}, {}
        for name, doc in documents.items():
            spells, runes, rune_ids = {}, {}, {}
            for page in doc['pages']:
                fields = page.get('fields', {})
                if page.get('template') == 'Infobox Spell':
                    words = ws.words_key(fields.get('words', ''))
                    if words:
                        spells.setdefault(words, page)
                elif page.get('template') == 'Infobox Object':
                    for key in {rune_name(fields.get(k, '')) for k in ('name', 'actualname')} | {rune_name(page['title'])}:
                        if key:
                            runes.setdefault(key, []).append(page)
                    item_id = reference_value('itemid', fields.get('itemid'))
                    if item_id is not None:
                        rune_ids.setdefault(item_id, []).append(page)
            self.spells[name], self.runes[name], self.rune_ids[name] = spells, runes, rune_ids

    def spell_pages(self, words, name):
        """Pages by spoken words; words plus a parameter ("utevo res <name>") only when unambiguous or named."""
        found = {}
        for ref, pages in self.spells.items():
            if words in pages:
                found[ref] = pages[words]
                continue
            match = [key for key in pages if key.startswith(words + ' ')]
            named = [key for key in match if ws.plain(pages[key]['fields'].get('name', pages[key]['title'])).lower() == name]
            if len(named) == 1 or len(match) == 1:
                found[ref] = pages[(named or match)[0]]
        return found

    def rune_matches(self, name, item_id=None):
        found, issues = {}, []
        for ref, pages in self.runes.items():
            by_id = self.rune_ids[ref].get(item_id, []) if item_id is not None else []
            matches = by_id or pages.get(rune_name(name), [])
            if not matches:
                continue
            if len(matches) > 1:
                issues.append({'reference': ref, 'reason': 'ambiguous_item_id' if by_id else 'ambiguous_name',
                               'itemid': item_id, 'pages': [page_ref(ref, p) for p in matches]})
                continue
            page = matches[0]
            reference_id = reference_value('itemid', page['fields'].get('itemid'))
            if item_id is not None and reference_id is not None and reference_id != item_id:
                issues.append({'reference': ref, 'reason': 'conflicting_item_id', 'itemid': item_id,
                               'reference_itemid': reference_id, 'pages': [page_ref(ref, page)]})
                continue
            found[ref] = page
        return found, issues

    def rune_pages(self, name, item_id=None):
        return self.rune_matches(name, item_id)[0]


def page_ref(ref, page):
    if 'url' in page:
        return {'reference': ref, 'url': page['url']}
    return {'reference': ref, 'title': page['title'], 'revision_id': page.get('revision_id')}


def classify(ours, found):
    if not found:
        return 'no_source', []
    if all(v == ours for v in found.values()):
        return 'agree', sorted(found)
    if len(set(json.dumps(v, sort_keys=True) for v in found.values())) == 1:
        return 'ours_differs', []
    return 'sources_disagree', sorted(r for r, v in found.items() if v == ours)


def verify(bundles, references, readiness, official):
    status = {(r['spell_type'], r['name']): r['status'] for r in readiness['spells']}
    changes = {(c['spell'], c['field']): c for c in official['changes']}
    counts, rows, unmatched, matching_issues = {}, [], [], []
    for spell in bundles:
        carrier = spell['carrier']
        name = spell['name'].lower()
        spell_type = 'rune' if carrier == 'rune' else 'instant'
        if carrier == 'rune':
            pages, issues = references.rune_matches(name, rune_item_id(spell))
            matching_issues.extend({'spell_type': spell_type, 'name': name, **issue} for issue in issues)
            ours, fields = our_rune_values(spell), RUNE_FIELDS
        else:
            pages = references.spell_pages(ws.words_key(spell.get('words', '')), name)
            ours, fields = our_spell_values(spell), SPELL_FIELDS
        if not pages:
            unmatched.append({'spell_type': spell_type, 'name': name, 'words': spell.get('words')})
            continue
        for field in fields:
            found = {}
            for ref, page in pages.items():
                raw = page['fields'].get('charges' if field == 'charges' else field)
                value = reference_value(field, raw, page['fields'])
                if value is not None:
                    found[ref] = value
            verdict, follows = classify(ours.get(field), found)
            counts.setdefault(field, Counter())[verdict] += 1
            if verdict in ('ours_differs', 'sources_disagree'):
                row = {'spell_type': spell_type, 'name': name, 'status': status.get((spell_type, name)),
                       'field': field, 'verdict': verdict, 'ours': ours.get(field), 'references': found,
                       'follows': follows, 'pages': [page_ref(r, pages[r]) for r in sorted(pages)]}
                # Our value follows one reference while the other two agree: 2 of 3 say otherwise.
                others = [v for r, v in found.items() if r not in follows]
                if verdict == 'sources_disagree' and len(follows) == 1 and len(others) == 2 and others[0] == others[1]:
                    row['minority'] = True
                change = changes.get((name, field))
                if change:
                    row['official_change'] = {k: change[k] for k in ('date', 'source', 'value')}
                rows.append(row)
    summary = {'bundles': len(bundles), 'unmatched': len(unmatched),
               'field_counts': {f: dict(sorted(c.items())) for f, c in sorted(counts.items())},
               'verdicts': dict(sorted(Counter(r['verdict'] for r in rows).items())),
               'ours_differs_ready': sum(1 for r in rows if r['verdict'] == 'ours_differs' and r['status'] == 'ready'),
               'minority': sum(1 for r in rows if r.get('minority')),
               'minority_ready': sum(1 for r in rows if r.get('minority') and r['status'] == 'ready'),
               'reference_match_issues': matching_issues}
    return summary, rows, unmatched


def load_bundles(directory):
    return [json.loads(path.read_text(encoding='utf-8'))['spell'] for path in sorted(directory.glob('*/spell.json'))]


def self_test():
    documents = {
        'fandom': {'pages': [{'template': 'Infobox Spell', 'title': 'Ice Strike', 'revision_id': 1,
                              'fields': {'words': 'exori frigo', 'levelrequired': '8', 'mana': '20', 'cooldown': '2',
                                         'subclass': 'Attack', 'voc': '[[Druid]]s and [[Sorcerer]]s'}},
                             {'template': 'Infobox Object', 'title': 'Sudden Death Rune',
                              'fields': {'name': 'Sudden Death Rune', 'levelrequired': '45', 'mlrequired': '15'}}]},
        'br': {'pages': [{'template': 'Infobox Spell', 'title': 'Ice Strike', 'revision_id': 2,
                          'fields': {'words': 'exori frigo', 'levelrequired': '8', 'mana': '25', 'cooldown': '2'}}]},
        'tibiopedia': {'pages': [{'template': 'Infobox Spell', 'title': 'Ice Strike', 'url': 'u',
                                  'fields': {'words': 'exori frigo', 'levelrequired': '8', 'mana': '20',
                                             'cooldown': '4', 'voc': 'Druid, Sorcerer'}},
                                 {'template': 'Infobox Object', 'title': 'Sudden Death Rune', 'url': 'v',
                                  'fields': {'name': 'Sudden Death Rune', 'levelrequired': '45', 'mlrequired': '15',
                                             'charges': '3'}}]}}
    ice = {'name': 'Ice Strike', 'carrier': 'instant', 'words': 'exori frigo', 'cooldown_ms': 2000,
           'requirements': {'level': 8, 'premium': False, 'vocations': ['druid', 'elder_druid', 'sorcerer']},
           'costs': {'mana': 20, 'soul': 0}, 'groups': [{'group': 'attack', 'cooldown_ms': 2000}],
           'execution': {'ability': {}}}
    sd = {'name': 'sudden death rune', 'carrier': 'rune', 'requirements': {'level': 45}, 'costs': {'mana': 0, 'soul': 0},
          'rune': {'magic_level': 15, 'charges': 5}, 'execution': {'ability': {}}}
    summary, rows, unmatched = verify([ice, sd], References(documents), {'spells': []}, {'changes': []})
    verdicts = {(r['name'], r['field']): (r['verdict'], r['follows']) for r in rows}
    assert verdicts == {('ice strike', 'mana'): ('sources_disagree', ['fandom', 'tibiopedia']),
                        ('ice strike', 'cooldown'): ('sources_disagree', ['br', 'fandom']),
                        ('sudden death rune', 'charges'): ('ours_differs', [])}, verdicts
    assert summary['field_counts']['levelrequired'] == {'agree': 2}, summary
    assert summary['minority'] == 0 and reference_value('mana', 'różnie') == 'varies', summary
    assert summary['field_counts']['voc'] == {'agree': 1}, summary
    assert summary['field_counts']['subclass'] == {'agree': 1, 'no_source': 1}, summary
    assert not unmatched
    assert group_key('Ultimate Strikes') == 'ultimatestrikes' and reference_value('mana', '?') is None
    assert reference_value('cooldowngroup2', '2', {}) is None
    assert reference_value('cooldowngroup2', '30', {'secondarygroup': 'Ultimate Strikes'}) == 30000
    assert reference_value('subclass', 'Conjure') == 'support' and reference_value('secondarygroup', 'Party') is None
    print('verify_spells self-test: ok')
    return 0


def main(argv=None):
    parser = argparse.ArgumentParser(description=__doc__.splitlines()[0])
    parser.add_argument('command', nargs='?', choices=('verify', 'self-test'), default='verify')
    parser.add_argument('--bundles', type=Path)
    parser.add_argument('--out', type=Path)
    args = parser.parse_args(argv)
    if args.command == 'self-test':
        return self_test()
    documents = {name: json.loads(path.read_text(encoding='utf-8')) for name, path in FACTS.items()}
    summary, rows, unmatched = verify(load_bundles(args.bundles), References(documents),
                                      json.loads(READINESS.read_text(encoding='utf-8')),
                                      json.loads(OFFICIAL.read_text(encoding='utf-8')))
    document = {'schema': 'OTERYN_SPELL_VERIFY/v1',
                'references': {name: {'file': path.name, 'cut': documents[name]['target_cut']}
                               for name, path in FACTS.items()},
                'readiness': READINESS.name, 'official_changes': OFFICIAL.name,
                'note': ('Rows list ours_differs and sources_disagree only; field_counts count every compared field. '
                         'reference_match_issues records rejected conflicting or ambiguous rune joins. '
                         'Missing evidence is not agreement; comparison results do not establish runtime readiness.'),
                'summary': summary, 'rows': rows, 'unmatched': unmatched}
    ws.write_lines(args.out, document, None)
    print(json.dumps(summary, indent=1))
    return 0


if __name__ == '__main__':
    sys.exit(main())
