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
import xml.etree.ElementTree as ET
from decimal import ROUND_HALF_EVEN, Decimal
from fractions import Fraction
from pathlib import Path

from normalize_monster_fields import cast_geometry
import spell_probes
import spell_scripts

ROOT = Path(__file__).resolve().parent
REPOSITORY = 'opentibiabr/canary'
REVISION = '47dfd51f45280a59a1d3e50ba7edd573d7234446'
REV = 'canary-47dfd51f'
MONSTER_DIR = 'data-otservbr-global/monster'
EFFECT_CONSTANTS = 'src/utils/utils_definitions.hpp'
CASTER_MAGNITUDE = 'canary:formula/caster-magnitude'


class SpellUnresolved(Exception):
    pass
BATCH_1 = ['mammals/rat', 'giants/cyclops', 'humanoids/orc_spearman', 'vermins/scorpion', 'humanoids/orc_shaman',
         'humans/necromancer', 'elementals/fire_elemental', 'dragons/dragon', 'undeads/ghost',
         'quests/killing_in_the_name_of/demodras']
BATCH_2 = ['bosses/morshabaal', 'humanoids/dworc_voodoomaster', 'fey/wisp',
           'quests/forgotten_knowledge/bosses/the_enraged_thorn_knight', 'quests/cults_of_tibia/bosses/summons/sand_vortex',
           'familiars/knight_familiar', 'humans/blood_hand', 'bosses/mad_mage', 'humanoids/crazed_summer_rearguard',
           'constructs/war_golem']
BATCHES = {REV: BATCH_1, REV + '-batch-2': BATCH_2}
RULES = {
    'armor_melee': 'src/creatures/monsters/monsters.cpp deserializeSpell sets COMBAT_PARAM_BLOCKARMOR and COMBAT_PARAM_BLOCKSHIELD '
                   'on a monster melee, so its damage is reduced by the target armor and shield defense (mitigated_by)',
    'armor_physical': 'src/creatures/monsters/monsters.cpp deserializeSpell sets COMBAT_PARAM_BLOCKARMOR on an inline physical '
                      'combat, so its damage is reduced by the target armor (mitigated_by)',
    'armor_script': 'src/creatures/combat/combat.cpp Combat::setParam maps COMBAT_PARAM_BLOCKARMOR/BLOCKSHIELD to '
                    'blockedByArmor/blockedByShield, which Creature::blockHit applies (mitigated_by)',
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
    'speed': 'monsters.cpp deserializeSpell speed: speedChange < -1000 clamps to -1000; >0 haste (non-aggressive), else paralyze; '
             'default duration 10000 ms; multiplier = 1 + speedChange/1000, formula vars (multiplier/2, 40, multiplier, 40)',
    'fixed_conditions': 'monsters.cpp deserializeSpell outfit/invisible/drunk: default duration 10000 ms; outfit/invisible non-aggressive',
    'familiar_look': 'data/libs/systems/familiar.lua FAMILIAR_ID gives the default look per vocation, which '
                     'creaturescripts/familiar/on_login.lua assigns to a character without a selection',
    'nil_constant': 'an undefined Lua global evaluates to nil in Canary',
    'primal_pack_beast': 'data-otservbr-global/lib/quests/the_primal_ordeal.lua RegisterPrimalPackBeast registers a separate '
                         'type "<name> (Primal)" (named Primal Pack Beast, 0 experience, no loot, 70% health, no Bestiary, no '
                         'corpse) and leaves this type unchanged; the derived Primal type is not generated here',
    'nil_zero': 'src/lua/functions/lua_functions_loader.hpp getNumber reads nil with lua_tonumber as 0',
    'dispel': 'src/creatures/combat/combat.cpp CombatDispelFunc removes every condition of COMBAT_PARAM_DISPEL type from '
              'each target after the health change (or on a combat without damage)',
    'element_over_100': 'monster.cpp blockHit sets damage <= 0 to 0, so more than 100% reduction equals 100% '
                        '(the excess only offsets the Wheel "Ballistic Mastery" element reduction)',
    'zero_condition': 'condition.cpp ConditionDamage::init: a damage condition with zero total damage never starts',
    'chance_clamp': 'monsters.cpp deserializeSpell sb.chance = min(chance, 100); monster.cpp summons use chance >= uniform(1, 100)',
    'inert_spells': 'monsters.cpp deserializeSpell strength/effect branches add no combat payload; only effect/shoot visuals remain',
    'bosstiary': 'io_bosstiary.hpp levelInfos kills/points per stage: bane 25/100/300 & 5/15/30, archfoe 5/20/60 & 10/30/60, '
                 'nemesis 1/3/5 & 10/30/60',
    'familiar': 'data/scripts/spells/familiar/<vocation>_familiar.lua (vocation, mana) and data/libs/functions/player.lua '
                'CreateFamiliarSpell: duration = 60 * familiarTime / 2 s with config default familiarTime=30 -> 900000 ms',
    'race_residue': 'creature.cpp dropCorpse (identical in Crystal be61cdd/ac447fef): venom/blood/ink/chocolate/candy create '
                    'ITEM_FULLSPLASH=2886 with that fluid; undead/fire/energy/none create nothing; summons drop no corpse',
}
# Owner decisions 2026-09-26 (docs/architecture/OTERYN_MONSTER_AUTHORING_SCHEMA_V1.md section 3).
WIKI_API = 'https://tibia.fandom.com/api.php'
WIKI_REFERENCE = 'wiki-2026-07-28.json'
BEHAVIOUR_PATTERNS = 'p4-behaviour-patterns-canary-47dfd51f.json'
PROBED_PATTERNS = ('conditional_summon', 'heal_allies_in_area', 'remove_magic_walls', 'path_trail_missile')
PATH_TRAIL = (r'local target = Creature\(var\.number\) if not target then return false end local creaturePos = creature:getPosition\(\) '
              r'local path = creaturePos:getPathTo\(target:getPosition\(\), 0, 0, true, (true|false), (\d+)\) if not path or #path == 0 '
              r'then return false end for i = 1, #path do creaturePos:getNextPosition\(path\[i\], 1\) '
              r'creaturePos:sendMagicEffect\((CONST_ME_\w+)\) end return combat:execute\(creature, var\)')
WIKI_ADOPTION = ('Owner decision D15: where the reference-date (2026-07-28) wiki differs from Canary, the wiki value replaces '
                 'it; applied to health, experience, armor, mitigation, element modifiers, flags, flee health, Bestiary '
                 'difficulty/occurrence (and the Bestiary class when Canary names no valid race), loot items missing in Canary and loot probabilities; never to an uncertain '
                 '(? or ~) or unparsed wiki value')
LOW_CONFIDENCE_DROPS = 10
LOOT_RATE_RULE = ('D15 loot rate: highest-version Loot Statistics block at the cut, estimate = drops / kills; '
                  'adopted at >= 10 drops, otherwise the Canary probability is kept as low confidence')
PASS_THROUGH_DEFAULT = 'Owner decision D5: imported monsters without a source counterpart default to pass_through=false.'
QUEST_EVENT_OMISSION = ('Owner decision D6: creature event scripts that only feed quest/task progress belong to Quest/Interaction '
                        'and are omitted from the monster bundle.')
# Creature events verified by reading their registering script at REVISION. Unlisted events stay unresolved.
EVENTS = {
    'RationalRequestRatDeath': ('quest', 'data-otservbr-global/scripts/quests/the_rookie_guard/mission03_rational_request.lua',
                                'increments the Rookie Guard mission 3 rat counter'),
    'TheFirstDragonDragonTaskDeath': ('quest', 'data-otservbr-global/scripts/quests/the_first_dragon/creaturescripts_kill_dragon.lua',
                                      'increments The First Dragon dragon counter'),
    'TheGreatDragonHuntDeath': ('quest', 'data-otservbr-global/scripts/quests/the_great_dragon_hunt_quest/creaturescripts_the_great_dragon_hunt.lua',
                                'increments The Great Dragon Hunt counter inside fixed areas'),
    'ForgottenKnowledgeBossDeath': ('encounter_bookkeeping', 'data-otservbr-global/scripts/quests/forgotten_knowledge/creaturescripts_bosses_kill.lua',
                                    'sets the per-player boss cooldown storage on death'),
    'GlowingRubbishAmuletDeath': ('quest', 'data-otservbr-global/scripts/quests/cults_of_tibia/creaturescripts_glowing_rubbish_amulet.lua',
                                  'advances the Misguided mission kill and exorcism counters of the killer\'s party and, at 10 kills '
                                  'while the glowing rubbish amulet 25296 is worn, replaces it by 25297; quest progress and a quest '
                                  'item only, no fight or map effect'),
    'HealthForgotten': ('encounter_mechanic', 'data-otservbr-global/scripts/quests/forgotten_knowledge/creaturescripts_healthchange_forgotten.lua',
                        'doubles damage taken unless a Possessed Tree is within 7 tiles'),
}
EVENT_CENSUS = ROOT / 'samples' / 'events-canary-47dfd51f.json'
ENCOUNTER_SAMPLES = ROOT.parent / 'encounter-authoring' / 'samples'
ENCOUNTERS = {}  # (event, creature key) -> encounters whose manifests cover that pair
for _manifest in sorted(ENCOUNTER_SAMPLES.glob('*/manifest.json')):
    _data = json.loads(_manifest.read_text(encoding='utf-8'))
    for _event, _creatures in _data['covers'].items():
        for _creature in _creatures:
            ENCOUNTERS.setdefault((_event, _creature), []).append(
                f'{_data["encounter"]} ({_manifest.parent.relative_to(ROOT.parent)})')
if EVENT_CENSUS.exists():
    for _event in json.loads(EVENT_CENSUS.read_text(encoding='utf-8'))['events']:
        if _event['event'] not in EVENTS:
            EVENTS[_event['event']] = (_event['kind'] if _event['confidence'] == 'high' else 'low_confidence',
                                       _event['script'] or '(no registering script)', _event['effect'].rstrip('.'))
ENCOUNTER_OMISSION = 'Owner decision D9: boss encounter bookkeeping belongs to the Encounter definition, not the monster bundle.'
RACE_RESIDUE = {'venom': 'slime', 'blood': 'blood', 'ink': 'ink', 'chocolate': 'chocolate', 'candy': 'candy',
                'undead': None, 'fire': None, 'energy': None}
SPLASH_ITEM = 2886
BOSSTIARY = {'RARITY_BANE': ('bane', (25, 100, 300), (5, 15, 30)), 'RARITY_ARCHFOE': ('archfoe', (5, 20, 60), (10, 30, 60)),
             'RARITY_NEMESIS': ('nemesis', (1, 3, 5), (10, 30, 60))}
FAMILIAR_DEFAULT_LOOK = {'sorcerer': 994, 'druid': 993, 'paladin': 992, 'knight': 991, 'monk': 1818}
FAMILIAR_DURATION_MS = 60 * 30 // 2 * 1000
FAMILIAR_SPELLS = {'knight': ('knight', 1000), 'druid': ('druid', 1000), 'paladin': ('paladin', 1000),
                   'sorcerer': ('sorcerer', 1000), 'monk': ('monk', 1000)}
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


def load_monster(path, errors=None):
    """Evaluate one monster file. With `errors` (a list), a Lua error raised after `mType:register`
    is appended to it instead of raised, because Canary keeps a monster type that registered."""
    from lupa.luajit21 import LuaRuntime
    lua = LuaRuntime(unpack_returned_tuples=True)
    registered, callbacks = lua.execute(LUA_PRELUDE)
    try:
        lua.execute(path.read_text(encoding='utf-8'))
    except Exception as exc:
        if errors is None or registered['monster'] is None:
            raise
        errors.append(str(exc).splitlines()[0][:160])
    return registered['name'], lua_value(registered['monster']), dict(callbacks.items())


def load_effect_constants(path):
    """MagicEffectClasses and ShootType_t values from Canary utils_definitions.hpp."""
    text = path.read_text(encoding='utf-8')
    tables = {}
    for enum in ('MagicEffectClasses', 'ShootType_t'):
        body = re.search(r'enum ' + enum + r'[^{]*\{(.*?)\};', text, re.S).group(1)
        values, current = {}, -1
        for line in body.splitlines():
            line = line.split('//')[0].strip().rstrip(',')
            match = re.match(r'(\w+)\s*(?:=\s*(\w+))?$', line)
            if not match:
                continue
            name, expression = match.groups()
            current = current + 1 if expression is None else (values[expression] if expression in values else int(expression, 0))
            values[name] = current
        tables[enum] = values
    return tables['MagicEffectClasses'], tables['ShootType_t']


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
    fraction = Fraction(Decimal(str(value)))
    return {'numerator': fraction.numerator, 'denominator': fraction.denominator}


def fraction_ratio(fraction):
    return {'numerator': fraction.numerator, 'denominator': fraction.denominator}


def percent_from_chance(chance):
    value = Decimal(min(int(chance), 100000)) / 1000
    return int(value) if value == value.to_integral_value() else float(value)


class Converter:
    def __init__(self, canary, objects, items, names, index):
        self.canary, self.objects, self.items, self.names, self.index = canary, objects, items, names, index
        self.wiki = {}
        self.spell_scripts = None
        self.magic_effects, self.missiles = load_effect_constants(canary / EFFECT_CONSTANTS)
        self.magic_effect_names = {v: k for k, v in self.magic_effects.items()}
        self.missile_names = {v: k for k, v in self.missiles.items()}

    def visual(self, value, kind):
        """Asset key and note for a spell `effect` (kind 'effect') or `shootEffect` (kind 'missile') value as the
        engine uses it: the value is a number, so a constant of the other enum or a raw id selects that numeric id."""
        own, other = (('CONST_ME_', 'CONST_ANI_') if kind == 'effect' else ('CONST_ANI_', 'CONST_ME_'))
        table, names = ((self.magic_effects, self.magic_effect_names) if kind == 'effect' else (self.missiles, self.missile_names))
        if isinstance(value, str) and value.startswith('@' + own):
            return f'canary.appearance:{kind}/' + value[len(own) + 1:].lower(), ''
        if isinstance(value, str) and value.startswith('@' + other):
            other_table = self.missiles if kind == 'effect' else self.magic_effects
            number = other_table[value[1:]]
            note = f'Source puts {value[1:]} ({number}) in the {kind} field; the engine sends it as {kind} id {number}. '
        elif isinstance(value, (int, float)):
            number, note = int(value), ''
        else:
            raise ValueError(f'unsupported {kind} value {value!r}')
        name = names.get(number)
        key = name[len(own):].lower() if name else f'id-{number}'
        return f'canary.appearance:{kind}/{key}', note

    def convert(self, relative):
        path = self.canary / MONSTER_DIR / (relative + '.lua')
        text = path.read_text(encoding='utf-8')
        lines = text.splitlines()
        late_errors = []
        name, m, callbacks = load_monster(path, late_errors)
        s = slug(name)
        self.current_slug = s
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

        for error in late_errors:
            if 'RegisterPrimalPackBeast' in error:
                row('RegisterPrimalPackBeast(monster)', 'approved_omission', 'script', line=line_of(r'^RegisterPrimalPackBeast'),
                    resolution=RULES['primal_pack_beast'] + '.')
                continue
            row('top-level script after mType:register', 'unresolved_semantics', 'script', line=len(lines),
                resolution='A top-level Lua call after registration failed here; Canary keeps the registered type and runs '
                           f'the call at load. Needs a native decision: {error}')

        flags = m.get('flags', {})
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
                if self.spell_scripts is None:
                    self.spell_scripts = spell_scripts.SpellScripts(self.canary)
                if str(spell.get('name', '')).lower() in self.spell_scripts.index:
                    result = self.registered_spell(spell, deps, asset)
                else:
                    result = self.spell(spell, base, deps, asset)
                    if result is None:
                        result = self.registered_spell(spell, deps, asset)
                if result and result[0] == 'UNRESOLVED':
                    row(f'{group}[{n}]', 'unresolved_semantics', line=line, resolution=result[1])
                    continue
                if result and result[0] == 'OMIT':
                    row(f'{group}[{n}]', 'approved_omission', line=line, resolution=result[1] + ' No visual either, so the entry has no effect.')
                    continue
                if result is None:
                    row(f'{group}[{n}].name={spell.get("name")}', 'unresolved_semantics', 'script', resolution=
                        'Spell name is not an inline Canary branch; it resolves to a registered spell script that needs a native behaviour decision.', line=line)
                    continue
                ability_key, note = result[:2]
                extras = result[2] if len(result) > 2 else {}
                chance = 100 if spell.get('name') == 'melee' else spell.get('chance', 100)
                if chance > 100:
                    note += f'Chance {chance} clamped to 100: ' + RULES['chance_clamp'] + '. '
                    chance = 100
                schedules[group].append({'ability': ref('Ability', ability_key), 'interval_ms': spell.get('interval', 2000),
                                         'chance_percent': chance, **extras})
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
            'inspection': {'description': m.get('description', name)},
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
        description = m.get('description', name)
        article = description.split(' ', 1)[0]
        if article in ('a', 'an') and description[len(article) + 1:].lower() == name.lower():
            creature['name_forms'] = {'article': article}
        if 'mitigation' in defenses:
            creature['stats']['mitigation_percent'] = ratio(defenses['mitigation'])
        if creature['summoning']['summonable'] or creature['summoning']['convinceable']:
            creature['summoning']['mana_cost'] = m.get('manaCost', 0)
        for element in m.get('elements', []):
            kind = element.get('type')
            if isinstance(kind, str) and kind.startswith('@COMBAT_') and kind[8:] not in DAMAGE:
                row(f'elements.type={kind[1:]}', 'approved_omission', line=line_of(re.escape(kind[1:])),
                    resolution=RULES['nil_constant'] + '; registerMonsterType.elements skips an entry without type.')
                continue
            if kind is None or not element.get('percent'):
                continue
            percent = element['percent']
            if percent > 100:
                row(f'elements.{kind[1:]}.percent', 'mapped', destination='/monster/creature/resistances', line=line_of(re.escape(kind[1:])),
                    resolution=f'{percent}% stored as 100%: ' + RULES['element_over_100'] + '.')
                percent = 100
            creature['resistances'].append({'damage_type': DAMAGE[constant(kind, 'COMBAT_')], 'reduction_percent': ratio(percent)})
        for source, target in (('reflects', 'damage_reflection'), ('heals', 'healing_from_damage')):
            for element in m.get(source, []):
                creature[target].append({'damage_type': DAMAGE[constant(element['type'], 'COMBAT_')], 'percent': ratio(element['percent'])})
        race = bestiary.get('race') if bestiary else None
        self.pending_bestiary = None
        if bestiary and not (isinstance(race, str) and race.startswith('@BESTY_RACE_')):
            self.pending_bestiary = (bestiary, len(rows))
            row('Bestiary.race', 'unresolved_dependency', 'dependency', line=line_of(r'^monster\.Bestiary'),
                resolution=f'Bestiary without a valid race ({race!r}); Canary leaves the Bestiary race unset, so the entry '
                           'has no Bestiary class page. Bestiary omitted until a taxonomy is chosen.')
            bestiary = None
        if bestiary:
            creature['bestiary'] = self.bestiary_payload(bestiary, constant(bestiary['race'], 'BESTY_RACE_').lower())
            row('Bestiary', 'mapped', destination='/monster/creature/bestiary', line=line_of(r'^monster\.Bestiary'),
                resolution='difficulty is derived from Stars (0 harmless .. 5 challenging) and occurrence from Occurrence (0 common .. 3 very rare).')
        if 'bosstiary' in m:
            category, kills, points = BOSSTIARY[m['bosstiary']['bossRace'][1:]]
            creature['bosstiary'] = {'category': category, 'prowess_kills': kills[0], 'expertise_kills': kills[1], 'mastery_kills': kills[2],
                                     'prowess_points': points[0], 'expertise_points': points[1], 'mastery_points': points[2]}
            row('bosstiary.bossRace', 'mapped', destination='/monster/creature/bosstiary', line=line_of(r'bossRace\s*='),
                resolution='Stage kills and points are derived from the rarity: ' + RULES['bosstiary'] + '.')
            row('bosstiary.bossRaceId', 'metadata_only', line=line_of(r'bossRaceId'), resolution='Foreign identifier; provenance only.')
        if creature['summoning']['is_familiar']:
            vocation = slug(name).split('_')[0]
            if vocation in FAMILIAR_SPELLS:
                voc, mana = FAMILIAR_SPELLS[vocation]
                spell_key = f'canary:ability/spell/summon_{voc}_familiar'
                definitions.add(('Ability', spell_key))
                creature['summoning']['familiar'] = {'vocation': voc, 'summon_ability': ref('Ability', spell_key),
                                                     'duration_ms': FAMILIAR_DURATION_MS, 'mana_cost': mana}
                row('flags.familiar', 'mapped', destination='/monster/creature/summoning/familiar', line=line_of(r'familiar\s*=\s*true'),
                    resolution='The monster file only flags the familiar; the profile comes from ' + RULES['familiar'] + '.')
            else:
                row('flags.familiar', 'unresolved_dependency', 'dependency', line=line_of(r'familiar\s*=\s*true'),
                    resolution='No familiar summon spell found for this monster name.')
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
                          'static_attack_chance_percent': flags.get('staticAttackChance', 95),
                          'flee_health': min(flags.get('runHealth', 0), creature['stats']['max_health'])},
            'attacks': schedules['attacks'], 'defenses': schedules['defenses'], 'event_bindings': []}
        if flags.get('runHealth', 0) > creature['stats']['max_health']:
            row('flags.runHealth', 'mapped', destination='/monster/behavior/targeting/flee_health', line=line_of(r'runHealth'),
                resolution=f'runHealth {flags["runHealth"]} exceeds maxHealth; stored as max_health, which behaves the same '
                           '(monster.cpp flees while health <= runAwayHealth).')
        if 'changeTarget' in m:
            behavior['targeting']['change_target'] = {'interval_ms': m['changeTarget']['interval'], 'chance_percent': m['changeTarget']['chance']}
        if target:
            behavior['targeting']['strategy_weights'] = {k: target.get(k, 0) for k in ('nearest', 'damage', 'health', 'random')}
        voices = m.get('voices')
        if voices and voices.get('_list'):
            entries = [{'text': v['text'], 'mode': 'yell' if v.get('yell') else 'say'} for v in voices['_list'] if v.get('text')]
            if len(entries) < len(voices['_list']):
                row('voices.text', 'approved_omission', line=line_of(r'^monster\.voices'),
                    resolution='Voice entries with empty text are dropped; they would only send an empty line.')
            if entries:
                behavior['voices'] = {'interval_ms': voices['interval'], 'chance_percent': voices['chance'], 'entries': entries}
        if voices and not voices.get('_list'):
            row('voices', 'approved_omission', line=line_of(r'^monster\.voices'),
                resolution='Interval/chance without any voice entry; the engine has nothing to say, so no voices section.')
        summon = m.get('summon')
        if summon:
            entries, notes = [], []
            for entry in summon.get('summons', []):
                key = f'canary:creature/{slug(entry["name"])}'
                if key != creature['identity']['key']:
                    definitions.add(('Creature', key))
                count = entry.get('count', 1)
                if not isinstance(count, int):
                    notes.append(f'count {count} is an undefined Lua global (nil), so addSummon uses its default 1')
                    count = 1
                if count > summon['maxSummons']:
                    notes.append(f'count {count} clamped to maxSummons {summon["maxSummons"]}, which caps it in monster.cpp anyway')
                    count = summon['maxSummons']
                chance = entry['chance']
                if chance > 100:
                    notes.append(f'chance {chance} clamped to 100 ({RULES["chance_clamp"]})')
                    chance = 100
                entries.append({'creature': ref('Creature', key), 'interval_ms': entry['interval'], 'chance_percent': chance, 'count': count})
            if entries:
                behavior['summons'] = {'max_summons': summon['maxSummons'], 'entries': entries}
                row('summon', 'mapped', destination='/monster/behavior/summons', line=line_of(r'^monster\.summon'),
                    resolution='Summoned creatures are declared source-scoped Creature references.' + ''.join(f' {n}.' for n in notes))
            else:
                row('summon', 'approved_omission', line=line_of(r'^monster\.summon'),
                    resolution='maxSummons without any summon entry; the engine summons nothing.')
        row('flags.pass_through', 'mapped', destination='/monster/behavior/movement/pass_through', line=line_of(r'^monster\.flags'),
            resolution='Oteryn-native movement field with no Canary counterpart. ' + PASS_THROUGH_DEFAULT)
        row('flags.canWalk/canTarget', 'mapped', destination='/monster/behavior/movement/can_walk', line=line_of(r'^monster\.flags'),
            resolution='Canary has no canWalk/canTarget flags (Crystal-only); the Canary engine always allows both, so true.')

        # Presentation.
        outfit = m.get('outfit', {})
        if outfit.get('lookTypeEx'):
            appearance_key = asset(f'canary.appearance:object/{outfit["lookTypeEx"]}')
        elif not outfit.get('lookType') and flags.get('familiar') and slug(name).split('_')[0] in FAMILIAR_DEFAULT_LOOK:
            appearance_key = asset(f'canary.appearance:outfit/{FAMILIAR_DEFAULT_LOOK[slug(name).split("_")[0]]}')
            row('outfit.lookType', 'mapped', destination='/monster/presentation/appearance', line=line_of(r'^monster\.outfit'),
                resolution='Owner decision D16: a familiar shows the look its owner selected (data/XML/familiars.xml, chosen per '
                           'character); ' + RULES['familiar_look'] + '. The monster file has no lookType.')
        elif not outfit.get('lookType'):
            appearance_key = None
            row('outfit.lookType', 'mapped', destination='/monster/presentation/appearance/selection', line=line_of(r'^monster\.outfit'),
                resolution='lookType 0 without lookTypeEx and not a familiar: the creature has no appearance (owner decision D24).')
        else:
            appearance_key = asset(f'canary.appearance:outfit/{outfit.get("lookType", 0)}')
        palette = [{'slot': slot, 'palette_binding': asset(f'canary.appearance:palette/{outfit[key]}')}
                   for slot, key in (('head', 'lookHead'), ('body', 'lookBody'), ('legs', 'lookLegs'), ('feet', 'lookFeet'))
                   if outfit.get(key)]
        light = m.get('light', {})
        presentation = {'identity': ident(f'canary:presentation/{s}'),
                        'appearance': {**({'asset_binding': appearance_key} if appearance_key else {'selection': 'invisible'}),
                                       'palette_bindings': palette if appearance_key else [],
                                       'attachment_bindings': [], 'visual_effect_bindings': []},
                        'light': {'level': light.get('level', 0)}, 'audio': {'event_bindings': []}}
        if light.get('level', 0) > 0:
            presentation['light']['color_binding'] = asset(f'canary.appearance:light-color/{light.get("color", 0)}')
        if m.get('variant'):
            presentation['variant_label'] = m['variant']
        if flags.get('familiar') and 'lookType' not in outfit and not outfit.get('lookTypeEx'):
            presentation['appearance']['selection'] = 'owner_familiar_look'
        addons = outfit.get('lookAddons', 0)
        if addons:
            for bit in (1, 2):
                if addons & bit:
                    presentation['appearance']['attachment_bindings'].append(
                        {'slot': 'addon', 'asset_binding': asset(f'canary.appearance:outfit/{outfit.get("lookType", 0)}/addon-{bit}')})
            row('outfit.lookAddons', 'mapped', destination='/monster/presentation/appearance/attachment_bindings', line=line_of(r'lookAddons'),
                resolution=f'lookAddons {addons} is a bit mask of the outfit addons (1 first, 2 second, 3 both); each shown addon is a '
                           'declared attachment binding of the outfit.')
        if outfit.get('lookMount'):
            presentation['appearance']['attachment_bindings'].append(
                {'slot': 'mount', 'asset_binding': asset(f'canary.appearance:outfit/{outfit["lookMount"]}')})
            for slot, key in (('mount_head', 'lookMountHead'), ('mount_body', 'lookMountBody'),
                              ('mount_legs', 'lookMountLegs'), ('mount_feet', 'lookMountFeet')):
                if outfit.get(key):
                    presentation['appearance']['palette_bindings'].append({'slot': slot, 'palette_binding': asset(f'canary.appearance:palette/{outfit[key]}')})
            row('outfit.lookMount', 'mapped', destination='/monster/presentation/appearance/attachment_bindings', line=line_of(r'lookMount'),
                resolution='The creature is shown riding mount look type ' + str(outfit['lookMount']) + ' (declared attachment binding).')

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
        if 'description' not in m:
            row('description', 'mapped', destination='/monster/creature/inspection/description', line=1,
                resolution='No description in the source; monsters.hpp MonsterType sets nameDescription to the monster name.')
        row('raceId', 'metadata_only', line=line_of(r'^monster\.raceId'), resolution='Foreign identifier; provenance only.') if 'raceId' in m else None
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
            kind, script, effect = EVENTS.get(event, (None, None, None))
            encounters = ENCOUNTERS.get((event, creature['identity']['key']))
            if encounters:
                row(f'events={event}', 'approved_omission', 'script', line=line_of(r'^monster\.events'),
                    resolution=f'{script} {effect}. Relocated to Encounter {", ".join(encounters)}, whose manifest covers this '
                               'creature for the whole event; the monster keeps no copy of the logic (D20, D28).')
            elif kind == 'quest':
                row(f'events={event}', 'approved_omission', 'script', line=line_of(r'^monster\.events'),
                    resolution=f'{script} {effect}. ' + QUEST_EVENT_OMISSION)
            elif kind == 'encounter_bookkeeping':
                row(f'events={event}', 'approved_omission', 'script', line=line_of(r'^monster\.events'),
                    resolution=f'{script} {effect}. ' + ENCOUNTER_OMISSION)
            elif kind == 'no_effect':
                row(f'events={event}', 'approved_omission', 'script', line=line_of(r'^monster\.events'),
                    resolution=f'{script}: {effect}. The event has no observable effect in Canary.')
            elif kind == 'monster_behavior':
                row(f'events={event}', 'unresolved_semantics', 'script', line=line_of(r'^monster\.events'),
                    resolution=f'{script} {effect}. Monster behaviour script; needs a native behaviour (D13).')
            elif kind == 'encounter_mechanic':
                row(f'events={event}', 'unresolved_semantics', 'script', line=line_of(r'^monster\.events'),
                    resolution=f'{script} {effect}. Combat-changing encounter mechanic (D9: Encounter); blocked until the Encounter models it.')
            elif kind == 'low_confidence':
                row(f'events={event}', 'unresolved_semantics', 'script', line=line_of(r'^monster\.events'),
                    resolution=f'{script}: {effect}. Classified with low confidence in {EVENT_CENSUS.name}; needs a manual read.')
            else:
                row(f'events={event}', 'unresolved_semantics', 'script', line=line_of(r'^monster\.events'),
                    resolution='Registered creature event whose script has not been verified.')
        for callback in sorted(callbacks):
            row(f'mType.{callback}', 'unresolved_semantics', 'script', line=line_of(r'mType\.' + callback),
                resolution='Inline Lua callback; needs an explicit native behaviour resolution.')

        sources = [{'repository': REPOSITORY, 'revision': REVISION}]
        self.adopt_wiki(s, monster, rows, sources, definitions, line_of)
        definitions.discard(('Creature', creature['identity']['key']))
        catalog = {'definitions': [ref(f, k) for f, k in sorted(definitions)], 'assets': sorted(assets)}
        manifest = {'sources': sources, 'entries': rows}
        source = {'file': source_file, 'git_blob': blob_id(path.read_bytes())}
        return s, monster, deps, catalog, manifest, source

    def adopt_wiki(self, s, monster, rows, sources, definitions, line_of):
        """Apply the owner-approved reference-date wiki values recorded by wiki_compare.py (D15)."""
        record = self.wiki.get(s)
        if not record or record.get('status') != 'COMPARED':
            return

        def source(title, page_id, revision_id, sha256):
            sources.append({'kind': 'mediawiki', 'api': WIKI_API, 'title': title, 'page_id': page_id,
                            'revision_id': revision_id, 'content_sha256': sha256})
            return len(sources) - 1

        def wiki_row(index, title, line, field, destination, resolution, status='mapped'):
            entry = {'source_index': index, 'source_file': title, 'source_line': line, 'source_field': field,
                     'kind': 'field' if status == 'mapped' else 'dependency', 'status': status, 'resolution': resolution}
            if destination:
                entry['destination'] = destination
            rows.append(entry)

        def superseded(canary_value, wiki_raw):
            return f'Canary value {canary_value} superseded by the reference-date wiki value {wiki_raw} ({WIKI_ADOPTION}).'

        title = record['wiki_title']
        page = source(title, record['page_id'], record['cut_revision_id'], record['cut_content_sha256'])
        creature, behavior = monster['creature'], monster['behavior']

        def supersede(canary_field, pattern, diff):
            text = superseded(diff['canary'], diff['wiki_raw'])
            for entry in rows:
                if entry['source_index'] == 0 and entry['source_field'] == canary_field and entry['status'] == 'mapped':
                    entry['status'] = 'approved_omission'
                    entry.pop('destination', None)
                    entry['resolution'] = text
                    return
            rows.append({'source_index': 0, 'source_file': rows[0]['source_file'], 'source_line': line_of(pattern),
                         'source_field': canary_field, 'kind': 'field', 'status': 'approved_omission', 'resolution': text})

        def adopt(diff, destination, canary_field, pattern, label):
            supersede(canary_field, pattern, diff)
            wiki_row(page, title, diff.get('wiki_line', 1), f'Infobox Creature.{label}', destination,
                     f'Wiki {label} "{diff["wiki_raw"]}" ({WIKI_ADOPTION}).')

        for diff in (r for r in record['rows'] if r['status'] == 'DIFF'):
            field, value = diff['field'], diff.get('wiki')
            if field == 'mitigation_percent' and isinstance(value, (int, float)) and 0 <= value <= 100:
                creature['stats']['mitigation_percent'] = ratio(value)
                adopt(diff, '/monster/creature/stats/mitigation_percent', 'defenses.mitigation', r'mitigation\s*=', 'mitigation')
            elif field == 'max_health' and isinstance(value, int) and value > 0:
                creature['stats']['max_health'] = creature['stats']['initial_health'] = value
                behavior['targeting']['flee_health'] = min(behavior['targeting']['flee_health'], value)
                adopt(diff, '/monster/creature/stats/max_health', 'maxHealth', r'^monster\.maxHealth', 'hp')
                supersede('health', r'^monster\.health', diff)
            elif field == 'experience' and isinstance(value, int):
                creature['stats']['experience'] = value
                adopt(diff, '/monster/creature/stats/experience', 'experience', r'^monster\.experience', 'exp')
            elif field == 'armor' and isinstance(value, int):
                creature['stats']['armor'] = value
                adopt(diff, '/monster/creature/stats/armor', 'defenses.armor', r'armor\s*=', 'armor')
            elif field.startswith('resistance.') and isinstance(value, (int, float)) and value <= 100:
                damage = field.split('.', 1)[1]
                creature['resistances'] = [r for r in creature['resistances'] if r['damage_type'] != damage]
                creature['immunities']['damage_types'] = [d for d in creature['immunities']['damage_types'] if d != damage]
                if value == 100:
                    creature['immunities']['damage_types'] = sorted(creature['immunities']['damage_types'] + [damage])
                elif value:
                    creature['resistances'].append({'damage_type': damage, 'reduction_percent': ratio(value)})
                    creature['resistances'].sort(key=lambda r: r['damage_type'])
                adopt(diff, '/monster/creature/resistances', f'elements.{damage}', r'^monster\.elements', f'{damage} modifier')
            elif field == 'pushable' and isinstance(value, bool):
                behavior['movement']['pushable'] = value
                adopt(diff, '/monster/behavior/movement/pushable', 'flags.pushable', r'^\s*pushable\s*=', 'pushable')
            elif field == 'push_items' and isinstance(value, bool):
                behavior['movement']['push_items'] = value
                adopt(diff, '/monster/behavior/movement/push_items', 'flags.canPushItems', r'canPushItems', 'pushobjects')
            elif field == 'sense_invisible' and isinstance(value, bool):
                behavior['targeting']['sense_invisible'] = value
                adopt(diff, '/monster/behavior/targeting/sense_invisible', 'immunities.invisible', r'"invisible"', 'senseinvis')
            elif field == 'paralyze_immune' and isinstance(value, bool):
                conditions = set(creature['immunities']['conditions']) - {'paralyze'} | ({'paralyze'} if value else set())
                creature['immunities']['conditions'] = sorted(conditions)
                adopt(diff, '/monster/creature/immunities/conditions', 'immunities.paralyze', r'"paralyze"', 'paraimmune')
            elif field == 'illusionable' and isinstance(value, bool):
                creature['flags']['illusionable'] = value
                adopt(diff, '/monster/creature/flags/illusionable', 'flags.illusionable', r'illusionable\s*=', 'illusionable')
            elif field == 'flee_health' and isinstance(value, int):
                behavior['targeting']['flee_health'] = min(value, creature['stats']['max_health'])
                adopt(diff, '/monster/behavior/targeting/flee_health', 'flags.runHealth', r'runHealth', 'runsat')
            elif field == 'bestiary.class' and getattr(self, 'pending_bestiary', None) and isinstance(value, str):
                source_bestiary, index = self.pending_bestiary
                if value != str(source_bestiary.get('class', '')).lower().replace(' ', '_'):
                    continue
                creature['bestiary'] = self.bestiary_payload(source_bestiary, value)
                rows[index].update({'status': 'mapped', 'kind': 'field', 'destination': '/monster/creature/bestiary',
                                    'resolution': rows[index]['resolution'].split(' Bestiary omitted')[0] + ' The Bestiary '
                                    f'class "{source_bestiary["class"]}" and the reference-date wiki class "{diff["wiki_raw"]}" '
                                    f'agree, so the taxonomy is {value} ({WIKI_ADOPTION}).'})
                wiki_row(page, title, diff.get('wiki_line', 1), 'Infobox Creature.bestiaryclass', '/monster/creature/bestiary/taxonomy',
                         f'Wiki bestiaryclass "{diff["wiki_raw"]}" ({WIKI_ADOPTION}).')
            elif field in ('bestiary.difficulty', 'bestiary.occurrence') and 'bestiary' in creature and isinstance(value, str):
                key = field.split('.')[1]
                allowed = DIFFICULTY.values() if key == 'difficulty' else OCCURRENCE.values()
                if value in allowed and creature['bestiary'][key] != value:
                    creature['bestiary'][key] = value
                    adopt(diff, f'/monster/creature/bestiary/{key}', f'Bestiary.{key}', r'^monster\.Bestiary',
                          'bestiarylevel' if key == 'difficulty' else 'occurrence')
        self.adopt_wiki_loot(record, monster, rows, source, wiki_row, definitions)

    def adopt_wiki_loot(self, record, monster, rows, source, wiki_row, definitions):
        """D15 loot rules: wiki loot missing in Canary is added; Canary chances take the wiki estimate at >= 10 drops."""
        stats = record.get('loot_statistics') or {}
        loot_diff = next((r for r in record['rows'] if r['field'] == 'loot.items'), {})
        only_wiki = loot_diff.get('only_wiki', [])
        if 'loot' not in monster or not (only_wiki or record.get('loot_chances')):
            return
        title = record['wiki_title']
        if stats.get('status') != 'COMPARED':
            for name in only_wiki:
                wiki_row(1, title, loot_diff['wiki_line'], f'Infobox Creature.loot={name}', None, status='approved_omission',
                         resolution='Wiki lists the item but no Loot Statistics page gives a probability, so the Canary loot '
                                    'list is kept and the item is not added (owner decision D23).')
            return
        stat = source(stats['page_title'], stats['page_id'], stats['cut_revision_id'], stats['cut_content_sha256'])
        block = f'the version {stats["version"]} block of {stats["page_title"]} ({stats["kills"]} kills)'
        entries = monster['loot']['entries']

        def estimate(times, share=1):
            exact = Fraction(times * 100, stats['kills']) / share
            percent = (Decimal(exact.numerator) / Decimal(exact.denominator)).quantize(Decimal('0.0001'), ROUND_HALF_EVEN)
            return exact, int(percent) if percent == percent.to_integral_value() else float(percent)

        for index, chance in enumerate(record.get('loot_chances', [])):
            position = chance.get('position', index)
            canary_row = next((r for r in rows if r['source_index'] == 0 and r['source_field'] == f'loot[{position + 1}]'
                               and r['status'] == 'mapped'), None)
            if canary_row is None:
                continue
            index = int(canary_row['destination'].rsplit('/', 1)[1])
            if chance['status'] in ('CONSISTENT', 'DIFF') and chance['confidence'] == 'estimate':
                exact, value = estimate(chance['times'])
                entries[index]['probability_percent'] = value
                canary_row['resolution'] += (f' Probability taken from the wiki estimate (source {stat}); Canary had '
                                             f'{chance["canary_percent"]}% ({WIKI_ADOPTION}).')
                wiki_row(stat, stats['page_title'], chance['wiki_line'], f'Loot2.{chance["item"]}',
                         f'/monster/loot/entries/{index}/probability_percent',
                         f'{chance["times"]} drops / {stats["kills"]} kills in {block}: {float(exact):.6f}%, 95% Wilson interval '
                         f'{chance["interval_95_wilson"][0]}-{chance["interval_95_wilson"][1]}%, rounded half-even to 1 ppm ({LOOT_RATE_RULE}).')
            elif chance['status'] in ('CONSISTENT', 'DIFF'):
                canary_row['resolution'] += (f' Wiki {block}: {chance["times"]} drops, estimate {chance["wiki_percent"]}% (95% '
                                             f'interval {chance["interval_95_wilson"][0]}-{chance["interval_95_wilson"][1]}%), '
                                             f'low confidence; Canary probability kept ({LOOT_RATE_RULE}).')
            elif chance['status'] == 'NOT_OBSERVED':
                canary_row['resolution'] += (f' Not observed in {block} although the wiki infobox lists it at the cut, so it was '
                                             f'probably added later; Canary probability kept ({LOOT_RATE_RULE}).')
            else:
                canary_row['resolution'] += ' ' + chance.get('note', '') + ' Canary probability kept.'

        wiki_items = [i for i in stats['items'] if i['name'] in only_wiki]
        # Rows resolved through a disambiguation page come last, so a row naming the exact item page wins.
        wiki_items.sort(key=lambda i: bool((i.get('item_page') or {}).get('variants')))
        listed = {e['item']['key'] for e in entries}
        for item in wiki_items:
            candidates = self.index.get(item['name'], [])
            page_ids = (item.get('item_page') or {}).get('item_ids', [])
            variants = (item.get('item_page') or {}).get('variants', [])
            if variants:
                mine = [v for v in variants if title.lower() in v.get('dropped_by', [])]
                chosen = mine if len(mine) == 1 else variants
                page_ids = sorted({i for v in chosen for i in v['item_ids']})
                for v in chosen:
                    source(v['page_title'], v['page_id'], v['cut_revision_id'], v['cut_content_sha256'])
                item = {**item, 'item_page': {**item['item_page'], 'item_ids': page_ids}}
                candidates = [] if len(mine) == 1 else candidates
                note_variant = (f' The wiki page is a disambiguation; its item page "{mine[0]["page_title"]}" names {title} in '
                                f'droppedby (line {mine[0]["droppedby_line"]}) and declares itemid {page_ids} (owner decision '
                                'D25).' if len(mine) == 1 else
                                f' The wiki page is a disambiguation and no variant names {title} in droppedby, so every '
                                f'variant id {page_ids} is used (owner decision D22).')
            else:
                note_variant = ''
            narrowed = [i for i in candidates if i in page_ids]
            id_note = note_variant
            if len(candidates) > 1 and len(narrowed) == 1:
                page_info = item['item_page']
                page_source = source(page_info['page_title'], page_info['page_id'], page_info['cut_revision_id'],
                                     page_info['cut_content_sha256'])
                id_note += (f' The name matches ids {sorted(candidates)}; the item page (source {page_source}, line '
                           f'{page_info["itemid_line"]}) declares itemid {narrowed[0]}.')
                candidates = narrowed
            elif candidates and page_ids and not narrowed and len(page_ids) == 1 and page_ids[0] in self.names:
                page_info = item['item_page']
                page_source = source(page_info['page_title'], page_info['page_id'], page_info['cut_revision_id'],
                                     page_info['cut_content_sha256'])
                id_note += (f' The name matches Canary ids {sorted(candidates)}, but the item page (source {page_source}, line '
                           f'{page_info["itemid_line"]}) declares itemid {page_ids[0]} ("{self.names[page_ids[0]]}"); the '
                           'wiki decides (owner decision D25).')
                candidates = page_ids
            elif len(candidates) == 1 and page_ids and candidates[0] not in page_ids:
                candidates = []
            elif not candidates and len(page_ids) == 1 and page_ids[0] in self.names:
                page_info = item['item_page']
                page_source = source(page_info['page_title'], page_info['page_id'], page_info['cut_revision_id'],
                                     page_info['cut_content_sha256'])
                id_note += (f' No Canary item has the wiki name; the item page (source {page_source}, line '
                           f'{page_info["itemid_line"]}) declares itemid {page_ids[0]}.')
                candidates = page_ids
            if len(candidates) > 1:
                pool = narrowed or candidates
                # MoveEvent equip transforms the dropped item into its transformEquipTo id; that id never drops.
                worn = {int(self.items[i]['attributes']['transformequipto']) for i in pool
                        if 'transformequipto' in self.items.get(i, {}).get('attributes', {})}
                unworn = [i for i in pool if i not in worn]
                if len(unworn) == 1:
                    id_note += (f' Ids {sorted(pool)} share the name; {sorted(worn & set(pool))} is the equipped state '
                                f'(items.xml transformEquipTo), so the dropped item is {unworn[0]}.')
                    candidates = unworn
            page_set = sorted(set(page_ids))
            if len(candidates) != 1 and len(page_set) > 1 and (set(candidates) == set(page_set) or
                                                               (not candidates and all(i in self.names for i in page_set))):
                page_info = item['item_page']
                page_source = source(page_info['page_title'], page_info['page_id'], page_info['cut_revision_id'],
                                     page_info['cut_content_sha256'])
                id_note += (f' The item page (source {page_source}, line {page_info["itemid_line"]}) lists ids {page_set} for '
                            f'this name, so each id gets an equal share of the probability (owner decision D22).')
                candidates = page_set
            elif item['times'] and len(candidates) != 1:
                wiki_row(stat, stats['page_title'], item['line'] or stats['kills_line'], f'Loot2.{item["name"]}', None,
                         status='unresolved_dependency',
                         resolution=f'{item["times"]} drops recorded; item name resolves to {len(candidates)} ids.')
                continue
            if item['times'] == 0:
                wiki_row(stat, stats['page_title'], item['line'] or stats['kills_line'], f'Loot2.{item["name"]}', None,
                         status='approved_omission', resolution=f'0 drops in {block}: no probability, so the Canary loot '
                                                                'list is kept (owner decision D23).')
                continue
            if variants and all(f'canary:item/{i}' in listed for i in candidates):
                wiki_row(stat, stats['page_title'], item['line'] or stats['kills_line'], f'Loot2.{item["name"]}', None,
                         status='approved_omission', resolution=f'{note_variant.strip()} The id is already in this loot table '
                         'under its own statistics row, so this row is not added again.')
                continue
            for item_id in candidates:
                listed.add(f'canary:item/{item_id}')
                exact, value = estimate(item['times'], len(candidates))
                low, _, high = item['amount'].partition('-')
                entries.append({'item': ref('Item', f'canary:item/{item_id}'), 'min_count': int(low), 'max_count': int(high or low),
                                'probability_percent': value, 'skip_later_same_item_after_success': False})
                definitions.add(('Item', f'canary:item/{item_id}'))
                confidence = '' if item['times'] >= LOW_CONFIDENCE_DROPS else ' Low confidence: fewer than 10 drops.'
                share = f' / {len(candidates)} ids' if len(candidates) > 1 else ''
                wiki_row(stat, stats['page_title'], item['line'], f'Loot2.{item["name"]}' + (f'#{item_id}' if share else ''),
                         f'/monster/loot/entries/{len(entries) - 1}',
                         f'Listed in the {title} infobox loot (source 1, line {loot_diff["wiki_line"]}), absent in Canary. '
                         f'Probability = {item["times"]} drops / {stats["kills"]} kills{share} in {block}, {float(exact):.6f}% '
                         f'rounded half-even to 1 ppm ({WIKI_ADOPTION}).{confidence}{id_note} Item {item_id} '
                         f'"{self.names.get(item_id)}" is a declared source-scoped reference, not an admitted canonical Item.')

        # Keep the registrar's ascending-probability order (stable) and move every loot pointer with its entry.
        order = sorted(range(len(entries)), key=lambda i: entries[i]['probability_percent'])
        moved = {old: new for new, old in enumerate(order)}
        entries[:] = [entries[i] for i in order]
        for entry in rows:
            match = re.fullmatch(r'/monster/loot/entries/(\d+)(/.*)?', entry.get('destination', ''))
            if match:
                entry['destination'] = f'/monster/loot/entries/{moved[int(match.group(1))]}{match.group(2) or ""}'

    def registered_spell(self, spell, deps, asset):
        """A monster spell entry naming a registered spell script (D10, D11, D12). Returns (ability key, note,
        schedule extras), ('UNRESOLVED', reason) or ('OMIT', reason)."""
        if self.spell_scripts is None:
            self.spell_scripts = spell_scripts.SpellScripts(self.canary)
        name = str(spell.get('name', '')).lower()
        info = self.spell_scripts.evaluate(name)
        if info is None:
            return 'UNRESOLVED', f'"{name}" is neither an inline spell kind nor a registered spell name (D10).'
        where = f'registered {info["kind"]} spell "{name}" ({info["script"]})'
        if info.get('tier') == 'NOOP':
            return 'OMIT', f'{where} returns false for a non-player caster, so it has no effect in Canary (D14).'
        pattern = self.behaviour_patterns().get(name)
        tag = f' D18 pattern `{pattern}` (samples/{BEHAVIOUR_PATTERNS}).' if pattern else ''
        if info.get('tier') == 'P4' or 'error' in info:
            reason = '; '.join(info.get('tier_reasons') or []) or info.get('error', '')
            probe_note = ''
            if pattern in PROBED_PATTERNS and 'error' not in info:
                result, probe_note = self.probed_spell(name, info, where, spell, deps, asset, pattern)
                if result:
                    return result
                probe_note = f' Probe: {probe_note}.'
            return 'UNRESOLVED', f'{where} has custom logic ({reason[:200]}); needs a native behaviour (D13).{tag}{probe_note}'
        key = f'canary:ability/spell/{slug(name)}'
        flags = info['spell_calls']
        geometry = {'needs_target': bool(flags.get('needTarget', [False])[0] or flags.get('needCasterTargetOrDirection', [False])[0]),
                    'needs_direction': bool(flags.get('needDirection', [False])[0])}
        range_tiles = int(flags.get('range', [0])[0] or 0)
        notes = [f'{where}, tier {info["tier"]}: resolved by name as Canary does (rune, instant, then built-in kind; D10).']
        notes += info.get('tier_reasons') or []
        existing = {a['identity']['key'] for a in deps['abilities']}
        geometric = self.geometric_dot(info) if len(info['variants']) > 2 else None
        if geometric and key not in existing:
            template, body, summary = geometric
            try:
                uses_magnitude = self.combat_ability(key, template, geometry, range_tiles, deps, asset, notes,
                                                     extra=[('-condition-1', body)])
            except SpellUnresolved as exc:
                return 'UNRESOLVED', f'{where}: {exc} Needs a schema or native decision.{tag}'
            notes.append(summary)
            return self.spell_result(key, spell, notes, uses_magnitude)
        if geometric:
            notes.append(geometric[2])
            return self.spell_result(key, spell, notes, self.ability_uses_magnitude(key, deps))
        if len(info['variants']) > 1 and key not in existing:
            payloads = set()
            for combat in info['variants']:
                scratch = {'abilities': [], 'effects': [], 'formulas': []}
                self.combat_ability('@variant', info['combats'][combat], geometry, range_tiles, scratch, lambda a: a, [])
                payloads.add(json.dumps(scratch, sort_keys=True))
            if len(payloads) == 1:
                notes.append(f'All {len(info["variants"])} random variants convert to the same Ability, so they form one.')
                info = {**info, 'variants': info['variants'][:1]}
        order = list(dict.fromkeys(info['variants']))
        uses_magnitude = False
        try:
            if len(order) == 1:
                if key not in existing:
                    uses_magnitude = self.combat_ability(key, info['combats'][order[0]], geometry, range_tiles, deps, asset, notes)
                else:
                    uses_magnitude = self.ability_uses_magnitude(key, deps)
            else:
                variant_keys = [f'{key}/variant-{n}' for n in range(1, len(info['variants']) + 1)]
                if key not in existing:
                    for variant_key, combat in zip(variant_keys, info['variants']):
                        uses_magnitude |= self.combat_ability(variant_key, info['combats'][combat], geometry, range_tiles, deps, asset, notes)
                    deps['abilities'].append({'identity': ident(key), 'kind': 'spell', 'range_tiles': range_tiles, **geometry,
                                              'variants': [ref('Ability', k) for k in variant_keys]})
                else:
                    uses_magnitude = any(self.ability_uses_magnitude(k, deps) for k in variant_keys)
                notes.append(f'{len(variant_keys)} variants picked uniformly by math.random (D12).')
        except SpellUnresolved as exc:
            return 'UNRESOLVED', f'{where}: {exc} Needs a schema or native decision.{tag}'
        return self.spell_result(key, spell, notes, uses_magnitude)

    def spell_result(self, key, spell, notes, uses_magnitude):
        extras = {}
        if uses_magnitude:
            low, high = abs(spell.get('minDamage', 0)), abs(spell.get('maxDamage', 0))
            extras['magnitude'] = {'minimum': min(low, high), 'maximum': max(low, high)}
            notes.append('Damage/heal magnitude comes from this monster entry: combat.cpp Combat::getCombatDamage uses '
                         'Monster::getCombatValues (D11); the script formula applies only to players.')
        if spell.get('range'):
            extras['range_tiles'] = int(spell['range'])
        return key, ' '.join(dict.fromkeys(notes)) + ' ', extras

    def probed_spell(self, name, info, where, spell, deps, asset, pattern):
        """D18: model a custom-logic spell from its probed behaviour. Returns ((key, note, extras), '') or (None, reason)."""
        key = f'canary:ability/spell/{slug(name)}'
        flags = info['spell_calls']
        geometry = {'needs_target': bool(flags.get('needTarget', [False])[0] or flags.get('needCasterTargetOrDirection', [False])[0]),
                    'needs_direction': bool(flags.get('needDirection', [False])[0])}
        range_tiles = int(flags.get('range', [0])[0] or 0)
        notes = [f'{where}: custom logic modelled from its probed behaviour against stub worlds (D18 pattern `{pattern}`, '
                 'spell_probes.py).']
        if key not in {a['identity']['key'] for a in deps['abilities']}:
            probe = spell_probes.Probe(self.canary, info['script'], self.spell_scripts.areas, {})
            if probe.entries():
                return None, 'the script rolls a value or touches the world while it loads (fixed per server start)'
            lua_spell = probe.spell(name)
            scratch = {'abilities': [], 'effects': [], 'formulas': []}
            try:
                if pattern == 'conditional_summon':
                    self.probe_summon(probe, lua_spell, key, geometry, range_tiles, scratch, asset, notes)
                elif pattern == 'path_trail_missile':
                    self.path_trail(probe, key, geometry, range_tiles, scratch, asset, notes)
                elif pattern == 'heal_allies_in_area':
                    self.probe_callbacks(probe, lua_spell, key, geometry, range_tiles, scratch, asset, notes)
                else:
                    self.probe_remove_items(probe, lua_spell, key, scratch, asset, notes)
            except SpellUnresolved as exc:
                return None, str(exc)
            for family in ('abilities', 'effects', 'formulas'):
                known = {e['identity']['key'] for e in deps[family]}
                deps[family].extend(e for e in scratch[family] if e['identity']['key'] not in known)
        extras = {}
        if self.ability_uses_magnitude(key, deps):
            low, high = abs(spell.get('minDamage', 0)), abs(spell.get('maxDamage', 0))
            extras['magnitude'] = {'minimum': min(low, high), 'maximum': max(low, high)}
            notes.append('Damage/heal magnitude of the combat itself comes from this monster entry (D11).')
        if spell.get('range'):
            extras['range_tiles'] = int(spell['range'])
        return (key, ' '.join(dict.fromkeys(notes)) + ' ', extras), ''

    def cast_runs(self, probe, lua_spell, summons=(0,), modes=('low', 'high')):
        """onCastSpell against a caster with each summon count and random mode: {(summons, mode): (log, executed)}."""
        runs = {}
        for count in summons:
            for mode in modes:
                caster = probe.make('monster', 'caster', None, count)
                probe.creatures['caster'] = caster
                ok, log = probe.run(lua_spell['onCastSpell'], caster, probe.lua.table(), random=mode)
                if not ok:
                    raise SpellUnresolved('onCastSpell needs more of the world than the stubs model')
                runs[count, mode] = (log, [c['__n'] for c in probe.rec['executed'].values()])
        return runs

    def executed_combat(self, probe, runs):
        executed = {tuple(e) for _, e in runs.values()}
        if len(executed) != 1 or len(next(iter(executed))) > 1:
            raise SpellUnresolved(f'the casts execute different combats {sorted(executed)}')
        numbers = next(iter(executed))
        if not numbers:
            return None
        combat = next(c for c in probe.rec['combats'].values() if c['__n'] == numbers[0])
        return self.spell_scripts._combat(probe.lua, combat)

    def probe_summon(self, probe, lua_spell, key, geometry, range_tiles, deps, asset, notes):
        runs = self.cast_runs(probe, lua_spell, summons=range(16))
        for log, _ in runs.values():
            other = {e[0] for e in log} - {'random', 'createMonster', 'setMaster', 'say'}
            if other:
                raise SpellUnresolved(f'the cast also does {sorted(other)}')
        created = {k: [e for e in log if e[0] == 'createMonster'] for k, (log, _) in runs.items()}
        counts = {s: len(created[s, 'low']) for s in range(16)}
        if any(len(created[s, 'high']) != counts[s] for s in range(16)):
            raise SpellUnresolved('the number of summons depends on a random roll')
        limit = next((s for s in range(16) if counts[s] == 0), None)
        if not limit or any(counts[s] for s in range(limit, 16)):
            raise SpellUnresolved(f'created counts {counts} have no summon limit')
        if all(counts[s] == limit - s for s in range(limit)):
            mode, count = 'fill_to_limit', limit
        elif len({counts[s] for s in range(limit)}) == 1:
            mode, count = 'fixed', counts[0]
        else:
            raise SpellUnresolved(f'created counts {counts} are neither fixed nor filling to a limit')
        every = [e for entries in created.values() for e in entries]
        names = sorted({e[1] for e in every})
        if len(names) != 1:
            raise SpellUnresolved(f'the summons pick between {names}')
        if any(e[4] for e in every):
            raise SpellUnresolved('summons are placed on another floor')
        low = {(e[2], e[3]) for s in range(limit) for e in created[s, 'low']}
        high = {(e[2], e[3]) for s in range(limit) for e in created[s, 'high']}
        reach = next(iter(high))[0] if len(high) == 1 else None
        if not (len(low) == 1 and reach is not None and low == {(-reach, -reach)} and high == {(reach, reach)} and reach >= 0):
            raise SpellUnresolved(f'summon positions {sorted(low | high)} are not the caster tile plus a uniform offset')
        masters = [e for log, _ in runs.values() for e in log if e[0] == 'setMaster']
        if masters and (len(masters) != len(every) or not all(e[2] for e in masters)):
            raise SpellUnresolved('only some summons get the caster as master')
        for text in sorted({e[1] for log, _ in runs.values() for e in log if e[0] == 'say'}):
            notes.append(f'The cast says "{text}" first (voice line, not modelled).')
        creature = f'canary:creature/{slug(names[0])}'
        self.pending_definitions.add(('Creature', creature))
        body = {'operation': 'summon_creature', 'summon': {
            'creatures': [ref('Creature', creature)], 'count_mode': mode, 'count': count, 'only_below_summons': limit,
            'owned': bool(masters), 'max_offset_tiles': reach}}
        notes.append(f'Probed: {counts[0]} created with no summons, none from {limit} summons on; Game.createMonster at the caster '
                     f'position{f" +-{reach} per axis" if reach else ""}; setMaster {"on every summon" if masters else "never"}. A '
                     'failed placement (`if not mid then return`) ends that cast early in Canary.')
        combat = self.executed_combat(probe, runs)
        if combat is None:
            deps['effects'].append({'identity': ident(f'{key}/effect-summon'), **body})
            deps['abilities'].append({'identity': ident(key), 'kind': 'spell', 'range_tiles': range_tiles, **geometry,
                                      'effects': [ref('Effect', f'{key}/effect-summon')]})
            return
        if combat['callbacks']:
            raise SpellUnresolved('the executed combat has Lua callbacks')
        self.combat_ability(key, combat, geometry, range_tiles, deps, asset, notes, extra=[('-summon', body)])

    def probe_callbacks(self, probe, lua_spell, key, geometry, range_tiles, deps, asset, notes):
        runs = self.cast_runs(probe, lua_spell)
        if any(log for log, _ in runs.values()):
            raise SpellUnresolved('onCastSpell does more than execute its combat')
        combat = self.executed_combat(probe, runs)
        if combat is None or not combat['callbacks']:
            raise SpellUnresolved('no executed combat with a callback')
        params = self.engine_params(combat.get('param_calls', []), [])
        aggressive = params.get('COMBAT_PARAM_AGGRESSIVE', 1) != 0
        extra = []
        for callback, function_name in sorted(combat['callbacks'].items()):
            function = probe.lua.globals()[function_name]
            if callback == 'CALLBACK_PARAM_TARGETCREATURE':
                extra += self.probe_target_creature(probe, function, key, len(extra), aggressive, deps)
            elif callback == 'CALLBACK_PARAM_TARGETTILE':
                extra += self.probe_target_tile(probe, function, key, len(extra), deps)
            else:
                raise SpellUnresolved(f'callback {callback}')
        notes.append(f'Probed {", ".join(sorted(combat["callbacks"]))}: '
                     + '; '.join(f'{b["operation"]} {b["affects"]["kind"]}' for _, b in extra) + '.')
        self.combat_ability(key, {**combat, 'callbacks': {}}, geometry, range_tiles, deps, asset, notes, extra=extra)

    def health_body(self, kind, low, high, affects, formula_key, deps):
        damage = 'healing' if kind == 'COMBAT_HEALING' else DAMAGE.get(kind[len('COMBAT_'):])
        if damage is None:
            raise SpellUnresolved(f'callback combat type {kind}')
        deps['formulas'].append({'identity': ident(formula_key), 'kind': 'range', 'magnitude': {'minimum': low, 'maximum': high}})
        return {'operation': 'heal' if damage == 'healing' else 'damage', 'damage_type': damage,
                'formula': ref('Formula', formula_key), 'affects': affects}

    def probe_target_creature(self, probe, function, key, offset, aggressive, deps):
        caster = probe.make('monster', 'caster')
        probe.creatures['caster'] = caster
        player = probe.make('player', 'player')
        targets = {'player': player, 'player_summon': probe.make('monster', 'player summon', player),
                   'monster_summon': probe.make('monster', 'monster summon', probe.make('monster', 'other master')),
                   'monster': probe.make('monster', 'monster'), 'caster': caster}
        groups = {}
        for label, target in targets.items():
            ok, log = probe.run(function, caster, target)
            if not ok or any(e[0] != 'combatHealth' for e in log):
                raise SpellUnresolved(f'the target callback does more than change health ({label})')
            for e in log:
                groups.setdefault((e[2].lstrip('@'), e[3], e[4]), set()).add(label)
        kinds = {frozenset({'monster'}): 'masterless_monsters', frozenset({'monster', 'monster_summon'}): 'non_player_side',
                 frozenset({'player', 'player_summon'}): 'player_side'}
        extra = []
        for n, ((kind, low, high), labels) in enumerate(sorted(groups.items()), offset + 1):
            side = kinds.get(frozenset(labels - {'caster'}))
            if side is None:
                raise SpellUnresolved(f'the callback reaches {sorted(labels)}, which is no modelled target group')
            affects = {'kind': side, 'top_creature_only': False, 'excludes_caster_name': False,
                       # combat.cpp CombatFunc passes the caster only to a non-aggressive combat.
                       'includes_caster': 'caster' in labels and not aggressive}
            extra.append((f'-callback-{n}', self.health_body(kind, min(low, high), max(low, high), affects,
                                                              f'{key}/formula-callback-{n}', deps)))
        return extra

    def probe_target_tile(self, probe, function, key, offset, deps):
        text = probe.source
        candidates = sorted({s.lower() for s in re.findall(r'"([^"\n]+)"', text)}) + ['zz unnamed monster']
        position = probe.lua.eval('Position(1001, 1000, 7)')
        caster = probe.make('monster', 'caster')
        probe.creatures['caster'] = caster
        healed, amounts = [], set()
        for name in candidates:
            per_mode = {}
            for mode in ('low', 'high'):
                probe.world['top'] = probe.make('monster', name)
                ok, log = probe.run(function, caster, position, random=mode)
                if not ok or any(e[0] not in ('tile', 'random', 'addHealth') for e in log):
                    raise SpellUnresolved(f'the tile callback does more than add health ({name})')
                per_mode[mode] = [e[2] for e in log if e[0] == 'addHealth']
            if per_mode['low'] or per_mode['high']:
                if len(per_mode['low']) != 1 or len(per_mode['high']) != 1:
                    raise SpellUnresolved(f'{name} is healed a varying number of times')
                healed.append(name)
                amounts.add((per_mode['low'][0], per_mode['high'][0]))
        if not healed or 'zz unnamed monster' in healed or len(amounts) != 1:
            raise SpellUnresolved(f'tile callback heals {healed} by {sorted(amounts)}')
        low, high = next(iter(amounts))

        def heals(caster_name, top_is_caster):
            me = probe.make('monster', caster_name)
            probe.creatures['caster'] = me
            probe.world['top'] = me if top_is_caster else probe.make('monster', caster_name)
            ok, log = probe.run(function, me, position)
            return ok and any(e[0] == 'addHealth' for e in log)

        affects = {'kind': 'named_creatures', 'creatures': [], 'top_creature_only': True,
                   'excludes_caster_name': not heals(healed[0], False), 'includes_caster': heals(healed[0], True)}
        for name in healed:
            creature = f'canary:creature/{slug(name)}'
            self.pending_definitions.add(('Creature', creature))
            affects['creatures'].append(ref('Creature', creature))
        return [(f'-callback-{offset + 1}', self.health_body('COMBAT_HEALING', min(low, high), max(low, high), affects,
                                                            f'{key}/formula-callback-{offset + 1}', deps))]

    def path_trail(self, probe, key, geometry, range_tiles, deps, asset, notes):
        """The Canary single-target 'chain' template: a path trail effect, then one combat on the target."""
        match = re.fullmatch(PATH_TRAIL, spell_scripts.cast_body(probe.source) or '')
        combats = list(probe.rec['combats'].values())
        if not match or len(combats) != 1:
            raise SpellUnresolved('onCastSpell is not the path trail template')
        clear_sight, search, trail = match.group(1) == 'true', int(match.group(2)), match.group(3)
        combat = self.spell_scripts._combat(probe.lua, combats[0])
        if combat['callbacks'] or combat['area']:
            raise SpellUnresolved('the path trail combat has callbacks or an area')
        self.combat_ability(key, combat, {**geometry, 'needs_target': True}, range_tiles, deps, asset, notes)
        ability = deps['abilities'][-1]
        ability['path_requirement'] = {'max_search_tiles': search, 'clear_sight': clear_sight}
        first = next(e for e in deps['effects'] if e['identity']['key'] == ability['effects'][0]['key'])
        first.setdefault('presentation', {})['path_asset_binding'] = asset(self.visual('@' + trail, 'effect')[0])
        notes.append(f'Template match: Position:getPathTo(target, 0, 0, true, {match.group(1)}, {search}) must find a path or '
                     f'the cast returns false; {trail} is sent on every path tile, then the combat runs on the target. The '
                     'name says chain but only the target is hit.')

    def probe_remove_items(self, probe, lua_spell, key, deps, asset, notes):
        probe.world['items'] = probe.lua.table()
        empty = self.cast_runs(probe, lua_spell, modes=('low',))[0, 'low'][0]
        tiles = sorted({tuple(e[1:4]) for e in empty if e[0] == 'tile'})
        queried = []
        for e in empty:
            if e[0] == 'getItemById' and e[4] not in queried:
                queried.append(e[4])
        if not tiles or not queried or any(dz for _, _, dz in tiles):
            raise SpellUnresolved('the cast inspects no items on its own floor')
        for present in (queried, queried[1:]):
            probe.world['items'] = probe.lua.table_from({i: True for i in present})
            log = self.cast_runs(probe, lua_spell, modes=('low',))[0, 'low'][0]
            removed = [e for e in log if e[0] == 'removeItem']
            if sorted(tuple(e[1:4]) for e in removed) != tiles or {e[4] for e in removed} != {present[0]}:
                raise SpellUnresolved('item removal is not one first-listed item per inspected tile')
            if any(e[0] not in ('tile', 'getItemById', 'removeItem', 'effect') for e in log):
                raise SpellUnresolved('the cast does more than remove items')
        effects = {e[4] for e in log if e[0] == 'effect'}
        items = []
        for item_id in queried:
            item = ref('Item', f'canary:item/{int(item_id)}')
            self.pending_definitions.add((item['family'], item['key']))
            items.append(item)
        body = {'operation': 'remove_items', 'removed_items': {'items': items, 'selection': 'first_listed_per_tile'}}
        if len(effects) == 1:
            binding, note = self.visual(next(iter(effects)), 'effect')
            body['presentation'] = {'impact_asset_binding': asset(binding)}
        xs, ys = [x for x, _, _ in tiles], [y for _, y, _ in tiles]
        rows = [''.join(('C' if (x, y) == (0, 0) else 'x') if (x, y, 0) in tiles else ('c' if (x, y) == (0, 0) else '.')
                        for x in range(min(xs), max(xs) + 1)) for y in range(min(ys), max(ys) + 1)]
        notes.append(f'Probed: every tile of the area around the caster loses the first present of items {queried}.')
        deps['effects'].append({'identity': ident(f'{key}/effect-remove'), **body})
        deps['abilities'].append({'identity': ident(key), 'kind': 'spell', 'range_tiles': 0, 'needs_target': False,
                                  'needs_direction': False, 'area': {'matrix': {'north': rows}},
                                  'effects': [ref('Effect', f'{key}/effect-remove')]})

    def undefined_damage_from_wiki(self, spell):
        """D25: the element of an undefined-damage combat from the one reference-date wiki ability it matches best."""
        import wiki_scenes
        record = (getattr(self, 'wiki', {}) or {}).get(getattr(self, 'current_slug', None)) or {}
        abilities = record.get('abilities') or []
        if not abilities:
            return None, 'The reference-date wiki lists no ability to decide the element (owner decision D25).'
        geometry = cast_geometry(length=spell.get('length', 0), spread=spell.get('spread', 0), radius=spell.get('radius'),
                                 target=bool(spell.get('target', False)))
        kind, tiles = wiki_scenes.canary_geometry({'identity': {'key': 'x/attack-1'}, 'kind': 'spell', **geometry})
        effect = self.magic_effects.get(spell['effect'][1:]) if isinstance(spell.get('effect'), str) else spell.get('effect')
        missile = self.missiles.get(spell['shootEffect'][1:]) if isinstance(spell.get('shootEffect'), str) else spell.get('shootEffect')
        maximum = max(abs(spell.get('minDamage', 0)), abs(spell.get('maxDamage', 0)))
        scored = []
        for ability in abilities:
            if ability.get('element') in (None, 'healing'):
                continue
            wiki_tiles = frozenset(tuple(t) for t in ability.get('tiles') or [])
            points = (2 * (ability.get('effect') is not None and ability['effect'] == effect) +
                      (ability.get('missile') is not None and ability['missile'] == missile) +
                      (ability.get('kind') == kind) + (ability.get('kind') == kind and wiki_tiles in wiki_scenes.rotations(tiles)) +
                      (ability.get('maximum') == maximum))
            scored.append((points, ability))
        scored.sort(key=lambda p: -p[0])
        if not scored or scored[0][0] < 3 or (len(scored) > 1 and scored[1][0] == scored[0][0]):
            return None, (f'No single reference-date wiki ability matches it by effect, missile, shape and maximum '
                          f'(best scores {[p for p, _ in scored[:3]]}; owner decision D25).')
        points, ability = scored[0]
        damage = {'life drain': 'life_drain', 'lifedrain': 'life_drain', 'mana drain': 'mana_drain',
                  'drown': 'drowning', 'poison': 'earth'}.get(ability['element'], ability['element'])
        if damage not in DAMAGE.values():
            return None, f'The matching wiki ability "{ability["name"]}" has element {ability["element"]!r} (owner decision D25).'
        return damage, (f'The reference-date wiki page (page {record.get("page_id")}, revision {record.get("cut_revision_id")}) '
                        f'ability "{ability["name"]}" ({ability["element_raw"]}) matches it best (score {points}), so the '
                        f'element is {damage} (owner decision D25).')

    def geometric_dot(self, info):
        """D21: (template combat, condition Effect, note) when the random variants differ only in one damage-over-time whose
        ticks grow geometrically from an integer base, every (base, tick count) pair appearing exactly once; else None."""
        combats = [info['combats'][n] for n in info['variants']]
        shape = {json.dumps({k: v for k, v in combats[0].items() if k != 'conditions'}, sort_keys=True, default=str)}
        pairs, header, factor = [], set(), None
        for combat in combats:
            shape.add(json.dumps({k: v for k, v in combat.items() if k != 'conditions'}, sort_keys=True, default=str))
            if len(shape) != 1 or len(combat['conditions']) != 1:
                return None
            condition = combat['conditions'][0]
            damages = [a for m, a in condition['calls'] if m == 'addDamage']
            others = tuple(tuple(a) for m, a in condition['calls'] if m != 'addDamage')
            if any(m not in ('addDamage', 'setParameter') for m, _ in condition['calls']) or len(damages) < 2:
                return None
            if any(r != 1 or v >= 0 for r, _, v in damages) or len({ms for _, ms, _ in damages}) != 1:
                return None
            values = [-v for _, _, v in damages]
            header.add((condition['type'], others, damages[0][1]))
            factor = factor or values[1] / values[0]
            expected, current = [values[0]], values[0]
            for _ in values[1:]:
                current = current * factor
                expected.append(current)
            if values != expected or values[0] != int(values[0]):
                return None
            pairs.append((int(values[0]), len(values)))
        bases, counts = {b for b, _ in pairs}, {n for _, n in pairs}
        full = {(b, n) for b in range(min(bases), max(bases) + 1) for n in counts}
        if len(header) != 1 or len(pairs) != len(set(pairs)) or set(pairs) != full:
            return None
        kind, others, interval = header.pop()
        params = dict(others)
        delayed = params.pop('CONDITION_PARAM_DELAYED', 0)
        params.pop('CONDITION_PARAM_SUBID', None)
        if params:
            return None
        body = {'operation': 'condition', 'condition': {'type': kind[len('CONDITION_'):].lower(), 'lifetime': 'damage_schedule',
                'damage_over_time': {'tick_profile': 'geometric', 'first_tick': 'after_interval' if delayed else 'immediate',
                                     'geometric': {'base_range': {'minimum': min(bases), 'maximum': max(bases)},
                                                   'factor': ratio(factor), 'tick_counts': sorted(counts),
                                                   'tick_interval_ms': int(interval)}}}}
        note = (f'{len(combats)} random variants differ only in a {kind} whose ticks start at an integer base '
                f'{min(bases)}-{max(bases)} and grow by x{factor} for {sorted(counts)} ticks, each pair once, so they are '
                'authored as one geometric damage over time (owner decision D21; Condition:addDamage truncates each tick).')
        return {**combats[0], 'conditions': []}, body, note

    def ability_uses_magnitude(self, key, deps):
        ability = next(a for a in deps['abilities'] if a['identity']['key'] == key)
        effects = {e['identity']['key']: e for e in deps['effects']}
        return any(effects[r['key']].get('formula', {}).get('key') == CASTER_MAGNITUDE for r in ability.get('effects', []))

    @staticmethod
    def bestiary_payload(bestiary, taxonomy):
        stars = bestiary.get('Stars', 0)
        payload = {'class': bestiary['class'], 'taxonomy': taxonomy, 'difficulty': DIFFICULTY[stars],
                   'occurrence': OCCURRENCE[bestiary.get('Occurrence', 0)], 'stars': stars,
                   'kill_thresholds': [bestiary['FirstUnlock'], bestiary['SecondUnlock'], bestiary['toKill']],
                   'charm_points': bestiary['CharmsPoints']}
        if bestiary.get('Locations', '').strip():
            payload['locations'] = bestiary['Locations']
        return payload

    def behaviour_patterns(self):
        """spell name -> D18 native behaviour pattern id from the committed pattern grouping."""
        if getattr(self, '_patterns', None) is None:
            path = ROOT / 'samples' / BEHAVIOUR_PATTERNS
            spells = json.loads(path.read_text(encoding='utf-8'))['spells'] if path.exists() else []
            self._patterns = {s['spell']: s['pattern'] for s in spells}
        return self._patterns

    def engine_params(self, param_calls, notes):
        """Combat:setParameter calls as the engine applies them, in call order: key and value are read as numbers
        (combat_functions.cpp luaCombatSetParameter), so an undefined global (nil) is 0 and a constant of another
        enum selects the parameter or value with its number. Values are named again by the parameter they set."""
        enums = self.spell_scripts.enums
        by_number = {v: k for k, v in enums['CombatParam_t'].items()}
        tables = (*enums.values(), self.magic_effects, self.missiles)

        def number(value):
            if isinstance(value, bool):
                return int(value)
            if isinstance(value, (int, float)):
                return int(value)
            return next((table[value] for table in tables if value in table), None)

        value_names = {'COMBAT_PARAM_TYPE': enums['CombatType_t'], 'COMBAT_PARAM_DISPEL': enums['ConditionType_t'],
                       'COMBAT_PARAM_CHAIN_EFFECT': self.magic_effects}
        params = {}
        for key, value in param_calls:
            key_number, value_number = number(key), number(value)
            if key_number is None:
                key_number = 0
                notes.append(f'{key} is not a Canary constant: {RULES["nil_zero"]}, so this call sets {by_number[0]}.')
            elif key not in enums['CombatParam_t']:
                notes.append(f'{key} is {key_number}, which the engine reads as {by_number.get(key_number, "no combat parameter")}.')
            if key_number not in by_number:
                continue
            name = by_number[key_number]
            if value_number is None:
                value_number = 0
                notes.append(f'{value} is not a Canary constant: {RULES["nil_zero"]}.')
            if name in value_names:
                reverse = {v: k for k, v in value_names[name].items()}
                params[name] = reverse.get(value_number, value_number)
            else:
                params[name] = value_number
        return params

    def combat_ability(self, key, combat, geometry, range_tiles, deps, asset, notes, extra=()):
        """One recorded Combat as an Ability plus its Effects; True when a damage/heal effect uses the caster magnitude.
        `extra` holds (suffix, body) Effects modelled from probed script logic (D18); they come first."""
        params = self.engine_params(combat.get('param_calls', []), notes)
        unsupported = set(params) - {'COMBAT_PARAM_TYPE', 'COMBAT_PARAM_EFFECT', 'COMBAT_PARAM_DISTANCEEFFECT',
                                     'COMBAT_PARAM_CHAIN_EFFECT', 'COMBAT_PARAM_CREATEITEM', 'COMBAT_PARAM_AGGRESSIVE',
                                     'COMBAT_PARAM_USECHARGES', 'COMBAT_PARAM_IMPACTSOUND', 'COMBAT_PARAM_CASTSOUND',
                                     'COMBAT_PARAM_BLOCKARMOR', 'COMBAT_PARAM_BLOCKSHIELD', 'COMBAT_PARAM_DISPEL'}
        if unsupported:
            raise SpellUnresolved(f'combat parameter(s) {sorted(unsupported)} have no authoring field.')
        if combat.get('formula'):
            raise SpellUnresolved(f'combat:setFormula{tuple(combat["formula"])} has no authoring field.')
        for callback in combat['callbacks']:
            if callback in ('CALLBACK_PARAM_LEVELMAGICVALUE', 'CALLBACK_PARAM_SKILLVALUE'):
                notes.append(f'{callback} is the player formula; a monster caster uses its own values instead.')
            elif callback != 'CALLBACK_PARAM_CHAINVALUE':
                raise SpellUnresolved(f'Lua combat callback {callback}.')
        if 'COMBAT_PARAM_USECHARGES' in params:
            notes.append('COMBAT_PARAM_USECHARGES only consumes player weapon charges.')
        if params.get('COMBAT_PARAM_AGGRESSIVE') in (0, False):
            notes.append('COMBAT_PARAM_AGGRESSIVE=0: a non-aggressive cast (no PvP/protection-zone checks).')
        visual = {}
        if params.get('COMBAT_PARAM_EFFECT') not in (None, 'CONST_ME_NONE', 0):
            binding, note = self.visual('@' + params['COMBAT_PARAM_EFFECT'] if isinstance(params['COMBAT_PARAM_EFFECT'], str)
                                        else params['COMBAT_PARAM_EFFECT'], 'effect')
            visual['impact_asset_binding'] = asset(binding)
            notes.append(note)
        if params.get('COMBAT_PARAM_DISTANCEEFFECT') not in (None, 'CONST_ANI_NONE', 0):
            value = params['COMBAT_PARAM_DISTANCEEFFECT']
            binding, note = self.visual('@' + value if isinstance(value, str) else value, 'missile')
            visual['projectile_asset_binding'] = asset(binding)
            notes.append(note)
        effects, uses_magnitude = [], False

        def add(suffix, body):
            effect_key = f'{key}/effect{suffix}'
            deps['effects'].append({'identity': ident(effect_key), **body})
            effects.append(ref('Effect', effect_key))

        for suffix, body in extra:
            add(suffix, body)
        if params.get('COMBAT_PARAM_TYPE') == 'COMBAT_NONE':
            notes.append('COMBAT_NONE: the combat itself changes no health; it shows its effects and runs its callbacks.')
            params.pop('COMBAT_PARAM_TYPE')
        if 'COMBAT_PARAM_TYPE' in params:
            kind = params['COMBAT_PARAM_TYPE']
            if not (isinstance(kind, str) and kind.startswith('COMBAT_') and kind[7:] in DAMAGE):
                raise SpellUnresolved(f'combat type {kind!r} has no authoring damage type.')
            damage = DAMAGE[kind[7:]]
            if not any(f['identity']['key'] == CASTER_MAGNITUDE for f in deps['formulas']):
                deps['formulas'].append({'identity': ident(CASTER_MAGNITUDE), 'kind': 'caster_magnitude'})
            mitigated = [name for name, param in (('armor', 'COMBAT_PARAM_BLOCKARMOR'), ('shield', 'COMBAT_PARAM_BLOCKSHIELD'))
                         if params.get(param) not in (None, 0, False)]
            if mitigated and damage != 'healing':
                notes.append(RULES['armor_script'] + '.')
            add('', {'operation': 'heal' if damage == 'healing' else 'damage', 'damage_type': damage,
                     'formula': ref('Formula', CASTER_MAGNITUDE),
                     **({'mitigated_by': mitigated} if mitigated and damage != 'healing' else {}),
                     **({'presentation': visual} if visual else {})})
            visual, uses_magnitude = {}, True
        if 'COMBAT_PARAM_CREATEITEM' in params:
            item = ref('Item', f'canary:item/{int(params["COMBAT_PARAM_CREATEITEM"])}')
            self.pending_definitions.add((item['family'], item['key']))
            add('-item', {'operation': 'create_item', 'created_item': item})
        if params.get('COMBAT_PARAM_DISPEL') not in (None, 'CONDITION_NONE'):
            notes.append(RULES['dispel'] + '.')
            add('-dispel', {'operation': 'remove_condition', 'removed_condition': params['COMBAT_PARAM_DISPEL'][len('CONDITION_'):].lower()})
        for n, condition in enumerate(combat['conditions'], 1):
            body = self.script_condition(condition, deps, f'{key}/formula-{n}', notes)
            if body:
                if visual:
                    body['presentation'], visual = visual, {}
                add(f'-condition-{n}', body)
        if visual and extra:
            add('-presentation', {'operation': 'presentation_only', 'presentation': visual})
        if not effects:
            if not visual:
                raise SpellUnresolved('the combat has no effect a monster caster can produce.')
            add('', {'operation': 'presentation_only', 'presentation': visual})
        ability = {'identity': ident(key), 'kind': 'spell', 'range_tiles': range_tiles, **geometry, 'effects': effects}
        if combat.get('area'):
            ability['area'] = {'matrix': {k: self.area_rows(v) for k, v in combat['area'].items() if v}}
        if 'chain' in combat:
            count, distance, backtracking = combat['chain']
            ability['chain'] = {'max_targets': int(count), 'range_tiles': int(distance), 'backtracking': bool(backtracking)}
            if params.get('COMBAT_PARAM_CHAIN_EFFECT'):
                ability['chain']['chain_asset_binding'] = asset(self.visual('@' + params['COMBAT_PARAM_CHAIN_EFFECT'], 'effect')[0])
        deps['abilities'].append(ability)
        return uses_magnitude

    @staticmethod
    def area_rows(rows):
        cells = {0: '.', 1: 'x', 2: 'c', 3: 'C'}
        if not isinstance(rows, list) or not all(isinstance(r, list) and all(v in cells for v in r) for r in rows):
            raise SpellUnresolved(f'area {rows!r} is not a 0-3 matrix.')
        text = [''.join(cells[v] for v in row) for row in rows]
        if len({len(r) for r in text}) != 1 or sum(r.count('c') + r.count('C') for r in text) != 1:
            raise SpellUnresolved('area matrix is not rectangular with one centre.')
        return text

    def script_condition(self, condition, deps, formula_key, notes):
        """Effect body for a recorded Condition, or None when it has no effect in Canary."""
        enums = self.spell_scripts.enums
        kind = condition['type']
        if kind not in enums['ConditionType_t']:
            raise SpellUnresolved(f'condition type {kind!r} is not a Canary constant.')
        params, damages, formula = {}, [], None
        for method, args in condition['calls']:
            if method == 'setParameter' and len(args) >= 2:
                if args[0] not in enums['ConditionParam_t']:
                    notes.append(f'{args[0]} is not a Canary constant (nil), so that setParameter has no effect.')
                    continue
                params[args[0]] = args[1]
            elif method == 'addDamage' and len(args) >= 3:
                damages.append(args[:3])
            elif method == 'setFormula':
                formula = args
            else:
                raise SpellUnresolved(f'condition {kind} call {method}{tuple(args)}.')
        name = kind[len('CONDITION_'):].lower()
        ticks = params.pop('CONDITION_PARAM_TICKS', None)
        delayed = params.pop('CONDITION_PARAM_DELAYED', 0)
        params.pop('CONDITION_PARAM_SUBID', None)
        if damages:
            if params or any(v >= 0 for _, _, v in damages):
                raise SpellUnresolved(f'condition {kind} with parameters {sorted(params)} or non-damaging ticks.')
            return {'operation': 'condition', 'condition': {'type': name, 'lifetime': 'damage_schedule', 'damage_over_time': {
                'tick_profile': 'fixed', 'first_tick': 'after_interval' if delayed else 'immediate',
                'fixed_ticks': [{'count': int(r), 'interval_ms': int(ms), 'amount': abs(int(v))} for r, ms, v in damages]}}}
        if kind in ('CONDITION_POISON', 'CONDITION_FIRE', 'CONDITION_ENERGY', 'CONDITION_BLEEDING', 'CONDITION_DROWN',
                    'CONDITION_FREEZING', 'CONDITION_DAZZLED', 'CONDITION_CURSED'):
            notes.append(f'{kind} without addDamage never starts (' + RULES['zero_condition'] + ').')
            return None
        if not ticks:
            raise SpellUnresolved(f'condition {kind} without CONDITION_PARAM_TICKS.')
        body = {'operation': 'condition', 'duration_ms': int(ticks), 'condition': {'type': name, 'lifetime': 'fixed_duration'}}
        if kind == 'CONDITION_ATTRIBUTES':
            modifiers = []
            for param, value in sorted(params.items()):
                attribute = param[len('CONDITION_PARAM_'):]
                percent = attribute.endswith('PERCENT')
                modifiers.append({'attribute': attribute[:-7].lower() if percent else attribute.lower(),
                                  'mode': 'percent_of_base' if percent else 'add', 'value': int(value)})
            if not modifiers:
                notes.append('CONDITION_ATTRIBUTES with no recognised attribute changes nothing.')
                return None
            body['condition']['attribute_modifiers'] = modifiers
        elif kind in ('CONDITION_PARALYZE', 'CONDITION_HASTE'):
            if formula is None or params:
                raise SpellUnresolved(f'{kind} without setFormula or with {sorted(params)}.')
            low_a, low_b, high_a, high_b = formula
            if kind == 'CONDITION_PARALYZE' and (low_a < 0 or high_a < 0):
                notes.append(f'setFormula{tuple(formula)} is read by Canary as the new speed (condition.cpp ConditionSpeed: '
                             'speedDelta = formula - base); negative factors clamp paralyze to speed 40.')
                low_a, low_b, high_a, high_b = 0, 40, 0, 40
            if min(low_a, high_a) < 0:
                raise SpellUnresolved(f'{kind} setFormula{tuple(formula)} has negative factors.')
            deps['formulas'].append({'identity': ident(formula_key), 'kind': 'speed_modifier', 'speed': {
                'minimum_multiplier': ratio(low_a), 'minimum_offset': int(low_b),
                'maximum_multiplier': ratio(high_a), 'maximum_offset': int(high_b)}})
            body['condition']['speed_formula'] = ref('Formula', formula_key)
        elif params or formula:
            raise SpellUnresolved(f'condition {kind} parameters {sorted(params)} have no authoring field.')
        return body

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

        visual_notes = []

        def presentation():
            value = {}
            if spell.get('effect') not in (None, False):
                key, visual_note = self.visual(spell['effect'], 'effect')
                value['impact_asset_binding'] = asset(key)
                visual_notes.append(visual_note)
            shoot = spell.get('shootEffect') or spell.get('shooteffect')
            if shoot:
                key, visual_note = self.visual(shoot, 'missile')
                value['projectile_asset_binding'] = asset(key)
                visual_notes.append(visual_note)
            return value

        def add_effect(suffix, body):
            key = base.format('effect') + suffix
            deps['effects'].append({'identity': ident(key), **body})
            effects.append(ref('Effect', key))

        note = ''
        if name == 'melee':
            if spell.get('attack', 0) > 0 and spell.get('skill', 0) > 0:
                formula_n[0] += 1
                key = base.format('formula') + f'-{formula_n[0]}'
                deps['formulas'].append({'identity': ident(key), 'kind': 'melee_attack_skill',
                                         'melee': {'attack': spell['attack'], 'skill': spell['skill']}})
                formula = ref('Formula', key)
            else:
                formula = formula_range(spell.get('minDamage', 0), spell.get('maxDamage', 0))
            add_effect('', {'operation': 'damage', 'damage_type': 'physical', 'formula': formula, 'mitigated_by': ['armor', 'shield'],
                            **({'presentation': presentation()} if presentation() else {})})
            note = RULES['armor_melee'] + '. '
            geometry = {'needs_target': True, 'needs_direction': False}
            kind, range_tiles = 'melee', 1
        elif name in ('combat', *FIELD_ITEMS):
            if name == 'combat':
                kind = spell.get('type')
                if isinstance(kind, str) and kind.startswith('@COMBAT_') and kind[8:] in DAMAGE:
                    damage = DAMAGE[constant(kind, 'COMBAT_')]
                else:
                    undefined = (f'combat entry with type {kind!r}: a missing or undefined constant leaves Canary '
                                 'MonsterSpell.combatType at COMBAT_UNDEFINEDDAMAGE, which has no authoring damage type.')
                    damage, wiki_note = self.undefined_damage_from_wiki(spell)
                    if damage is None:
                        return 'UNRESOLVED', f'{undefined} {wiki_note}'
                    note = f'{undefined} {wiki_note} '
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
                note += RULES['area_effect'] + '. '
            if body.get('damage_type') == 'physical' and body['operation'] == 'damage':
                body['mitigated_by'] = ['armor']
                note += RULES['armor_physical'] + '. '
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
        elif name in ('speed', 'outfit', 'invisible', 'drunk', 'strength', 'effect'):
            visual = presentation()
            area = spell.get('radius', 0) > 1 or spell.get('length') or spell.get('spread')
            if area and spell.get('effect') is None:
                visual['impact_asset_binding'] = asset('canary.appearance:effect/poff')
                note = RULES['area_effect'] + '. '
            duration = spell.get('duration') or 10000
            body = None
            if name == 'speed':
                change = max(spell.get('speedChange', 0), -1000)
                multiplier = Fraction(1000 + change, 1000)
                formula_n[0] += 1
                key = base.format('formula') + f'-{formula_n[0]}'
                deps['formulas'].append({'identity': ident(key), 'kind': 'speed_modifier', 'speed': {
                    'minimum_multiplier': fraction_ratio(multiplier / 2), 'minimum_offset': 40,
                    'maximum_multiplier': fraction_ratio(multiplier), 'maximum_offset': 40}})
                body = {'operation': 'condition', 'duration_ms': duration,
                        'condition': {'type': 'haste' if change > 0 else 'paralyze', 'lifetime': 'fixed_duration',
                                      'speed_formula': ref('Formula', key)}}
                note += RULES['speed'] + '. '
            elif name == 'outfit':
                if spell.get('outfitMonster'):
                    target_ref = ref('Creature', f'canary:creature/{slug(spell["outfitMonster"])}')
                    transform = {'creature': target_ref}
                else:
                    target_ref = ref('Item', f'canary:item/{spell["outfitItem"]}')
                    transform = {'item': target_ref}
                self.pending_definitions.add((target_ref['family'], target_ref['key']))
                body = {'operation': 'appearance_transform', 'appearance_transform': transform, 'duration_ms': duration}
                note += RULES['fixed_conditions'] + '. '
            elif name in ('invisible', 'drunk'):
                body = {'operation': 'condition', 'duration_ms': duration,
                        'condition': {'type': name, 'lifetime': 'fixed_duration'}}
                note += RULES['fixed_conditions'] + '. '
            elif visual:
                body = {'operation': 'presentation_only'}
                note += RULES['inert_spells'] + '. '
            elif name in ('strength', 'effect'):
                return 'OMIT', RULES['inert_spells'] + ' (D14).'
            else:
                return None
            if visual:
                body['presentation'] = visual
            add_effect('', body)
            geometry = cast_geometry(length=spell.get('length', 0), spread=spell.get('spread', 0),
                                     radius=spell.get('radius'), target=bool(spell.get('target', False)))
            kind, range_tiles = 'spell', spell.get('range', 0)
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
        if condition_type and max(low, high) == 0:
            note += RULES['zero_condition'] + '. '
            condition_type = None
            if not effects:
                visual = presentation()
                if not visual:
                    return 'OMIT', note.strip()
                add_effect('', {'operation': 'presentation_only', 'presentation': visual})
        if condition_type:
            add_effect('-condition', {'operation': 'condition', 'condition': {'type': condition_type, 'lifetime': 'damage_schedule',
                       'damage_over_time': {'total_damage_range': {'minimum': min(low, high), 'maximum': max(low, high)},
                                            'tick_interval_ms': tick,
                                            'initial_tick': {'mode': 'fixed', 'amount': start} if start else {'mode': 'automatic'},
                                            'tick_profile': 'decreasing', 'first_tick': 'after_interval'}}})
            note += RULES['condition'] + '. '
        ability_key = base.format('ability')
        ability = {'identity': ident(ability_key), 'kind': kind, 'range_tiles': range_tiles, **geometry, 'effects': effects}
        deps['abilities'].append(ability)
        return ability_key, note + ''.join(dict.fromkeys(visual_notes))

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
        if 'decayto' in attributes and int(attributes.get('duration', 0)) > 0:
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
    parser.add_argument('--batch', choices=sorted(BATCHES), action='append', help='default: every batch')
    parser.add_argument('--out', type=Path, default=ROOT / 'samples', help='parent directory of the batch directories')
    args = parser.parse_args()
    objects = load_appearance_objects(args.canary / 'data/items/appearances.dat')
    items = load_items_xml(args.canary / 'data/items/items.xml')
    names, index = name_index(objects, items)
    converter = Converter(args.canary, objects, items, names, index)
    for batch in args.batch or sorted(BATCHES):
        write_batch(converter, args.canary, args.out / batch, BATCHES[batch])


def write_batch(converter, canary, out, batch):
    sources = []
    reference = ROOT / 'samples' / out.name / WIKI_REFERENCE
    converter.wiki = ({m['monster']: m for m in json.loads(reference.read_text(encoding='utf-8'))['monsters']}
                      if reference.exists() else {})
    for relative in batch:
        converter.pending_definitions = set()
        s, monster, deps, catalog, manifest, source = converter.convert(relative)
        for family, key in sorted(converter.pending_definitions):
            local = {i['identity']['key'] for i in deps['items']}
            if ref(family, key) not in catalog['definitions'] and key != monster['creature']['identity']['key'] and key not in local:
                catalog['definitions'].append(ref(family, key))
        target = out / s
        target.mkdir(parents=True, exist_ok=True)
        for filename, value in (('monster.json', monster), ('dependencies.json', deps), ('catalog.json', catalog), ('manifest.json', manifest)):
            (target / filename).write_text(json.dumps(value, ensure_ascii=False, indent=2) + '\n', encoding='utf-8', newline='\n')
        sources.append({'monster': s, **source})
    shared = {'repository': REPOSITORY, 'revision': REVISION, 'rules': RULES,
              'shared_sources': {p: blob_id((canary / p).read_bytes()) for p in
                                 ('data/items/items.xml', 'data/items/appearances.dat', 'data/scripts/lib/register_monster_type.lua')},
              'monsters': sources}
    (out / 'sources.json').write_text(json.dumps(shared, ensure_ascii=False, indent=2) + '\n', encoding='utf-8', newline='\n')
    print(json.dumps({'monsters': len(sources), 'out': str(out)}))


if __name__ == '__main__':
    main()
