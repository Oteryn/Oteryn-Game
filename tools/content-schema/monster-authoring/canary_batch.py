"""Convert a bounded batch of pinned Canary monster definitions into candidate v1 authoring bundles.

Evidence tooling only: every output is OTS_HYPOTHESIS_ONLY source evidence, never Game truth.
Keys use the provisional `canary:` namespace; they are source-scoped and are not native Oteryn
identities. The declarative monster Lua files are evaluated in a stubbed LuaJIT sandbox (lupa)
that records the table passed to `mType:register`; no Canary engine code runs. Engine rules
(defaults, scales, branch behaviour) are transcribed from the pinned C++/Lua sources cited in
RULES and every value that needs an owner decision is left as an unresolved manifest row.

Usage: python canary_batch.py --canary <checkout of opentibiabr/canary at REVISION> [--out DIR]
"""
import argparse
import hashlib
import json
import re
import subprocess
import xml.etree.ElementTree as ET
from decimal import Decimal
from fractions import Fraction
from pathlib import Path

from normalize_monster_fields import cast_geometry

ROOT = Path(__file__).resolve().parent
REPOSITORY = 'opentibiabr/canary'
REVISION = '47dfd51f45280a59a1d3e50ba7edd573d7234446'
REV = 'canary-47dfd51f'
MONSTER_DIR = 'data-otservbr-global/monster'
BATCH = ['mammals/rat', 'giants/cyclops', 'humanoids/orc_spearman', 'vermins/scorpion', 'humanoids/orc_shaman',
         'humans/necromancer', 'elementals/fire_elemental', 'dragons/dragon', 'undeads/ghost',
         'quests/killing_in_the_name_of/demodras']
SECOND_BATCH = ['dragons/hydra', 'humans/warlock', 'humanoids/dworc_voodoomaster', 'event_creatures/the_halloween_hare',
                'dragons/wyrm', 'bosses/zushuka', 'bosses/black_knight',
                'quests/forgotten_knowledge/bosses/the_enraged_thorn_knight',
                'quests/cults_of_tibia/bosses/summons/sand_vortex', 'familiars/paladin_familiar']
BOSSTIARY_LEVELS = {'bane': [(25, 5), (100, 15), (300, 30)],
                   'archfoe': [(5, 10), (20, 30), (60, 60)],
                   'nemesis': [(1, 10), (3, 30), (5, 60)]}
RULES = {
    'loot_scale': 'src/utils/const.hpp MAX_LOOTCHANCE=100000, so percent = chance/1000',
    'loot_order': 'register_monster_type.lua SortLootByChance sorts the table by ascending chance before registration',
    'loot_defaults': 'creatures_definitions.hpp LootBlock countmin=countmax=1; unique=false',
    'item_names': 'items.cpp nameToItems: appearance names, overridden per id by items.xml names; lookup is lower-case',
    'item_flags': 'items.cpp: blockSolid=unpass, blockProjectile=unsight, blockPathFind=avoid, pickupable=take, movable=!unmove',
    'monster_defaults': 'monsters.hpp MonsterInfo defaults',
    'melee_chance': 'monsters.cpp deserializeSpell: melee is scheduled by interval; the declared chance is not applied',
    'condition': 'monsters.cpp deserializeSpell: |min|,|max| totals, max=0 -> min, startDamage>min -> 0, tick default 2000 ms',
    'area_effect': 'register_monster_type.lua readSpell: an area spell without effect gets CONST_ME_POFF unless its name contains "field"',
    'fields': 'utils_definitions.hpp ITEM_FIREFIELD_PVP_FULL=2118, ITEM_POISONFIELD_PVP=105, ITEM_ENERGYFIELD_PVP=2122',
    'elements': 'register_monster_type.lua elements: values may be clipped by server config MIN/MAX_ELEMENTAL_RESISTANCE',
    'race_residue': 'creature.cpp dropCorpse (identical in Crystal be61cdd/ac447fef): venom/blood/ink/chocolate/candy create '
                    'ITEM_FULLSPLASH=2886 with that fluid; undead/fire/energy/none create nothing; summons drop no corpse',
}
# Owner decisions 2026-09-26 (docs/architecture/OTERYN_MONSTER_AUTHORING_SCHEMA_V1.md section 3).
PASS_THROUGH_DEFAULT = 'Owner decision D5: imported monsters without a source counterpart default to pass_through=false.'
QUEST_EVENT_OMISSION = ('Owner decision D6: creature event scripts that only feed quest/task progress belong to Quest/Interaction '
                        'and are omitted from the monster bundle.')
RACE_RESIDUE = {'venom': 'slime', 'blood': 'blood', 'ink': 'ink', 'chocolate': 'chocolate', 'candy': 'candy',
                'undead': None, 'fire': None, 'energy': None}
SPLASH_ITEM = 2886
DAMAGE = {'PHYSICALDAMAGE': 'physical', 'ENERGYDAMAGE': 'energy', 'EARTHDAMAGE': 'earth', 'FIREDAMAGE': 'fire',
          'LIFEDRAIN': 'life_drain', 'MANADRAIN': 'mana_drain', 'DROWNDAMAGE': 'drowning', 'ICEDAMAGE': 'ice',
          'HOLYDAMAGE': 'holy', 'DEATHDAMAGE': 'death', 'AGONYDAMAGE': 'agony', 'NEUTRALDAMAGE': 'neutral',
          'HEALING': 'healing'}
IMMUNITY_DAMAGE = {'physical': 'physical', 'energy': 'energy', 'earth': 'earth', 'fire': 'fire', 'lifedrain': 'life_drain',
                   'manadrain': 'mana_drain', 'drown': 'drowning', 'ice': 'ice', 'holy': 'holy', 'death': 'death'}
FIELD_ITEMS = {'firefield': 2118, 'poisonfield': 105, 'energyfield': 2122}
OCCURRENCE = {0: 'common', 1: 'uncommon', 2: 'rare', 3: 'very_rare'}
DIFFICULTY = {0: 'harmless', 1: 'trivial', 2: 'easy', 3: 'medium', 4: 'hard', 5: 'challenging'}
PERIOD = {'RESPAWNPERIOD_ALL': 'all', 'RESPAWNPERIOD_DAY': 'day', 'RESPAWNPERIOD_NIGHT': 'night'}

LUA_PRELUDE = r'''
local registered = {}
local callbacks = {}
setmetatable(_G, {__index = function(_, k) return "@" .. k end})
Game = {createMonsterType = function(name)
  local mt = {}
  mt.register = function(self, monster) registered.name = name; registered.monster = monster end
  return setmetatable(mt, {__newindex = function(t, k, v) rawset(callbacks, k, type(v)) end})
end}
return registered, callbacks
'''


def blob_id(data):
    return hashlib.sha1(b'blob %d\0' % len(data) + data).hexdigest()


def source_blob(canary, path):
    """Git object identity is independent of checkout line-ending conversion."""
    return subprocess.check_output(['git', '-C', str(canary), 'rev-parse', f'{REVISION}:{path}'], text=True).strip()


def lua_value(value):
    if hasattr(value, 'items') and not isinstance(value, dict):
        items = list(value.items())
        array = [v for k, v in sorted((k, v) for k, v in items if isinstance(k, int))]
        named = {k: lua_value(v) for k, v in items if not isinstance(k, int)}
        if named:
            if array:
                named['_list'] = [lua_value(v) for v in array]
            return named
        return [lua_value(v) for v in array]
    if isinstance(value, float) and value.is_integer():
        return int(value)
    return value


def load_monster(path):
    from lupa.luajit21 import LuaRuntime
    lua = LuaRuntime(unpack_returned_tuples=True)
    registered, callbacks = lua.execute(LUA_PRELUDE)
    lua.execute(path.read_text(encoding='utf-8'))
    return registered['name'], lua_value(registered['monster']), dict(callbacks.items())


def constant(value, prefix):
    if not isinstance(value, str) or not value.startswith('@' + prefix):
        raise ValueError(f'expected {prefix} constant, got {value!r}')
    return value[len(prefix) + 1:]


def varint(data, pos):
    shift = result = 0
    while True:
        byte = data[pos]
        pos += 1
        result |= (byte & 0x7F) << shift
        if byte < 0x80:
            return result, pos
        shift += 7


def fields(data):
    pos = 0
    while pos < len(data):
        key, pos = varint(data, pos)
        number, wire = key >> 3, key & 7
        if wire == 0:
            value, pos = varint(data, pos)
        elif wire == 2:
            size, pos = varint(data, pos)
            value, pos = data[pos:pos + size], pos + size
        elif wire == 5:
            value, pos = data[pos:pos + 4], pos + 4
        elif wire == 1:
            value, pos = data[pos:pos + 8], pos + 8
        else:
            raise ValueError('unsupported protobuf wire type')
        yield number, value


def load_appearance_objects(path):
    """Minimal decoder for Appearances.object -> {id: name, flag subset} (src/protobuf/appearances.proto)."""
    flag_numbers = {5: 'container', 13: 'unpass', 14: 'unmove', 15: 'unsight', 16: 'avoid', 18: 'take', 42: 'corpse'}
    objects = {}
    for number, value in fields(path.read_bytes()):
        if number != 1:
            continue
        record = {'flags': {}}
        for inner, inner_value in fields(value):
            if inner == 1:
                record['id'] = inner_value
            elif inner == 4:
                record['name'] = inner_value.decode('utf-8', 'replace')
            elif inner == 3:
                for flag, flag_value in fields(inner_value):
                    if flag in flag_numbers:
                        record['flags'][flag_numbers[flag]] = bool(flag_value) if isinstance(flag_value, int) else True
        objects[record['id']] = record
    return objects


def load_items_xml(path):
    items = {}
    for node in ET.parse(path).getroot().iter('item'):
        attributes = {a.get('key').lower(): a.get('value') for a in node.findall('attribute')}
        record = {'name': node.get('name'), 'article': node.get('article'), 'attributes': attributes}
        if node.get('id'):
            ids = [int(node.get('id'))]
        else:
            low, high = int(node.get('fromid')), int(node.get('toid'))
            ids = list(range(low, high + 1)) if low <= high else []
        for item_id in ids:
            items[item_id] = record
    return items


def name_index(objects, items):
    names = {i: o['name'] for i, o in objects.items() if o.get('name')}
    for item_id, record in items.items():
        if record['name']:
            names[item_id] = record['name']
    index = {}
    for item_id, name in names.items():
        index.setdefault(name.lower(), []).append(item_id)
    return names, index


def slug(name):
    return re.sub(r'[^a-z0-9]+', '_', name.lower()).strip('_')


def ref(family, key):
    return {'family': family, 'key': key, 'revision': REV}


def ident(key):
    return {'key': key, 'revision': REV}


def ratio(value):
    fraction = value if isinstance(value, Fraction) else Fraction(Decimal(str(value)))
    return {'numerator': fraction.numerator, 'denominator': fraction.denominator}


def percent_from_chance(chance):
    value = Decimal(min(int(chance), 100000)) / 1000
    return int(value) if value == value.to_integral_value() else float(value)


class Converter:
    def __init__(self, canary, objects, items, names, index):
        self.canary, self.objects, self.items, self.names, self.index = canary, objects, items, names, index

    def convert(self, relative):
        path = self.canary / MONSTER_DIR / (relative + '.lua')
        text = path.read_text(encoding='utf-8')
        lines = text.splitlines()
        name, m, callbacks = load_monster(path)
        s = slug(name)
        source_file = f'{MONSTER_DIR}/{relative}.lua'
        rows = []
        assets = set()
        definitions = set()

        def line_of(pattern, start=0):
            for number, line in enumerate(lines[start:], start + 1):
                if re.search(pattern, line):
                    return number
            return 1

        def row(field, status, kind='field', destination=None, resolution=None, line=None):
            entry = {'source_index': 0, 'source_file': source_file, 'source_line': line or line_of(r'\b' + re.escape(field.split('.')[0]) + r'\b'),
                     'source_field': field, 'kind': kind, 'status': status}
            if destination:
                entry['destination'] = destination
            if resolution:
                entry['resolution'] = resolution
            rows.append(entry)

        def asset(key):
            assets.add(key)
            return key

        flags = m.get('flags', {})
        familiar = None
        if flags.get('familiar'):
            vocation = s.removesuffix('_familiar')
            spell_path = f'data/scripts/spells/familiar/{vocation}_familiar.lua'
            spell_text = (self.canary / spell_path).read_text(encoding='utf-8')
            config_text = (self.canary / 'config.lua.dist').read_text(encoding='utf-8')
            minutes = int(re.search(r'^familiarTime\s*=\s*(\d+)\s*$', config_text, re.M)[1])
            mana = int(re.search(r'spell:mana\((\d+)\)', spell_text)[1])
            ability_key = f'canary:ability/summon-{s}'
            familiar = {'vocation': vocation, 'summon_ability': ref('Ability', ability_key),
                        'duration_ms': minutes * 30000, 'mana_cost': mana}
            definitions.add(('Ability', ability_key))
            # This is the source base appearance, not the player's chosen familiar skin.
            system = (self.canary / 'data/libs/systems/familiar.lua').read_text(encoding='utf-8')
            base_look = int(re.search(r'VOCATION\.BASE_ID\.' + vocation.upper() + r'\].*?id\s*=\s*(\d+)', system)[1])
        defenses = m.get('defenses', {})
        if isinstance(defenses, list):
            defenses = {'_list': defenses}
        immunities = m.get('immunities', [])
        condition_immune = sorted({i['type'] for i in immunities if i.get('condition')})
        damage_immune = sorted({IMMUNITY_DAMAGE[i['type']] for i in immunities if i.get('combat')})

        # Dependencies: abilities, effects and formulas from attacks/defenses.
        deps = {'abilities': [], 'effects': [], 'formulas': [], 'documents': [], 'items': [], 'loot_tables': []}
        schedules = {'attacks': [], 'defenses': []}
        block_start = {'attacks': line_of(r'^monster\.attacks\s*='), 'defenses': line_of(r'^monster\.defenses\s*=')}
        spell_lists = {'attacks': m.get('attacks', []), 'defenses': defenses.get('_list', []) if isinstance(defenses, dict) else defenses}
        for group, spells in spell_lists.items():
            cursor = block_start[group]
            for n, spell in enumerate(spells, 1):
                line = line_of(r'^\s*\{\s*name\s*=', cursor)
                cursor = line
                base = f'canary:{{}}/{s}/{group[:-1]}-{n}'
                pointer = f'/monster/behavior/{group}/{len(schedules[group])}'
                result = self.spell(spell, base, deps, asset)
                if result is None:
                    row(f'{group}[{n}].name={spell.get("name")}', 'unresolved_semantics', 'script', resolution=
                        'Spell name is not an inline Canary branch; it resolves to a registered spell script that needs a native behaviour decision.', line=line)
                    continue
                ability_key, note = result
                chance = 100 if spell.get('name') == 'melee' else spell.get('chance', 100)
                schedules[group].append({'ability': ref('Ability', ability_key), 'interval_ms': spell.get('interval', 2000),
                                         'chance_percent': chance})
                row(f'{group}[{n}]', 'mapped', destination=pointer, line=line,
                    resolution=(note + ('Melee chance normalized to 100: ' + RULES['melee_chance'] + '.' if spell.get('name') == 'melee' else '')).strip()
                    or 'Inline Canary spell branch mapped to Ability/Effect/Formula.')

        # Corpse Item payload and decay chain.
        corpse_id = m.get('corpse', 0)
        if corpse_id:
            chain = corpse_id
            seen = set()
            while chain and chain not in seen:
                seen.add(chain)
                payload, chain, note = self.item_payload(chain, asset)
                deps['items'].append(payload)
            row('corpse', 'mapped', destination='/monster/creature/corpse_item', line=line_of(r'^monster\.corpse'),
                resolution='Corpse and its decay chain are local capability projections from appearances.dat + items.xml; '
                           + RULES['item_flags'] + '. Weight absent in items.xml is the engine default 0. Duration seconds -> ms.')

        # Loot (sorted as the registrar does).
        loot_entries = []
        source_loot = m.get('loot', [])
        order = sorted(range(len(source_loot)), key=lambda i: source_loot[i].get('chance', 0))
        loot_line = line_of(r'^monster\.loot\s*=')
        for position in order:
            entry = source_loot[position]
            line = line_of(r'^\s*\{\s*(name|id)\s*=', loot_line)
            for _ in range(position):
                line = line_of(r'^\s*\{\s*(name|id)\s*=', line)
            if 'id' in entry:
                item_id = int(entry['id'])
            else:
                candidates = self.index.get(entry['name'].lower(), [])
                if len(candidates) != 1:
                    row(f'loot[{position + 1}].name={entry["name"]}', 'unresolved_dependency', 'dependency', line=line,
                        resolution=f'Item name resolves to {len(candidates)} ids {sorted(candidates)[:5]}; Canary nameToItems is not unique for this name.')
                    continue
                item_id = candidates[0]
            item_key = f'canary:item/{item_id}'
            definitions.add(('Item', item_key))
            loot_entry = {'item': ref('Item', item_key), 'min_count': entry.get('minCount', 1),
                          'max_count': entry.get('maxCount', 1), 'probability_percent': percent_from_chance(entry.get('chance', 0)),
                          'skip_later_same_item_after_success': bool(entry.get('unique', False))}
            if 'subType' in entry or 'charges' in entry:
                loot_entry['instance_attributes'] = {'charges': entry.get('subType', entry.get('charges'))}
            if 'child' in entry:
                row(f'loot[{position + 1}].child', 'unresolved_dependency', 'dependency', line=line,
                    resolution='Nested container loot needs a local container Item payload; not generated in this batch.')
            loot_entries.append(loot_entry)
            row(f'loot[{position + 1}]', 'mapped', 'dependency', f'/monster/loot/entries/{len(loot_entries) - 1}', line=line,
                resolution=f'{RULES["loot_scale"]}. Item {item_id} "{self.names.get(item_id)}" is a declared source-scoped reference, not an admitted canonical Item.')

        # Creature.
        bestiary = m.get('Bestiary')
        creature = {
            'identity': ident(f'canary:creature/{s}'), 'display_name': name,
            'inspection': {'description': m['description']},
            'stats': {'max_health': m.get('maxHealth', 100), 'initial_health': m.get('health', 100),
                      'experience': m.get('experience', 0), 'speed': m.get('speed', 110),
                      'armor': defenses.get('armor', 0), 'defense': defenses.get('defense', 0),
                      'critical_chance_percent': flags.get('critChance', 0)},
            'resistances': [], 'immunities': {'damage_types': damage_immune, 'conditions': condition_immune},
            'flags': {'attackable': flags.get('attackable', True), 'illusionable': flags.get('illusionable', False),
                      'health_hidden': flags.get('healthHidden', False)},
            'summoning': {'summonable': flags.get('summonable', False), 'convinceable': flags.get('convinceable', False),
                          'is_familiar': flags.get('familiar', False)},
            'presentation': ref('Presentation', f'canary:presentation/{s}'),
            'behavior': ref('Behavior', f'canary:behavior/{s}'),
            'damage_reflection': [], 'healing_from_damage': [],
            'system_eligibility': {'prey': flags.get('isPreyable', True), 'exclusive_prey': flags.get('isPreyExclusive', False),
                                   'forge': flags.get('isForgeCreature', True), 'reward_boss': flags.get('rewardBoss', False)},
            'spawn_eligibility': {'period': PERIOD[m.get('respawnType', {}).get('period', '@RESPAWNPERIOD_ALL')[1:]],
                                  'ignore_period_underground': m.get('respawnType', {}).get('underground', False),
                                  'blocked_by_nearby_players': flags.get('isBlockable', False)},
        }
        article = m['description'].split(' ', 1)[0]
        if article in ('a', 'an') and m['description'][len(article) + 1:].lower() == name.lower():
            creature['name_forms'] = {'article': article}
        if 'mitigation' in defenses:
            creature['stats']['mitigation_percent'] = ratio(defenses['mitigation'])
        if creature['summoning']['summonable'] or creature['summoning']['convinceable']:
            creature['summoning']['mana_cost'] = m.get('manaCost', 0)
        if familiar:
            creature['summoning']['familiar'] = familiar
            row('flags.familiar/profile', 'mapped', destination='/monster/creature/summoning/familiar',
                resolution='Profile from pinned familiar spell and Player:CreateFamiliarSpell: '
                           'duration = 60 * config familiarTime / 2 seconds, using config.lua.dist baseline. '
                           'Summon Ability is a declared external reference; no Lua implementation is admitted.')
            row('familiar.owner_speed', 'unresolved_semantics',
                resolution='Player:createFamiliar changes speed by max(owner speed - familiar base speed, 0); '
                           'the optional fixed owner_speed_bonus cannot represent this state-dependent rule.')
        for element in m.get('elements', []):
            if element.get('percent'):
                creature['resistances'].append({'damage_type': DAMAGE[constant(element['type'], 'COMBAT_')],
                                                'reduction_percent': ratio(element['percent'])})
        for source, target in (('reflects', 'damage_reflection'), ('heals', 'healing_from_damage')):
            for element in m.get(source, []):
                creature[target].append({'damage_type': DAMAGE[constant(element['type'], 'COMBAT_')], 'percent': ratio(element['percent'])})
        if bestiary:
            stars = bestiary.get('Stars', 0)
            creature['bestiary'] = {'class': bestiary['class'], 'taxonomy': constant(bestiary['race'], 'BESTY_RACE_').lower(),
                                    'difficulty': DIFFICULTY[stars], 'occurrence': OCCURRENCE[bestiary.get('Occurrence', 0)],
                                    'stars': stars, 'kill_thresholds': [bestiary['FirstUnlock'], bestiary['SecondUnlock'], bestiary['toKill']],
                                    'charm_points': bestiary['CharmsPoints']}
            if bestiary.get('Locations', '').strip():
                creature['bestiary']['locations'] = bestiary['Locations']
            row('Bestiary', 'mapped', destination='/monster/creature/bestiary', line=line_of(r'^monster\.Bestiary'),
                resolution='difficulty is derived from Stars (0 harmless .. 5 challenging) and occurrence from Occurrence (0 common .. 3 very rare).')
        if m.get('bosstiary'):
            category = constant(m['bosstiary']['bossRace'], 'RARITY_').lower()
            levels = BOSSTIARY_LEVELS[category]
            creature['bosstiary'] = {'category': category,
                                    **dict(zip(('prowess_kills', 'expertise_kills', 'mastery_kills'), (v[0] for v in levels))),
                                    'boss_points': sum(v[1] for v in levels),
                                    'points_per_unlock': dict(zip(('prowess', 'expertise', 'mastery'), (v[1] for v in levels)))}
            row('bosstiary', 'mapped', destination='/monster/creature/bosstiary',
                resolution='Pinned src/io/io_bosstiary.hpp levelInfos gives kills and incremental points; '
                           'io_bosstiary.cpp addBosstiaryKill adds the points of the new level. boss_points is the full total.')
            row('bosstiary.bossRaceId', 'metadata_only', resolution='Foreign Bosstiary identifier; provenance only.')
        if corpse_id:
            creature['corpse_item'] = ref('Item', f'canary:item/{corpse_id}')
        fluid = RACE_RESIDUE.get(m.get('race', 'blood'))
        if fluid:
            creature['death_residue'] = {'item': ref('Item', f'canary:item/{SPLASH_ITEM}'), 'fluid_type': fluid}
            definitions.add(('Item', f'canary:item/{SPLASH_ITEM}'))
        if loot_entries:
            creature['loot'] = ref('Loot', f'canary:loot/{s}')

        # Behavior.
        target = m.get('strategiesTarget')
        behavior = {
            'identity': ident(f'canary:behavior/{s}'),
            'movement': {'can_walk': True, 'pass_through': False, 'pushable': flags.get('pushable', True),
                         'push_items': flags.get('canPushItems', False), 'push_creatures': flags.get('canPushCreatures', False),
                         'field_permissions': {'energy': flags.get('canWalkOnEnergy', True), 'fire': flags.get('canWalkOnFire', True),
                                               'poison': flags.get('canWalkOnPoison', True)}},
            'targeting': {'hostile': flags.get('hostile', True), 'can_target': True, 'sense_invisible': 'invisible' in condition_immune,
                          'target_distance_tiles': flags.get('targetDistance', 1),
                          'static_attack_chance_percent': flags.get('staticAttackChance', 95), 'flee_health': flags.get('runHealth', 0)},
            'attacks': schedules['attacks'], 'defenses': schedules['defenses'], 'event_bindings': []}
        if 'changeTarget' in m:
            behavior['targeting']['change_target'] = {'interval_ms': m['changeTarget']['interval'], 'chance_percent': m['changeTarget']['chance']}
        if target:
            behavior['targeting']['strategy_weights'] = {k: target.get(k, 0) for k in ('nearest', 'damage', 'health', 'random')}
        voices = m.get('voices')
        if voices and voices.get('_list'):
            behavior['voices'] = {'interval_ms': voices['interval'], 'chance_percent': voices['chance'],
                                  'entries': [{'text': v['text'], 'mode': 'yell' if v.get('yell') else 'say'} for v in voices['_list']]}
        summon = m.get('summon')
        if summon:
            entries = []
            for entry in summon.get('summons', []):
                key = f'canary:creature/{slug(entry["name"])}'
                if key != creature['identity']['key']:
                    definitions.add(('Creature', key))
                entries.append({'creature': ref('Creature', key), 'interval_ms': entry['interval'], 'chance_percent': entry['chance'],
                                'count': entry.get('count', 1)})
            behavior['summons'] = {'max_summons': summon['maxSummons'], 'entries': entries}
            row('summon', 'mapped', destination='/monster/behavior/summons', line=line_of(r'^monster\.summon'),
                resolution='Summoned creatures are declared source-scoped Creature references.')
        row('flags.pass_through', 'mapped', destination='/monster/behavior/movement/pass_through', line=line_of(r'^monster\.flags'),
            resolution='Oteryn-native movement field with no Canary counterpart. ' + PASS_THROUGH_DEFAULT)
        row('flags.canWalk/canTarget', 'mapped', destination='/monster/behavior/movement/can_walk', line=line_of(r'^monster\.flags'),
            resolution='Canary has no canWalk/canTarget flags (Crystal-only); the Canary engine always allows both, so true.')

        # Presentation.
        outfit = m.get('outfit', {})
        if outfit.get('lookTypeEx'):
            appearance_key = asset(f'canary.appearance:object/{outfit["lookTypeEx"]}')
        else:
            appearance_key = asset(f'canary.appearance:outfit/{outfit.get("lookType", base_look if familiar else 0)}')
        palette = [{'slot': slot, 'palette_binding': asset(f'canary.appearance:palette/{outfit[key]}')}
                   for slot, key in (('head', 'lookHead'), ('body', 'lookBody'), ('legs', 'lookLegs'), ('feet', 'lookFeet'))
                   if outfit.get(key)]
        light = m.get('light', {})
        presentation = {'identity': ident(f'canary:presentation/{s}'),
                        'appearance': {'asset_binding': appearance_key, 'palette_bindings': palette,
                                       'attachment_bindings': [], 'visual_effect_bindings': []},
                        'light': {'level': light.get('level', 0)}, 'audio': {'event_bindings': []}}
        if light.get('level', 0) > 0:
            presentation['light']['color_binding'] = asset(f'canary.appearance:light-color/{light.get("color", 0)}')
        if m.get('variant'):
            presentation['variant_label'] = m['variant']
        if outfit.get('lookAddons') or outfit.get('lookMount'):
            if outfit.get('lookAddons'):
                presentation['appearance']['attachment_bindings'].append({'slot': 'addon', 'asset_binding':
                    asset(f'canary.appearance:addon/{outfit.get("lookType", 0)}/{outfit["lookAddons"]}')})
            if outfit.get('lookMount'):
                presentation['appearance']['attachment_bindings'].append({'slot': 'mount', 'asset_binding':
                    asset(f'canary.appearance:outfit/{outfit["lookMount"]}')})
            row('outfit.lookAddons/lookMount', 'mapped', destination='/monster/presentation/appearance/attachment_bindings',
                line=line_of(r'^monster\.outfit'), resolution='Exact source addon bitset and mount lookType in '
                'declared source-scoped attachment bindings; no asset or runtime admission.')

        monster = {'creature': creature, 'behavior': behavior, 'presentation': presentation}
        if loot_entries:
            monster['loot'] = {'identity': ident(f'canary:loot/{s}'), 'algorithm': 'IndependentBernoulli', 'entries': loot_entries}

        # Remaining source rows.
        for field, destination, note in (
                ('description', '/monster/creature/inspection/description', 'Original source text, unchanged.'),
                ('health', '/monster/creature/stats/initial_health', None), ('maxHealth', '/monster/creature/stats/max_health', None),
                ('experience', '/monster/creature/stats/experience', None), ('speed', '/monster/creature/stats/speed', None),
                ('outfit', '/monster/presentation/appearance/asset_binding', 'Declared source appearance binding, not an admitted asset.'),
                ('light', '/monster/presentation/light/level', None),
                ('flags', '/monster/creature/flags', 'Absent flags take ' + RULES['monster_defaults'] + '.'),
                ('changeTarget', '/monster/behavior/targeting/change_target', None),
                ('strategiesTarget', '/monster/behavior/targeting/strategy_weights', None),
                ('voices', '/monster/behavior/voices', 'Original source text, unchanged.'),
                ('elements', '/monster/creature/resistances', 'Zero entries omitted. ' + RULES['elements'] + '.'),
                ('immunities', '/monster/creature/immunities', 'Invisible condition immunity also sets sense_invisible.'),
                ('defenses.armor', '/monster/creature/stats/armor', None), ('defenses.defense', '/monster/creature/stats/defense', None),
                ('defenses.mitigation', '/monster/creature/stats/mitigation_percent', 'Canary float percent points as an exact decimal ratio.'),
                ('manaCost', '/monster/creature/summoning/mana_cost', None)):
            key = field.split('.')[-1] if field.startswith('defenses.') else field
            present = (key in defenses) if field.startswith('defenses.') else (field in m)
            if not present:
                continue
            try:
                resolved = {'monster': monster}
                for part in destination.strip('/').split('/'):
                    resolved = resolved[part]
            except (KeyError, TypeError):
                continue
            row(field, 'mapped', destination=destination, resolution=note or 'Direct source value.',
                line=line_of(r'^\s*(monster\.)?' + re.escape(key) + r'\s*='))
        row('raceId', 'metadata_only', line=line_of(r'^monster\.raceId'), resolution='Foreign identifier; provenance only.') if 'raceId' in m else None
        if familiar:
            row('manaCost', 'metadata_only', resolution='The source summoning spell supplies familiar mana cost; '
                'this monster-level field is not used by summonable/convinceable=false.')
        if m.get('race', 'blood') not in RACE_RESIDUE:
            row('race', 'unresolved_dependency', 'dependency', line=line_of(r'^monster\.race\s*='),
                resolution=f'Unknown source race "{m["race"]}".')
        elif RACE_RESIDUE[m.get('race', 'blood')]:
            row('race', 'mapped', 'dependency', '/monster/creature/death_residue', line=line_of(r'^monster\.race\s*='),
                resolution=f'Race "{m.get("race", "blood")}": ' + RULES['race_residue'] + '.')
        else:
            row('race', 'approved_omission', 'dependency', line=line_of(r'^monster\.race\s*='),
                resolution=f'Race "{m["race"]}" leaves no death residue: ' + RULES['race_residue'] + '.')
        for event in m.get('events', []):
            # D6 covers quest/task counters only, not every creature event in later batches.
            known_quest_counter = event in {'RationalRequestRatDeath', 'TheFirstDragonDragonTaskDeath', 'TheGreatDragonHuntDeath'}
            row(f'events={event}', 'approved_omission' if known_quest_counter else 'unresolved_semantics',
                'script', line=line_of(r'^monster\.events'), resolution=(
                f'Registered creature event "{event}" updates quest/task progress on death. ' + QUEST_EVENT_OMISSION
                if known_quest_counter else f'Registered creature event "{event}" has not been qualified as quest/task-only; '
                'D6 does not authorize omission of this event.'))
        for callback in sorted(callbacks):
            row(f'mType.{callback}', 'unresolved_semantics', 'script', line=line_of(r'mType\.' + callback),
                resolution='Inline Lua callback; needs an explicit native behaviour resolution.')

        catalog = {'definitions': [ref(f, k) for f, k in sorted(definitions)], 'assets': sorted(assets)}
        manifest = {'sources': [{'repository': REPOSITORY, 'revision': REVISION}], 'entries': rows}
        source = {'file': source_file, 'git_blob': source_blob(self.canary, source_file)}
        return s, monster, deps, catalog, manifest, source

    def spell(self, spell, base, deps, asset):
        name = spell.get('name')
        effects = []
        formula_n = [0]

        def formula_range(low, high):
            formula_n[0] += 1
            key = base.format('formula') + f'-{formula_n[0]}'
            a, b = abs(low), abs(high)
            deps['formulas'].append({'identity': ident(key), 'kind': 'range', 'magnitude': {'minimum': min(a, b), 'maximum': max(a, b)}})
            return ref('Formula', key)

        def presentation():
            value = {}
            if spell.get('effect') not in (None, False):
                value['impact_asset_binding'] = asset('canary.appearance:effect/' + constant(spell['effect'], 'CONST_ME_').lower())
            shoot = spell.get('shootEffect') or spell.get('shooteffect')
            if shoot:
                value['projectile_asset_binding'] = asset('canary.appearance:missile/' + constant(shoot, 'CONST_ANI_').lower())
            return value

        def add_effect(suffix, body):
            key = base.format('effect') + suffix
            deps['effects'].append({'identity': ident(key), **body})
            effects.append(ref('Effect', key))

        note = ''
        if name == 'melee':
            if spell.get('attack') and spell.get('skill'):
                formula_n[0] += 1
                key = base.format('formula') + f'-{formula_n[0]}'
                deps['formulas'].append({'identity': ident(key), 'kind': 'melee_attack_skill',
                                         'melee': {'attack': spell['attack'], 'skill': spell['skill']}})
                formula = ref('Formula', key)
            else:
                formula = formula_range(spell.get('minDamage', 0), spell.get('maxDamage', 0))
            add_effect('', {'operation': 'damage', 'damage_type': 'physical', 'formula': formula, **({'presentation': presentation()} if presentation() else {})})
            geometry = {'needs_target': True, 'needs_direction': False}
            kind, range_tiles = 'melee', 1
        elif name in ('combat', *FIELD_ITEMS):
            if name == 'combat':
                damage = DAMAGE[constant(spell['type'], 'COMBAT_')]
                body = {'operation': 'heal' if damage == 'healing' else 'damage', 'damage_type': damage,
                        'formula': formula_range(spell.get('minDamage', 0), spell.get('maxDamage', 0))}
            else:
                self_field = FIELD_ITEMS[name]
                body = {'operation': 'create_item', 'created_item': ref('Item', f'canary:item/{self_field}')}
                self.pending_definitions.add(('Item', f'canary:item/{self_field}'))
            visual = presentation()
            area = spell.get('radius', 0) > 1 or spell.get('length') or spell.get('spread')
            if area and spell.get('effect') is None and 'field' not in name:
                visual['impact_asset_binding'] = asset('canary.appearance:effect/poff')
                note = RULES['area_effect'] + '. '
            if visual:
                body['presentation'] = visual
            add_effect('', body)
            geometry = cast_geometry(length=spell.get('length', 0), spread=spell.get('spread', 0),
                                     radius=spell.get('radius'), target=bool(spell.get('target', False)))
            kind, range_tiles = 'spell', spell.get('range', 0)
        elif name == 'condition':
            geometry = cast_geometry(length=spell.get('length', 0), spread=spell.get('spread', 0),
                                     radius=spell.get('radius'), target=bool(spell.get('target', False)))
            kind, range_tiles = 'spell', spell.get('range', 0)
        elif name in ('speed', 'outfit', 'invisible', 'drunk', 'effect'):
            geometry = cast_geometry(length=spell.get('length', 0), spread=spell.get('spread', 0),
                                     radius=spell.get('radius'), target=bool(spell.get('target', False)))
            kind, range_tiles = 'spell', spell.get('range', 0)
            duration = spell.get('duration') or 10000
            visual = presentation()
            if geometry.get('area') and spell.get('effect') is None:
                visual['impact_asset_binding'] = asset('canary.appearance:effect/poff')
                note += RULES['area_effect'] + '. '
            if name == 'speed':
                change = max(spell.get('speedChange', 0), -1000)
                multiplier = Fraction(1000 + change, 1000)
                formula_key = base.format('formula') + '-speed'
                deps['formulas'].append({'identity': ident(formula_key), 'kind': 'speed_modifier', 'speed': {
                    'minimum_multiplier': ratio(multiplier / 2), 'minimum_offset': 40,
                    'maximum_multiplier': ratio(multiplier), 'maximum_offset': 40}})
                body = {'operation': 'condition', 'duration_ms': duration,
                        'condition': {'type': 'haste' if change > 0 else 'paralyze', 'lifetime': 'fixed_duration',
                                      'speed_formula': ref('Formula', formula_key)}}
                note += 'monsters.cpp speed branch: speedChange clamped at -1000; '
                note += 'formula (1+change/1000)/2,40,(1+change/1000),40; default duration 10000 ms. '
            elif name == 'outfit':
                if spell.get('outfitMonster'):
                    transform = {'creature': ref('Creature', f'canary:creature/{slug(spell["outfitMonster"])}')}
                    self.pending_definitions.add(('Creature', transform['creature']['key']))
                elif spell.get('outfitItem', 0) > 0:
                    transform = {'item': ref('Item', f'canary:item/{spell["outfitItem"]}')}
                    self.pending_definitions.add(('Item', transform['item']['key']))
                else:
                    return None
                body = {'operation': 'appearance_transform', 'duration_ms': duration, 'appearance_transform': transform}
                note += 'monsters.cpp outfit branch: lazy Creature appearance or Item appearance, duration default 10000 ms. '
            elif name in ('invisible', 'drunk'):
                body = {'operation': 'condition', 'duration_ms': duration,
                        'condition': {'type': name, 'lifetime': 'fixed_duration'}}
                note += f'monsters.cpp {name} branch, duration default 10000 ms. '
            else:
                if not visual:
                    return None
                body = {'operation': 'presentation_only'}
                note += 'monsters.cpp effect branch adds no damage/condition; only its presentation parameters. '
            if visual:
                body['presentation'] = visual
            add_effect('', body)
        else:
            return None
        condition_type = None
        if name == 'condition':
            condition_type = constant(spell['type'], 'CONDITION_').lower()
            low, high, start, tick = spell.get('minDamage', 0), spell.get('maxDamage', 0), spell.get('startDamage', 0), 2000
        elif isinstance(spell.get('condition'), dict):
            c = spell['condition']
            condition_type = constant(c['type'], 'CONDITION_').lower()
            low = high = c.get('totalDamage', 0)
            start, tick = 0, c.get('interval', 2000)
        if condition_type:
            low, high, start = abs(low), abs(high), abs(start)
            high = high or low
            start = 0 if start > low else start
            add_effect('-condition', {'operation': 'condition', 'condition': {'type': condition_type, 'lifetime': 'damage_schedule',
                       'damage_over_time': {'total_damage_range': {'minimum': min(low, high), 'maximum': max(low, high)},
                                            'tick_interval_ms': tick,
                                            'initial_tick': {'mode': 'fixed', 'amount': start} if start else {'mode': 'automatic'},
                                            'tick_profile': 'decreasing', 'first_tick': 'after_interval'}}})
            note += RULES['condition'] + '. '
        ability_key = base.format('ability')
        ability = {'identity': ident(ability_key), 'kind': kind, 'range_tiles': range_tiles, **geometry, 'effects': effects}
        deps['abilities'].append(ability)
        return ability_key, note

    def item_payload(self, item_id, asset):
        record = self.items.get(item_id, {'name': None, 'article': None, 'attributes': {}})
        flags = self.objects.get(item_id, {}).get('flags', {})
        attributes = record['attributes']
        name = record['name'] or self.names.get(item_id) or f'item {item_id}'
        payload = {'identity': ident(f'canary:item/{item_id}'),
                   'presentation': {'name': name, 'asset_binding': asset(f'canary.appearance:object/{item_id}')},
                   'classification': {'is_corpse': bool(flags.get('corpse'))},
                   'physical': {'weight_centioz': int(attributes.get('weight', 0)), 'movable': not flags.get('unmove', False),
                                'pickupable': bool(flags.get('take'))},
                   'collision': {'block_walk': bool(flags.get('unpass')), 'block_projectiles': bool(flags.get('unsight')),
                                 'block_pathfinding': bool(flags.get('avoid'))},
                   'temporal': {'decay_action': 'none', 'stop_duration': False}}
        if record['article']:
            payload['presentation']['article'] = record['article']
        if flags.get('container') and attributes.get('containersize'):
            payload['container'] = {'capacity': int(attributes['containersize'])}
        next_id = None
        if 'decayto' in attributes and 'duration' in attributes:
            next_id = int(attributes['decayto'])
            payload['temporal']['duration_ms'] = int(attributes['duration']) * 1000
            if next_id:
                payload['temporal'].update(decay_action='transform', decay_target=ref('Item', f'canary:item/{next_id}'))
            else:
                payload['temporal']['decay_action'] = 'remove'
        return payload, next_id, None


def main():
    parser = argparse.ArgumentParser(description=__doc__.split('\n')[0])
    parser.add_argument('--canary', required=True, type=Path)
    parser.add_argument('--batch', choices=('first', 'second'), default='first')
    parser.add_argument('--out', type=Path)
    args = parser.parse_args()
    actual = subprocess.check_output(['git', '-C', str(args.canary), 'rev-parse', 'HEAD'], text=True).strip()
    if actual != REVISION:
        parser.error(f'Canary HEAD must be {REVISION}, got {actual}')
    if subprocess.check_output(['git', '-C', str(args.canary), 'status', '--porcelain', '--untracked-files=no'], text=True).strip():
        parser.error('Canary tracked source files must be unchanged')
    args.out = args.out or ROOT / 'samples' / (REV if args.batch == 'first' else REV + '-batch-2')
    objects = load_appearance_objects(args.canary / 'data/items/appearances.dat')
    items = load_items_xml(args.canary / 'data/items/items.xml')
    names, index = name_index(objects, items)
    converter = Converter(args.canary, objects, items, names, index)
    sources = []
    for relative in BATCH if args.batch == 'first' else SECOND_BATCH:
        converter.pending_definitions = set()
        s, monster, deps, catalog, manifest, source = converter.convert(relative)
        for family, key in sorted(converter.pending_definitions):
            catalog['definitions'].append(ref(family, key))
        target = args.out / s
        target.mkdir(parents=True, exist_ok=True)
        for filename, value in (('monster.json', monster), ('dependencies.json', deps), ('catalog.json', catalog), ('manifest.json', manifest)):
            (target / filename).write_text(json.dumps(value, ensure_ascii=False, indent=2) + '\n', encoding='utf-8', newline='\n')
        sources.append({'monster': s, **source})
    shared = {'repository': REPOSITORY, 'revision': REVISION, 'rules': RULES,
              'shared_sources': {p: source_blob(args.canary, p) for p in
                                 ('data/items/items.xml', 'data/items/appearances.dat', 'data/scripts/lib/register_monster_type.lua')},
              'monsters': sources}
    if args.batch == 'second':
        additional = ['src/creatures/monsters/monsters.cpp', 'src/io/io_bosstiary.hpp', 'src/io/io_bosstiary.cpp',
                      'config.lua.dist', 'data/libs/functions/player.lua', 'data/libs/systems/familiar.lua',
                      'data/XML/familiars.xml', 'data/scripts/spells/familiar/paladin_familiar.lua']
        shared['shared_sources'].update({p: source_blob(args.canary, p) for p in additional})
        paths = sorted((args.canary / MONSTER_DIR).rglob('*.lua'))
        tree = subprocess.check_output(['git','-C',str(args.canary),'ls-tree','-r',REVISION,'--',MONSTER_DIR],text=True)
        blobs = {line.split('\t',1)[1]: line.split('\t',1)[0].split()[2] for line in tree.splitlines()}
        shared['literal_source_census'] = {'lua_files': len(paths),
            'path_blob_sha256': hashlib.sha256('\n'.join(p.relative_to(args.canary).as_posix() + ':' + blobs[p.relative_to(args.canary).as_posix()]
                                                       for p in paths).encode()).hexdigest(),
            'child_assignment_matches': sum(bool(re.search(r'\bchild\s*=', p.read_text(encoding='utf-8'))) for p in paths),
            'combat_true_matches': sum(bool(re.search(r'\bcombat\s*=\s*true\b', p.read_text(encoding='utf-8'))) for p in paths),
            'scope': 'Literal source census only, not arbitrary Lua or engine behavior completeness.'}
    (args.out / 'sources.json').write_text(json.dumps(shared, ensure_ascii=False, indent=2) + '\n', encoding='utf-8', newline='\n')
    print(json.dumps({'monsters': len(sources), 'out': str(args.out)}))


if __name__ == '__main__':
    main()
