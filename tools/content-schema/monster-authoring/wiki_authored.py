"""Monsters that Tibia has at the 2026-09-27 target but Canary does not, authored from the wiki (owner decision D44).

Evidence tooling only. Each monster is written as a Canary-format monster file from pinned wiki facts, converted by the
normal converter (canary_batch.py), and given a manifest whose rows cite the wiki revision line each value comes from.
Values that no wiki page gives are taken from the closest Canary template monster and marked NEEDS VERIFICATION (D44):
they are the best available approximation of Tibia, not observed values.

    python wiki_authored.py fetch --br-capture wiki-br-grave-danger.json   # refresh samples/wiki-authored-2026-09-27.json
    python wiki_authored.py self-test
"""
import argparse
import hashlib
import json
import os
import sys
import tempfile
from pathlib import Path

import canary_batch as cb
import wiki_compare as wc

ROOT = Path(__file__).resolve().parent
SAMPLE = ROOT / 'samples' / 'wiki-authored-2026-09-27.json'
DECISION = 'Owner decision D44'
VERIFY = f'NEEDS VERIFICATION ({DECISION}): no wiki page gives this value at the target date; it is taken from the Canary template'

# Pinned wiki revisions, all at or before the 2026-09-27 cut. BR revisions come from the monster-wiki-capture.yml capture.
FANDOM = {'monster': ('Dark Merudri', 1098116), 'corpse': ('Good Remains of a Merudri', 1116306), 'outfit': ('Monk Outfits', 1102608)}
BR = {'monster': ('Dark Merudri', 423733)}
TEMPLATE = 'quests/grave_danger/dark_knight'

# The Dark Merudri file. Each value is a wiki fact or a line of the Dark Knight template (the four Grave Danger darks of
# Canary share it); `{name}` fields are filled from the pinned facts so a fact change fails the digest, not silently.
DARK_MERUDRI = '''local mType = Game.createMonsterType("{name}")
local monster = {{}}

monster.description = "{article} {actualname}"
monster.experience = {exp}
monster.outfit = {{
	lookType = {male_id},
	lookHead = 95,
	lookBody = 95,
	lookLegs = 95,
	lookFeet = 95,
	lookAddons = 0,
	lookMount = 0,
}}

monster.health = {hp}
monster.maxHealth = {hp}
monster.race = "blood"
monster.corpse = {itemid}
monster.speed = 125
monster.manaCost = 0

monster.changeTarget = {{
	interval = 4000,
	chance = 10,
}}

monster.strategiesTarget = {{
	nearest = 100,
}}

monster.flags = {{
	summonable = false,
	attackable = true,
	hostile = true,
	convinceable = false,
	pushable = false,
	rewardBoss = false,
	illusionable = false,
	canPushItems = true,
	canPushCreatures = true,
	staticAttackChance = 90,
	targetDistance = 1,
	runHealth = 0,
	healthHidden = false,
	isBlockable = false,
	canWalkOnEnergy = true,
	canWalkOnFire = true,
	canWalkOnPoison = true,
}}

monster.light = {{
	level = 0,
	color = 0,
}}

monster.voices = {{
	interval = 5000,
	chance = 10,
}}

monster.loot = {{}}

monster.attacks = {{
	{{ name = "melee", interval = 2000, chance = 100, minDamage = 0, maxDamage = -{melee_max} }},
	{{ name = "combat", interval = 2000, chance = 20, type = COMBAT_ENERGYDAMAGE, minDamage = -{exori_min}, maxDamage = -{exori_max}, radius = 1, effect = CONST_ME_ENERGYAREA, target = false }},
	{{ name = "combat", interval = 2000, chance = 20, type = COMBAT_ENERGYDAMAGE, minDamage = -{ball_min}, maxDamage = -{ball_max}, range = 7, radius = 1, shootEffect = CONST_ANI_ENERGY, effect = CONST_ME_ENERGYAREA, target = true }},
}}

monster.defenses = {{
	defense = 40,
	armor = 40,
}}

monster.elements = {{
	{{ type = COMBAT_PHYSICALDAMAGE, percent = 0 }},
	{{ type = COMBAT_ENERGYDAMAGE, percent = 0 }},
	{{ type = COMBAT_EARTHDAMAGE, percent = 0 }},
	{{ type = COMBAT_FIREDAMAGE, percent = 0 }},
	{{ type = COMBAT_LIFEDRAIN, percent = 0 }},
	{{ type = COMBAT_MANADRAIN, percent = 0 }},
	{{ type = COMBAT_DROWNDAMAGE, percent = 0 }},
	{{ type = COMBAT_ICEDAMAGE, percent = 0 }},
	{{ type = COMBAT_HOLYDAMAGE, percent = 0 }},
	{{ type = COMBAT_DEATHDAMAGE, percent = 0 }},
}}

monster.immunities = {{
	{{ type = "paralyze", condition = true }},
	{{ type = "outfit", condition = false }},
	{{ type = "invisible", condition = true }},
	{{ type = "bleed", condition = false }},
}}

mType:register(monster)
'''

# Converter source_field -> the sources of its value: (source, fact) pairs from the sample, and a verification note when
# part of the value comes from the template.
FIELDS = {
    'description': ([('fandom.monster', 'article'), ('fandom.monster', 'actualname')], None),
    'experience': ([('fandom.monster', 'exp'), ('br.monster', 'exp')], None),
    'health': ([('fandom.monster', 'hp'), ('br.monster', 'hp')], None),
    'maxHealth': ([('fandom.monster', 'hp'), ('br.monster', 'hp')], None),
    'corpse': ([('fandom.corpse', 'itemid'), ('fandom.monster', 'notes')], None),
    'outfit': ([('fandom.outfit', 'male_id')], 'the Monk looktype is the Fandom male Monk outfit; the colours (95) are those of '
                                             'the Dark Knight template'),
    'outfit.lookAddons': ([], 'no addons are assumed'),
    'race': ([('br.monster', 'creatureclass')], 'blood follows the Humans class and the Dark Knight template'),
    'speed': ([], 'speed 125'),
    'defenses.armor': ([], 'armor 40'),
    'defenses.defense': ([], 'defense 40'),
    'flags': ([('fandom.monster', 'pushable'), ('fandom.monster', 'pushobjects'), ('fandom.monster', 'illusionable'),
               ('br.monster', 'ignoresfields')], 'pushing creatures, targeting distance and static attack chance are the '
                                                  'Dark Knight template'),
    'flags.pass_through': ([], 'movement flags are the Dark Knight template'),
    'flags.canWalk/canTarget': ([], 'movement flags are the Dark Knight template'),
    'changeTarget': ([], 'target changes'),
    'strategiesTarget': ([], 'targeting strategy'),
    'light': ([], 'no light'),
    'elements': ([('br.monster', 'physicalDmgMod'), ('br.monster', 'energyDmgMod'), ('br.monster', 'earthDmgMod')],
                 'Fandom marks every element 100%? (uncertain); TibiaWiki BR gives 100% for each'),
    'immunities': ([('fandom.monster', 'paraimmune'), ('fandom.monster', 'senseinvis'), ('br.monster', 'immunities')], None),
    'attacks[1]': ([('br.monster', 'hab_physical')], 'the 2000 ms interval'),
    'attacks[2]': ([('br.monster', 'hab_energy')], 'the 3x3 area is modelled as radius 1 around the caster; the interval, '
                                                  'chance and energy-area effect'),
    'attacks[3]': ([('br.monster', 'hab_energy')], 'the 3x3 ball is modelled as radius 1 on the target at range 7; the '
                                                  'interval, chance and energy effects'),
    'voices': ([], 'a voice interval and chance without any voice text'),
}
OMITTED = [('br.monster', 'hab_earth', 'TibiaWiki BR lists an earth wave with unknown damage (0-???); it is left out until a '
                                      'source gives its damage (NEEDS VERIFICATION, D44).'),
           ('br.monster', 'loot', 'TibiaWiki BR gives no loot ("Nenhum"); the corpse is the only drop.')]


def fetch_fandom(title, revision_id):
    page = wc.api({'action': 'query', 'revids': revision_id, 'prop': 'revisions', 'rvprop': 'ids|timestamp|content',
                   'rvslots': 'main'})['query']['pages'][0]
    rev = page['revisions'][0]
    return page['title'], page['pageid'], rev, rev['slots']['main']['content']


def facts_of(content):
    """{field: {'value': raw, 'line': n}} for the top-level `| key = value` lines of the infobox."""
    facts = {}
    for number, line in enumerate(content.splitlines(), 1):
        stripped = line.strip()
        if stripped.startswith('|') and '=' in stripped:
            key, _, value = stripped[1:].partition('=')
            facts.setdefault(key.strip(), {'value': value.strip(), 'line': number})
    return facts


def fetch(br_capture):
    sources = {}
    for name, (title, revision_id) in FANDOM.items():
        page_title, page_id, rev, content = fetch_fandom(title, revision_id)
        if rev['timestamp'] > wc.CUT_TIMESTAMP:
            raise SystemExit(f'{title} revision {revision_id} is after the cut')
        sources[f'fandom.{name}'] = {'api': cb.WIKI_API, 'title': page_title, 'page_id': page_id, 'revision_id': rev['revid'],
                                     'revision_timestamp': rev['timestamp'],
                                     'content_sha256': hashlib.sha256(content.encode('utf-8')).hexdigest(),
                                     'facts': facts_of(content)}
    capture = json.loads(Path(br_capture).read_text(encoding='utf-8'))
    for name, (title, revision_id) in BR.items():
        page = next(p for p in capture['pages'] if p.get('title') == title)
        if page['revision_id'] != revision_id:
            raise SystemExit(f'BR {title} is revision {page["revision_id"]}, pinned {revision_id}')
        sources[f'br.{name}'] = {'api': cb.BR_API, 'title': page['page_title'], 'page_id': page['page_id'],
                                 'revision_id': page['revision_id'], 'revision_timestamp': page['revision_timestamp'],
                                 'content_sha256': page['content_sha256'],
                                 'facts': {k: {'value': v, 'line': page['field_lines'][k]} for k, v in page['fields'].items()
                                           if k in page['field_lines']}}
    document = {'target_cut': wc.TARGET_CUT, 'rule': f'{DECISION}: a monster Tibia has and Canary lacks is authored from the '
                'wiki; values no wiki gives come from the Canary template and are marked NEEDS VERIFICATION.',
                'br_capture': {'workflow': '.github/workflows/monster-wiki-capture.yml',
                               'sha256': hashlib.sha256(Path(br_capture).read_bytes()).hexdigest()},
                'template': f'{cb.MONSTER_DIR}/{TEMPLATE}.lua', 'sources': sources}
    SAMPLE.write_text(json.dumps(document, ensure_ascii=False, indent=1) + '\n', encoding='utf-8', newline='\n')
    print(json.dumps({name: s['revision_id'] for name, s in sources.items()}))


def number_range(raw):
    """(low, high) of the first `(a-b)` in a BR ability text."""
    import re
    match = re.search(r'\((\d+)-(\d+)\)', raw)
    return int(match.group(1)), int(match.group(2))


def lua_text(sources):
    fandom, br = sources['fandom.monster']['facts'], sources['br.monster']['facts']
    if fandom['hp']['value'] != br['hp']['value']:
        raise ValueError('Fandom and TibiaWiki BR disagree on hp')
    if fandom['name']['value'] != br['name']['value']:
        raise ValueError('Fandom and TibiaWiki BR disagree on the name')
    energy = br['hab_energy']['value']
    exori, ball = energy.split('),')[0] + ')', energy.split('),')[1]
    values = {'name': fandom['name']['value'], 'article': fandom['article']['value'], 'actualname': fandom['actualname']['value'], 'exp': int(fandom['exp']['value']),
              'hp': int(fandom['hp']['value']), 'male_id': int(sources['fandom.outfit']['facts']['male_id']['value']),
              'itemid': int(sources['fandom.corpse']['facts']['itemid']['value']),
              'melee_max': number_range(br['hab_physical']['value'])[1]}
    values['exori_min'], values['exori_max'] = number_range(exori)
    values['ball_min'], values['ball_max'] = number_range(ball)
    return DARK_MERUDRI.format(**values)


def manifest_for(sources, rows, template_lines):
    """Rows of the converted file re-pointed at the wiki sources (and the template for the NEEDS VERIFICATION part)."""
    order = ['fandom.monster', 'br.monster', 'fandom.corpse', 'fandom.outfit']
    manifest_sources = [{'kind': 'mediawiki', 'api': sources[n]['api'], 'title': sources[n]['title'], 'page_id': sources[n]['page_id'],
                         'revision_id': sources[n]['revision_id'], 'content_sha256': sources[n]['content_sha256']} for n in order]
    manifest_sources.append({'repository': cb.REPOSITORY, 'revision': cb.REVISION})
    template_index = len(manifest_sources) - 1
    entries = []
    for row in rows:
        field = row['source_field']
        if field not in FIELDS:
            raise ValueError(f'no source mapping for converted field {field}')
        cites, verify = FIELDS[field]
        base = {k: v for k, v in row.items() if k not in ('source_index', 'source_file', 'source_line', 'source_field')}
        for source_name, fact in cites:
            meta = sources[source_name]
            entry = {'source_index': order.index(source_name), 'source_file': meta['title'],
                     'source_line': meta['facts'][fact]['line'], 'source_field': f'infobox.{fact}', **base}
            entry['resolution'] = f'{meta["title"]} revision {meta["revision_id"]} {fact} = "{meta["facts"][fact]["value"][:80]}" ' \
                                  f'({DECISION}). ' + (row.get('resolution') or '')
            entries.append(entry)
        if verify or not cites:
            entry = {'source_index': template_index, 'source_file': f'{cb.MONSTER_DIR}/{TEMPLATE}.lua',
                     'source_line': template_lines.get(field, 1), 'source_field': field, **base}
            entry['resolution'] = (f'{VERIFY} Dark Knight: {verify or field}. ' + (row.get('resolution') or '')).strip()
            entries.append(entry)
    for source_name in ('fandom.monster', 'br.monster'):
        meta = sources[source_name]
        entries.append({'source_index': order.index(source_name), 'source_file': meta['title'],
                        'source_line': meta['facts']['name']['line'], 'source_field': 'infobox.name', 'kind': 'field',
                        'status': 'mapped', 'destination': '/monster/creature/display_name',
                        'resolution': f'{meta["title"]} revision {meta["revision_id"]} name = "{meta["facts"]["name"]["value"]}" '
                                      f'({DECISION}). Original source text, unchanged.'})
    for source_name, fact, text in OMITTED:
        meta = sources[source_name]
        entries.append({'source_index': order.index(source_name), 'source_file': meta['title'],
                        'source_line': meta['facts'][fact]['line'], 'source_field': f'infobox.{fact}', 'kind': 'field',
                        'status': 'approved_omission', 'resolution': text})
    return {'sources': manifest_sources, 'entries': entries}


def bundles(converter):
    """[(slug, monster, deps, catalog, manifest, binding)] for every wiki-authored monster, converted in memory."""
    document = json.loads(SAMPLE.read_text(encoding='utf-8'))
    sources = document['sources']
    text = lua_text(sources)
    template_rows = {}
    with tempfile.TemporaryDirectory() as directory:
        path = Path(directory) / 'dark_merudri.lua'
        path.write_text(text, encoding='utf-8')
        relative = os.path.relpath(Path(directory, 'dark_merudri'), converter.canary / cb.MONSTER_DIR)
        converter.pending_definitions = set()
        slug, monster, deps, catalog, manifest, _ = converter.convert(relative)
        converter.pending_definitions = set()
        _, _, _, _, template_manifest, _ = converter.convert(TEMPLATE)
    for row in template_manifest['entries']:
        template_rows.setdefault(row['source_field'], row['source_line'])
    if slug != 'dark_merudri':
        raise ValueError(slug)
    result = manifest_for(sources, [dict(r) for r in manifest['entries']], template_rows)
    binding = {'source_key': 'oteryn:source.tibiawiki', 'identity_namespace': 'mediawiki/page_id',
               'external_id': str(sources['fandom.monster']['page_id'])}
    return [(slug, monster, deps, catalog, result, binding)]


def self_test():
    content = '{{Infobox Creature\n| hp = 6500\n| exp=0\n| hp = 1\n}}'
    assert facts_of(content) == {'hp': {'value': '6500', 'line': 2}, 'exp': {'value': '0', 'line': 3}}
    assert number_range('[[Melee|Exori 3x3]] (430-550), x') == (430, 550)
    sample = json.loads(SAMPLE.read_text(encoding='utf-8'))
    text = lua_text(sample['sources'])
    assert 'monster.maxHealth = 6500' in text and 'lookType = 1824' in text and 'monster.corpse = 50311' in text, text
    assert 'minDamage = -430, maxDamage = -550' in text and 'minDamage = -290, maxDamage = -460' in text
    assert text.startswith('local mType = Game.createMonsterType("Dark Merudri")'), text[:60]
    rows = [{'source_field': field, 'kind': 'field', 'status': 'approved_omission' if field == 'voices' else 'mapped',
             'destination': '/x', 'resolution': 'r'} for field in FIELDS]
    manifest = manifest_for(sample['sources'], rows, {})
    template = len(manifest['sources']) - 1
    assert all(e['resolution'].startswith('NEEDS VERIFICATION') for e in manifest['entries'] if e['source_index'] == template)
    assert any(e['source_field'] == 'infobox.name' and e['destination'] == '/monster/creature/display_name'
               for e in manifest['entries'])
    for name, source in sample['sources'].items():
        assert source['revision_timestamp'] <= wc.CUT_TIMESTAMP, name
    print('wiki_authored self-test: PASS')


def main(argv=None):
    argv = sys.argv[1:] if argv is None else argv
    if argv == ['self-test']:
        self_test()
        return 0
    parser = argparse.ArgumentParser(description=__doc__.split('\n')[0])
    parser.add_argument('command', choices=['fetch'])
    parser.add_argument('--br-capture', required=True, type=Path, help='wiki-br capture JSON from monster-wiki-capture.yml')
    args = parser.parse_args(argv)
    fetch(args.br_capture)
    return 0


if __name__ == '__main__':
    sys.exit(main())
