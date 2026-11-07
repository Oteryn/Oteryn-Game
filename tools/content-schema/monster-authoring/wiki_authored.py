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
import re
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
monster.race = "{race}"
monster.corpse = {itemid}
monster.speed = 125
monster.manaCost = {mana_cost}

monster.changeTarget = {{
	interval = 4000,
	chance = 10,
}}

monster.strategiesTarget = {{
	nearest = 100,
}}

monster.flags = {{
	summonable = {summonable},
	attackable = true,
	hostile = true,
	convinceable = {convinceable},
	pushable = {pushable},
	rewardBoss = {reward_boss},
	illusionable = {illusionable},
	canPushItems = {push_items},
	canPushCreatures = true,
	staticAttackChance = 90,
	targetDistance = 1,
	runHealth = 0,
	healthHidden = false,
	isBlockable = false,
	canWalkOnEnergy = {walk_energy},
	canWalkOnFire = {walk_fire},
	canWalkOnPoison = {walk_poison},
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
	{{ name = "melee", interval = 2000, chance = 100, minDamage = {melee_min}, maxDamage = -{melee_max} }},
	{{ name = "combat", interval = 2000, chance = 20, type = COMBAT_ENERGYDAMAGE, minDamage = -{exori_min}, maxDamage = -{exori_max}, radius = 1, effect = CONST_ME_ENERGYAREA, target = false }},
	{{ name = "combat", interval = 2000, chance = 20, type = COMBAT_ENERGYDAMAGE, minDamage = -{ball_min}, maxDamage = -{ball_max}, range = 7, radius = 1, shootEffect = CONST_ANI_ENERGY, effect = CONST_ME_ENERGYAREA, target = true }},
}}

monster.defenses = {{
	defense = 40,
	armor = 40,
}}

monster.elements = {{
	{{ type = COMBAT_PHYSICALDAMAGE, percent = {res_physical} }},
	{{ type = COMBAT_ENERGYDAMAGE, percent = {res_energy} }},
	{{ type = COMBAT_EARTHDAMAGE, percent = {res_earth} }},
	{{ type = COMBAT_FIREDAMAGE, percent = {res_fire} }},
	{{ type = COMBAT_LIFEDRAIN, percent = {res_life_drain} }},
	{{ type = COMBAT_MANADRAIN, percent = 0 }},
	{{ type = COMBAT_DROWNDAMAGE, percent = {res_drown} }},
	{{ type = COMBAT_ICEDAMAGE, percent = {res_ice} }},
	{{ type = COMBAT_HOLYDAMAGE, percent = {res_holy} }},
	{{ type = COMBAT_DEATHDAMAGE, percent = {res_death} }},
}}

monster.immunities = {{
	{{ type = "paralyze", condition = {para_immune} }},
	{{ type = "outfit", condition = false }},
	{{ type = "invisible", condition = {invis_immune} }},
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
    'outfit.lookMount': ([], 'no mount is assumed'),
    'manaCost': ([('fandom.monster', 'summon'), ('fandom.monster', 'convince')], None),
    'race': ([('br.monster', 'creatureclass')], 'blood follows the Humans class and the Dark Knight template'),
    'speed': ([], 'speed 125'),
    'defenses.armor': ([], 'armor 40'),
    'defenses.defense': ([], 'defense 40'),
    'flags': ([('fandom.monster', 'pushable'), ('fandom.monster', 'pushobjects'), ('fandom.monster', 'illusionable'),
               ('fandom.monster', 'summon'), ('fandom.monster', 'convince'), ('fandom.monster', 'isboss'),
               ('br.monster', 'ignoresfields')], 'attackable, hostile, pushing creatures, targeting distance and static attack '
                                                  'chance are the Dark Knight template'),
    'flags.pass_through': ([], 'movement flags are the Dark Knight template'),
    'flags.canWalk/canTarget': ([], 'movement flags are the Dark Knight template'),
    'changeTarget': ([], 'target changes'),
    'strategiesTarget': ([], 'targeting strategy'),
    'light': ([], 'no light'),
    'loot': ([('br.monster', 'loot'), ('fandom.monster', 'loot')], None),
    'elements': ([('br.monster', f'{element}DmgMod') for element in ('physical', 'earth', 'fire', 'death', 'energy', 'holy', 'ice')]
                 + [('fandom.monster', 'hpDrainDmgMod'), ('fandom.monster', 'drownDmgMod'),
                    ('br.monster', 'healDmgMod'), ('fandom.monster', 'healMod')],
                 'Fandom marks every element 100%? (uncertain); life drain and drowning are only on Fandom, as 100%?, and mana '
                 'drain is on neither wiki; all take the template 0% reduction, and healing 100% is the default with no modifier'),
    'immunities': ([('fandom.monster', 'paraimmune'), ('fandom.monster', 'senseinvis'), ('br.monster', 'immunities')],
                   'the outfit and bleed condition entries (not immune)'),
    'attacks[1]': ([('br.monster', 'hab_physical')], 'the 2000 ms interval'),
    'attacks[2]': ([('br.monster', 'hab_energy')], 'the 3x3 area is modelled as radius 1 around the caster; the interval, '
                                                  'chance and energy-area effect'),
    'attacks[3]': ([('br.monster', 'hab_energy')], 'the 3x3 ball is modelled as radius 1 on the target at range 7; the '
                                                  'interval, chance and energy effects'),
    'voices': ([], 'a voice interval and chance without any voice text'),
}
# (source, fact, exact source text the omission is approved for, resolution). A changed fact fails regeneration.
OMITTED = [('br.monster', 'hab_earth', '[[Magias de Criaturas#Wave|Wave costas]] (0-???).',
            'TibiaWiki BR lists an earth wave with unknown damage (0-???); it is left out until a '
            'source gives its damage (NEEDS VERIFICATION, D44).'),
]
# These facts support the converter's explicit empty-loot disposition, not template assumptions.
EMPTY_LOOT_FACTS = {'br.monster': 'Nenhum.', 'fandom.monster': '{{Loot Table|}}'}


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
    match = re.search(r'\((\d+)-(\d+)\)', raw)
    if not match:
        raise ValueError(f'no damage range in {raw!r}')
    return int(match.group(1)), int(match.group(2))


def exact_ranges(raw, count):
    """The `(a-b)` damage ranges of a BR ability text, which must have exactly `count` of them and no unknown range."""
    if '?' in raw:
        raise ValueError(f'unknown damage in {raw!r}')
    found = [(int(low), int(high)) for low, high in re.findall(r'\((\d+)-(\d+)\)', raw)]
    if len(found) != count or len(re.findall(r'\(', raw)) != count:
        raise ValueError(f'expected {count} damage ranges in {raw!r}')
    return found


YES, NO = {'yes', 'sim'}, {'no', 'não', '--'}


def boolean(*values):
    """One boolean from wiki yes/no facts; the sources must agree and every value must be a plain yes or no."""
    parsed = set()
    for value in values:
        value = value.strip().lower()
        if value not in YES | NO:
            raise ValueError(f'not a plain yes/no wiki value: {value!r}')
        parsed.add(value in YES)
    if len(parsed) != 1:
        raise ValueError(f'wiki sources disagree: {values}')
    return parsed.pop()


def resistance(value):
    """Canary reduction percent from a wiki damage modifier (100% -> 0, 80% -> 20); an uncertain `?` mark is accepted."""
    match = re.fullmatch(r'(\d+)%\??', value.strip())
    if not match:
        raise ValueError(f'not a wiki damage modifier: {value!r}')
    return 100 - int(match.group(1))


def lua(value):
    return 'true' if value else 'false'


def derived_values(sources):
    """Every source-backed flag, immunity and element of the authored file, derived from the pinned facts."""
    fandom, br = sources['fandom.monster']['facts'], sources['br.monster']['facts']
    summonable, convinceable = fandom['summon']['value'] != '--', fandom['convince']['value'] != '--'
    if summonable or convinceable:
        raise ValueError('a summonable or convinceable wiki-authored monster needs its mana cost authored')
    br_immune = {part.strip().lower() for part in br['immunities']['value'].split(',')}
    ignored = {part.strip().lower() for part in br['ignoresfields']['value'].split(',')}
    if br_immune - {'paralysis', 'invisibility'}:
        raise ValueError(f'unhandled TibiaWiki BR immunities: {sorted(br_immune - {"paralysis", "invisibility"})}')
    if ignored - {'energy', 'fire', 'poison'}:
        raise ValueError(f'unhandled TibiaWiki BR ignored fields: {sorted(ignored - {"energy", "fire", "poison"})}')
    values = {'summonable': lua(summonable), 'convinceable': lua(convinceable), 'mana_cost': 0,
              'pushable': lua(boolean(fandom['pushable']['value'], br['pushable']['value'])),
              'illusionable': lua(boolean(fandom['illusionable']['value'], br['illusionable']['value'])),
              'reward_boss': lua(boolean(fandom['isboss']['value'], br['isboss']['value'])),
              'push_items': lua(boolean(fandom['pushobjects']['value'], br['pushobjects']['value'])),
              'para_immune': lua(boolean(fandom['paraimmune']['value'], 'yes' if 'paralysis' in br_immune else 'no')),
              'invis_immune': lua(boolean(fandom['senseinvis']['value'], 'yes' if 'invisibility' in br_immune else 'no'))}
    for element in ('energy', 'fire', 'poison'):
        values[f'walk_{element}'] = lua(element in ignored)
    for element in ('physical', 'energy', 'earth', 'fire', 'ice', 'holy', 'death'):
        values[f'res_{element}'] = resistance(br[f'{element}DmgMod']['value'])
    values['res_life_drain'] = resistance(fandom['hpDrainDmgMod']['value'])
    values['res_drown'] = resistance(fandom['drownDmgMod']['value'])
    # Canary has no healing modifier: only the default 100% (no change) can be authored.
    if br['healDmgMod']['value'].strip() != '100%' or fandom['healMod']['value'].strip() != '100%':
        raise ValueError('a healing modifier other than 100% cannot be authored in the Canary format')
    creature_class = br['creatureclass']['value'].strip()
    if creature_class not in CLASS_RACE:
        raise ValueError(f'no race for TibiaWiki BR creature class {creature_class!r}')
    values['race'] = CLASS_RACE[creature_class]
    corpse = sources['fandom.corpse']['facts']['name']['value']
    if f'[[{corpse}]]' not in fandom['notes']['value']:
        raise ValueError(f'the Fandom monster notes no longer name the corpse {corpse!r}')
    return values


# The exact raw text of every cited fact that Dark Merudri was authored and reviewed against. The derivations above
# check what they parse; this pin catches everything they do not (labels, shapes, uncertainty marks), so any refreshed
# fact fails regeneration until the authoring is reviewed and the pin updated.
AUTHORED_FOR = {
    ('br.monster', 'creatureclass'): 'Humanos',
    ('br.monster', 'deathDmgMod'): '100%',
    ('br.monster', 'earthDmgMod'): '100%',
    ('br.monster', 'energyDmgMod'): '100%',
    ('br.monster', 'exp'): '0',
    ('br.monster', 'fireDmgMod'): '100%',
    ('br.monster', 'hab_energy'): '[[Melee|Exori 3x3]] (430-550), [[Magias de Criaturas|Ball 3x3]] (290-460).',
    ('br.monster', 'hab_physical'): '[[Melee|Corpo a corpo]] (0-160).',
    ('br.monster', 'healDmgMod'): '100%',
    ('br.monster', 'holyDmgMod'): '100%',
    ('br.monster', 'hp'): '6500',
    ('br.monster', 'iceDmgMod'): '100%',
    ('br.monster', 'ignoresfields'): 'Poison, Fire, Energy',
    ('br.monster', 'immunities'): 'Invisibility, Paralysis',
    ('br.monster', 'name'): 'Dark Merudri',
    ('br.monster', 'loot'): 'Nenhum.',
    ('br.monster', 'physicalDmgMod'): '100%',
    ('fandom.corpse', 'itemid'): '50311',
    ('fandom.monster', 'actualname'): 'dark merudri',
    ('fandom.monster', 'article'): 'a',
    ('fandom.monster', 'convince'): '--',
    ('fandom.monster', 'drownDmgMod'): '100%?',
    ('fandom.monster', 'exp'): '0',
    ('fandom.monster', 'healMod'): '100%',
    ('fandom.monster', 'hp'): '6500',
    ('fandom.monster', 'hpDrainDmgMod'): '100%?',
    ('fandom.monster', 'illusionable'): 'no',
    ('fandom.monster', 'isboss'): 'no',
    ('fandom.monster', 'name'): 'Dark Merudri',
    ('fandom.monster', 'loot'): '{{Loot Table|}}',
    ('fandom.monster', 'notes'): 'It becomes [[Good Remains of a Merudri]] when it dies.',
    ('fandom.monster', 'paraimmune'): 'yes',
    ('fandom.monster', 'pushable'): 'no',
    ('fandom.monster', 'pushobjects'): 'yes',
    ('fandom.monster', 'senseinvis'): 'yes',
    ('fandom.monster', 'summon'): '--',
    ('fandom.outfit', 'male_id'): '1824',
}


def changed_authored_facts(sources):
    """Cited facts whose pinned text differs from the one the authoring was reviewed against."""
    missing = cited_facts() - set(AUTHORED_FOR)
    if missing:
        raise ValueError(f'cited facts with no AUTHORED_FOR pin: {sorted(missing)}')
    return sorted(key for key, value in AUTHORED_FOR.items() if sources[key[0]]['facts'][key[1]]['value'] != value)


# TibiaWiki BR creature class -> Canary race (the blood residue); only classes the authored monsters use.
CLASS_RACE = {'Humanos': 'blood'}


def cited_facts():
    """(source, fact) pairs the manifest cites as the source of an authored value."""
    cited = {(source_name, fact) for cites, _ in FIELDS.values() for source_name, fact in cites}
    return cited | {('fandom.monster', 'name'), ('br.monster', 'name')}


class TrackedFacts(dict):
    """A facts dict that records which facts the authoring code reads."""

    def __init__(self, source_name, facts, used):
        super().__init__(facts)
        self.source_name, self.used = source_name, used

    def __getitem__(self, fact):
        self.used.add((self.source_name, fact))
        return super().__getitem__(fact)


def unread_cited_facts(sources):
    """Cited facts the authoring code never reads, so a change to them could not change or fail the output."""
    used = set()
    tracked = {name: {**source, 'facts': TrackedFacts(name, source['facts'], used)} for name, source in sources.items()}
    lua_text(tracked)
    return sorted(cited_facts() - used)


def verify_empty_loot_facts(sources):
    for source_name, expected in EMPTY_LOOT_FACTS.items():
        if sources[source_name]['facts']['loot']['value'] != expected:
            raise ValueError(f'{source_name} loot changed from the reviewed empty value: re-author it')


def lua_text(sources):
    verify_empty_loot_facts(sources)
    fandom, br = sources['fandom.monster']['facts'], sources['br.monster']['facts']
    if fandom['hp']['value'] != br['hp']['value']:
        raise ValueError('Fandom and TibiaWiki BR disagree on hp')
    if fandom['name']['value'] != br['name']['value']:
        raise ValueError('Fandom and TibiaWiki BR disagree on the name')
    if fandom['exp']['value'] != br['exp']['value'] or not fandom['exp']['value'].isdigit():
        raise ValueError('Fandom and TibiaWiki BR disagree on exp, or it is not a number')
    if not fandom['hp']['value'].isdigit():
        raise ValueError('hp is not a number')
    (melee_min, melee_max), = exact_ranges(br['hab_physical']['value'], 1)
    (exori_min, exori_max), (ball_min, ball_max) = exact_ranges(br['hab_energy']['value'], 2)
    values = {'name': fandom['name']['value'], 'article': fandom['article']['value'], 'actualname': fandom['actualname']['value'], 'exp': int(fandom['exp']['value']),
              'hp': int(fandom['hp']['value']), 'male_id': int(sources['fandom.outfit']['facts']['male_id']['value']),
              'itemid': int(sources['fandom.corpse']['facts']['itemid']['value']),
              'melee_min': -melee_min if melee_min else 0, 'melee_max': melee_max, 'exori_min': exori_min, 'exori_max': exori_max,
              'ball_min': ball_min, 'ball_max': ball_max}
    values.update(derived_values(sources))
    return DARK_MERUDRI.format(**values)


# Values the converter leaves implicit (it emits no row for them), so no converted row carries their provenance.
IMPLICIT = {
    'outfit.lookAddons': {'kind': 'field', 'status': 'mapped', 'destination': '/monster/presentation/appearance/attachment_bindings',
                          'resolution': 'lookAddons 0: the outfit is shown with no addon attachment.'},
    'outfit.lookMount': {'kind': 'field', 'status': 'mapped', 'destination': '/monster/presentation/appearance/attachment_bindings',
                         'resolution': 'lookMount 0: the creature is shown without a mount.'},
    'manaCost': {'kind': 'field', 'status': 'approved_omission',
                 'resolution': 'manaCost 0: the creature can be neither summoned nor convinced, so no summoning mana cost is stored.'},
}
# Empty loot now carries an explicit converted disposition bound to both wiki facts.
OMITTED_KEYS = set()
# Outfit keys the converter maps in rows of their own; the other outfit keys are part of the `outfit` row.
OUTFIT_OWN_ROWS = ('lookAddons', 'lookMount')


def uncovered_keys(text, fields):
    """Keys of the authored Lua with no converted or implicit row: top-level monster.<key> and the outfit keys with own rows."""
    keys = set(re.findall(r'^monster\.(\w+)\s*=', text, re.M))
    keys |= {f'outfit.{key}' for key in OUTFIT_OWN_ROWS if re.search(rf'^\s*{key}\s*=', text, re.M)}
    return sorted(key for key in keys if not any(f == key or f.startswith((key + '.', key + '[')) for f in fields))


def manifest_for(sources, rows, template_lines):
    """Rows of the converted file re-pointed at the wiki sources (and the template for the NEEDS VERIFICATION part)."""
    verify_empty_loot_facts(sources)
    for row in rows:
        if row['source_field'] == 'loot' and (row.get('status') != 'approved_omission' or row.get('destination')):
            raise ValueError('wiki-authored loot must preserve the explicit empty-source omission')
    emitted = {row['source_field'] for row in rows}
    rows = rows + [{'source_field': field, **row} for field, row in IMPLICIT.items() if field not in emitted]
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
    for source_name, fact, expected, text in OMITTED:
        meta = sources[source_name]
        if meta['facts'][fact]['value'] != expected:
            raise ValueError(f'{source_name} {fact} changed from the omitted value {expected!r}: re-author it')
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
    template_text = (converter.canary / cb.MONSTER_DIR / f'{TEMPLATE}.lua').read_text(encoding='utf-8').splitlines()
    for field in IMPLICIT:
        key = field.split('.')[-1]
        template_rows.setdefault(field, next((n for n, line in enumerate(template_text, 1) if key in line), 1))
    if slug != 'dark_merudri':
        raise ValueError(slug)
    result = manifest_for(sources, [dict(r) for r in manifest['entries']], template_rows)
    emitted = {row['source_field'] for row in manifest['entries']}
    missing = set(FIELDS) - emitted - set(IMPLICIT)
    if changed_authored_facts(sources):
        raise ValueError(f'cited wiki facts changed since the authoring was reviewed: {changed_authored_facts(sources)}')
    if unread_cited_facts(sources):
        raise ValueError(f'cited wiki facts the authoring does not read: {unread_cited_facts(sources)}')
    uncovered = uncovered_keys(text, emitted | set(IMPLICIT) | OMITTED_KEYS)
    if uncovered:
        raise ValueError(f'authored keys with no manifest row: {uncovered}')
    if missing:
        raise ValueError(f'mapped fields with no converted row: {sorted(missing)}')
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
    assert '\tpushable = false,' in text and '\tcanPushItems = true,' in text and '\tcanWalkOnPoison = true,' in text
    assert '{ type = "paralyze", condition = true }' in text and 'COMBAT_FIREDAMAGE, percent = 0' in text
    changed = json.loads(json.dumps(sample['sources']))
    for source_name in ('fandom.monster', 'br.monster'):
        changed[source_name]['facts']['pushable']['value'] = {'fandom.monster': 'yes', 'br.monster': 'sim'}[source_name]
    changed['br.monster']['facts']['fireDmgMod']['value'] = '80%'
    changed_text = lua_text(changed)
    assert '\tpushable = true,' in changed_text and 'COMBAT_FIREDAMAGE, percent = 20' in changed_text
    changed['br.monster']['facts']['pushable']['value'] = 'não'
    try:
        lua_text(changed)
    except ValueError:
        pass
    else:
        raise AssertionError('disagreeing wiki pushable facts were accepted')
    assert unread_cited_facts(sample['sources']) == [], unread_cited_facts(sample['sources'])
    assert changed_authored_facts(sample['sources']) == [], changed_authored_facts(sample['sources'])
    relabelled = json.loads(json.dumps(sample['sources']))
    relabelled['br.monster']['facts']['hab_energy']['value'] = '[[Melee|Exori 5x5]] (430-550), [[Magias de Criaturas|Beam]] (290-460).'
    relabelled['fandom.monster']['facts']['healMod']['value'] = '100%?'
    assert changed_authored_facts(relabelled) == [('br.monster', 'hab_energy'), ('fandom.monster', 'healMod')]
    for source_name, fact, value in (('br.monster', 'healDmgMod', '50%'), ('fandom.monster', 'healMod', '100%?'),
                                     ('br.monster', 'creatureclass', 'Mortos-Vivos'), ('fandom.monster', 'notes', 'Unrelated.'),
                                     ('br.monster', 'exp', '100'), ('br.monster', 'immunities', 'Invisibility, Paralysis, Fire'),
                                     ('br.monster', 'hab_physical', '[[Melee|Corpo a corpo]] (0-???).')):
        broken = json.loads(json.dumps(sample['sources']))
        broken[source_name]['facts'][fact]['value'] = value
        try:
            lua_text(broken)
        except ValueError:
            pass
        else:
            raise AssertionError(f'a changed {source_name} {fact} was accepted')
    for source_name, fact, value in (('br.monster', 'hab_earth', '[[Magias de Criaturas#Wave|Wave costas]] (0-300).'),
                                     ('br.monster', 'loot', 'Gold Coin (0-10).')):
        broken = json.loads(json.dumps(sample['sources']))
        broken[source_name]['facts'][fact]['value'] = value
        try:
            manifest_for(broken, [], {})
        except ValueError:
            pass
        else:
            raise AssertionError(f'a stale omission of {source_name} {fact} was approved')
    changed['fandom.monster']['facts']['summon']['value'] = '450'
    try:
        derived_values(changed)
    except ValueError:
        pass
    else:
        raise AssertionError('a summonable wiki monster was authored without a mana cost')
    assert text.startswith('local mType = Game.createMonsterType("Dark Merudri")'), text[:60]
    rows = [{'source_field': field, 'kind': 'field', 'status': 'approved_omission' if field in ('voices', 'loot') else 'mapped',
             **({} if field == 'loot' else {'destination': '/x'}), 'resolution': 'r'} for field in FIELDS if field not in IMPLICIT]
    manifest = manifest_for(sample['sources'], rows, {})
    template = len(manifest['sources']) - 1
    assert all(e['resolution'].startswith('NEEDS VERIFICATION') for e in manifest['entries'] if e['source_index'] == template)
    assert uncovered_keys(text, set(FIELDS) | OMITTED_KEYS) == [], uncovered_keys(text, set(FIELDS) | OMITTED_KEYS)
    assert uncovered_keys(text, (set(FIELDS) | OMITTED_KEYS) - {'outfit.lookMount'}) == ['outfit.lookMount']
    for field, implicit in IMPLICIT.items():
        cites, verify = FIELDS[field]
        if verify or not cites:
            assert [e for e in manifest['entries'] if e['source_field'] == field
                    and e['resolution'].startswith(VERIFY)], f'{field} has no verification entry'
        for source_name, fact in cites:
            assert [e for e in manifest['entries'] if e['source_field'] == f'infobox.{fact}'
                    and e['resolution'].endswith(implicit['resolution'])], f'{field} does not cite {source_name} {fact}'
    assert any(e['source_field'] == 'infobox.name' and e['destination'] == '/monster/creature/display_name'
               for e in manifest['entries'])
    cited = {(manifest['sources'][e['source_index']]['title'], e['source_line']) for e in manifest['entries']
             if e['source_index'] != template}
    br = sample['sources']['br.monster']
    for fact, value in br['facts'].items():
        if fact.endswith('DmgMod'):
            assert (br['title'], value['line']) in cited, f'BR {fact} is not cited'
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
