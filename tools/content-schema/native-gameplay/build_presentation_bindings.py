"""Build the SPELL-PRESENT-0 §3 presentation bindings from the pinned Canary source.

Writes two content files:
- content/presentations/bindings/hit-table.json: §5's colour and hit effect per damage type and
  race, parsed from Canary `Game::combatGetTypeInfo` plus the heal and mana rows of
  `combatChangeHealth` and `combatChangeMana`.
- content/presentations/bindings/client-ids.json: every effect, missile and sound key used in
  `content/` (the hit table included) mapped to its 15.30 client id, with the enum member that
  supplies it (build_spell_cues.py resolves enums and their Lua registration).

`validate_presentation_bindings.py` checks both files against `content/` and appearances.dat
without the source checkout.
"""
import argparse
import hashlib
import json
import re
import subprocess
from pathlib import Path

import build_spell_cues as cues

PIN = cues.PIN
GAME = 'src/game/game.cpp'
COLOR_HEADER = 'src/utils/utils_definitions.hpp'
ROOT = Path(__file__).resolve().parents[3]
CONTENT = ROOT / 'content'
BINDINGS = CONTENT / 'presentations' / 'bindings'
CLIENT_IDS = BINDINGS / 'client-ids.json'
HIT_TABLE = BINDINGS / 'hit-table.json'
CREATURE_RACES = BINDINGS / 'creature-races.json'
# The one key whose id the runtime weapon policy resolves (build_spell_cues.py).
DYNAMIC = ['canary.appearance:missile/weapontype']
KEY = re.compile(r'(canary\.)?(sound:(effect/[0-9]+|[a-z][a-z0-9_]*)|appearance:(effect|missile)/[a-z0-9_]+)')

DAMAGE = {'COMBAT_PHYSICALDAMAGE': 'physical', 'COMBAT_ENERGYDAMAGE': 'energy',
          'COMBAT_EARTHDAMAGE': 'earth', 'COMBAT_DROWNDAMAGE': 'drowning',
          'COMBAT_FIREDAMAGE': 'fire', 'COMBAT_ICEDAMAGE': 'ice', 'COMBAT_HOLYDAMAGE': 'holy',
          'COMBAT_DEATHDAMAGE': 'death', 'COMBAT_LIFEDRAIN': 'life_drain'}
# Canary damage types Oteryn content has no damage type for.
NOT_OTERYN = {'COMBAT_AGONYDAMAGE', 'COMBAT_NEUTRALDAMAGE'}
COLORS = {'TEXTCOLOR_RED': 'red', 'TEXTCOLOR_LIGHTGREEN': 'light_green',
          'TEXTCOLOR_LIGHTGREY': 'light_grey', 'TEXTCOLOR_ORANGE': 'orange',
          'TEXTCOLOR_PURPLE': 'purple', 'TEXTCOLOR_SKYBLUE': 'sky_blue',
          'TEXTCOLOR_YELLOW': 'yellow', 'TEXTCOLOR_DARKRED': 'dark_red',
          'TEXTCOLOR_LIGHTBLUE': 'light_blue', 'TEXTCOLOR_BLUE': 'blue',
          'TEXTCOLOR_PASTELRED': 'pastel_red'}
# Monster files name these effects, but neither source defines them; Lua reads nil and shows
# no effect, so the binding is "no effect".
UNDEFINED = {
    'canary.appearance:effect/poison': 'CONST_ME_POISON is undefined in both sources; harpy.lua shows no effect',
    'canary.appearance:effect/redspark': 'CONST_ME_REDSPARK is undefined in both sources; izcandar_champion_of_summer.lua shows no effect',
}
# Crystal (the 15.30 Canary fork the Crystal creature bindings cite) defines the newer effects.
CRYSTAL_PIN = '00ce02a57ca5a12e48f32a3476e37471167e4c3f'
RACES = ['venom', 'blood', 'undead', 'fire', 'energy', 'ink', 'chocolate', 'candy']


def function_body(text, signature):
    start = text.find(signature)
    if start < 0 or text.find(signature, start + 1) >= 0:
        raise ValueError('missing or ambiguous source function ' + signature)
    depth, i = 0, text.index('{', start)
    for j in range(i, len(text)):
        depth += {'{': 1, '}': -1}.get(text[j], 0)
        if depth == 0:
            return text[i:j + 1], text.count('\n', 0, start) + 1
    raise ValueError('unterminated source function ' + signature)


def effect_key(member):
    if member == 'CONST_ME_NONE':
        return None
    if not member.startswith('CONST_ME_'):
        raise ValueError('unexpected effect ' + member)
    return 'canary.appearance:effect/' + member.removeprefix('CONST_ME_').lower()


def color_name(member):
    if member == 'TEXTCOLOR_NONE':
        return None
    if member not in COLORS:
        raise ValueError('colour outside the PRESENTATION_V1 enum ' + member)
    return COLORS[member]


def cases(body):
    """`case X: ... color = C; effect = E;` arms, in source order."""
    arms = re.split(r'\bcase\s+([A-Z_]+)\s*:', body)
    out = []
    for name, arm in zip(arms[1::2], arms[2::2]):
        color = re.search(r'\bcolor\s*=\s*(TEXTCOLOR_[A-Z_]+)\s*;', arm)
        effect = re.search(r'\beffect\s*=\s*(CONST_ME_[A-Z_]+)\s*;', arm)
        if color is None or effect is None:
            raise ValueError('source arm without colour and effect ' + name)
        out.append((name, color[1], effect[1]))
    return out


def hit_table(game):
    text = cues.without_comments(game.decode())
    body, line = function_body(text, 'void Game::combatGetTypeInfo(')
    physical, rest = body.split('case COMBAT_ENERGYDAMAGE', 1)
    race_arms = cases(physical.split('switch (target->getRace())', 1)[1])
    races = []
    for name, color, effect in race_arms:
        if not name.startswith('RACE_') or name.removeprefix('RACE_').lower() not in RACES:
            raise ValueError('unexpected race ' + name)
        races.append({'race': name.removeprefix('RACE_').lower(), 'color': color_name(color),
                      'hit_effect': effect_key(effect), 'source_member': name})
    if sorted(r['race'] for r in races) != sorted(RACES):
        raise ValueError('source physical arms do not cover the races')
    types = []
    for name, color, effect in cases('case COMBAT_ENERGYDAMAGE' + rest):
        if name in NOT_OTERYN:
            continue
        if name not in DAMAGE:
            raise ValueError('unmapped source damage type ' + name)
        types.append({'damage_type': DAMAGE[name], 'color': color_name(color),
                      'hit_effect': effect_key(effect), 'source_member': name})
    heal, heal_line = function_body(text, 'bool Game::combatChangeHealth(')
    mana, mana_line = function_body(text, 'bool Game::combatChangeMana(')
    if 'message.primary.color = TEXTCOLOR_PASTELRED;' not in heal:
        raise ValueError('source heal colour unavailable')
    # Mana shield absorption: the loseenergy effect and a blue number (combatChangeHealth); a
    # mana drain: a blue number (combatChangeMana). §5 gives both the loseenergy effect.
    if not re.search(r'addMagicEffect\([^;]*CONST_ME_LOSEENERGY\);[^}]*message\.primary\.color = TEXTCOLOR_BLUE;', heal, re.S):
        raise ValueError('source mana shield presentation unavailable')
    if 'message.primary.color = TEXTCOLOR_BLUE;' not in mana:
        raise ValueError('source mana drain colour unavailable')
    types += [
        {'damage_type': 'mana_drain', 'color': 'blue', 'hit_effect': effect_key('CONST_ME_LOSEENERGY'),
         'source_member': 'Game::combatChangeHealth mana shield; Game::combatChangeMana'},
        {'damage_type': 'healing', 'color': 'pastel_red', 'hit_effect': None,
         'source_member': 'Game::combatChangeHealth'},
        {'damage_type': 'untyped', 'color': None, 'hit_effect': None, 'source_member': 'default'},
    ]
    return {
        'schema': 'OTERYN_PRESENTATION_HIT_TABLE/v1',
        'decision': 'SPELL-PRESENT-0 §3, §5',
        'source': {'repository': 'opentibiabr/canary', 'revision': PIN, 'path': GAME,
                   'sha256': hashlib.sha256(game).hexdigest(),
                   'functions': [{'name': 'Game::combatGetTypeInfo', 'line': line},
                                 {'name': 'Game::combatChangeHealth', 'line': heal_line},
                                 {'name': 'Game::combatChangeMana', 'line': mana_line}]},
        'player_race': 'blood',
        'physical_by_race': sorted(races, key=lambda r: r['race']),
        'by_damage_type': sorted(types, key=lambda r: r['damage_type']),
    }


def content_keys(content, extra):
    keys = set()

    def walk(value):
        if isinstance(value, dict):
            for item in value.values():
                walk(item)
        elif isinstance(value, list):
            for item in value:
                walk(item)
        elif isinstance(value, str) and KEY.fullmatch(value):
            keys.add(value)

    for path in sorted(content.rglob('*.json')):
        if path in (CLIENT_IDS, HIT_TABLE, CREATURE_RACES):
            continue
        walk(json.loads(path.read_bytes()))
    walk(extra)
    return keys


def resolve(key, sources):
    """(kind, member, value, source index) from the first source that defines and registers it."""
    short = key.removeprefix('canary.')
    numeric = short.startswith('sound:effect/')
    if numeric:
        kind = 'sound'
    elif short.startswith('sound:'):
        kind, member = 'sound', short.split(':')[1].upper()
    elif short.startswith('appearance:effect/'):
        kind, member = 'effect', 'CONST_ME_' + short.split('/')[1].upper()
    else:
        kind, member = 'missile', 'CONST_ANI_' + short.split('/')[1].upper()
    table = 'projectile' if kind == 'missile' else kind
    for index, (values, registered) in enumerate(sources):
        if numeric:
            value = int(short.split('/')[1])
            members = sorted(m for m, v in values[table].items() if v == value and m in registered[table])
            if members:
                return kind, members[0], value, index
        elif member in values[table] and member in registered[table]:
            return kind, member, values[table][member], index
    return kind, None, None, None


def client_ids(keys, sources, provenance):
    records = []
    for key in sorted(keys):
        if key in DYNAMIC:
            continue
        kind, member, value, index = resolve(key, sources)
        if member is None:
            if key not in UNDEFINED:
                raise ValueError('unknown or unregistered source member for ' + key)
            records.append({'key': key, 'kind': kind, 'client_id': None,
                            'source_member': None, 'note': UNDEFINED[key]})
            continue
        if key in UNDEFINED:
            raise ValueError('key listed as undefined resolves: ' + key)
        records.append({'key': key, 'kind': kind, 'source': provenance[index]['repository'],
                        'source_member': member, 'client_id': value})
    return {
        'schema': 'OTERYN_PRESENTATION_CLIENT_IDS/v1',
        'decision': 'SPELL-PRESENT-0 §3',
        'client_version': '15.30',
        'sources': provenance,
        'dynamic_keys': DYNAMIC,
        'records': records,
    }


def crystal_enums(blobs):
    """Crystal registers each magic effect by name instead of Canary's magic_enum loop."""
    utils, creatures, lua = (b.decode() for b in blobs)
    tables = {
        'effect': cues.enum_values(utils, 'MagicEffectClasses'),
        'projectile': cues.enum_values(utils, 'ShootType_t'),
        'sound': cues.enum_values(creatures, 'SoundEffect_t'),
    }
    lua = cues.without_comments(lua)
    if not re.search(r'soundNamespace\s*=\s*"SOUND_EFFECT_TYPE_"\s*;', lua):
        raise ValueError('source sound Lua namespace unavailable')
    registered = {
        'effect': set(re.findall(r'\bregisterEnum\(L,\s*(CONST_ME_[A-Z_0-9]+)\s*\)', lua)),
        'projectile': set(re.findall(r'\bregisterEnum\(L,\s*(CONST_ANI_[A-Z_0-9]+)\s*\)', lua)),
        'sound': set(re.findall(r'\bregisterEnumNamespace\(L,\s*soundNamespace,\s*SoundEffect_t::([A-Z_0-9]+)\s*\)', lua)),
    }
    return tables, registered


def qualified(source, pin, files, enums=cues.qualified_cue_enums):
    revision = subprocess.check_output(['git', '-C', str(source), 'rev-parse', 'HEAD'], text=True).strip()
    if revision != pin:
        raise ValueError('unqualified source revision ' + str(source))
    show = lambda p: subprocess.check_output(['git', '-C', str(source), 'show', pin + ':' + p])
    blobs = [show(p) for p in files]
    return show, blobs, enums(blobs)


def build(canary, crystal):
    show, blobs, canary_enums = qualified(canary, PIN, cues.FILES)
    _, crystal_blobs, crystal_tables = qualified(crystal, CRYSTAL_PIN, cues.FILES, crystal_enums)
    provenance = [{'repository': repository, 'revision': revision,
                   'files': [{'path': p, 'sha256': hashlib.sha256(b).hexdigest()}
                             for p, b in zip(cues.FILES, files)]}
                  for repository, revision, files in [('opentibiabr/canary', PIN, blobs),
                                                      ('zimbadev/crystalserver', CRYSTAL_PIN, crystal_blobs)]]
    hits = hit_table(show(GAME))
    keys = content_keys(CONTENT, hits)
    return client_ids(keys, [canary_enums, crystal_tables], provenance), hits


def dumps(value):
    return json.dumps(value, sort_keys=True, indent=2, ensure_ascii=False) + '\n'


if __name__ == '__main__':
    parser = argparse.ArgumentParser()
    parser.add_argument('--reference-engine', type=Path, required=True,
                        help='opentibiabr/canary checkout at ' + PIN)
    parser.add_argument('--crystal-engine', type=Path, required=True,
                        help='zimbadev/crystalserver checkout at ' + CRYSTAL_PIN)
    args = parser.parse_args()
    ids, hits = build(args.reference_engine, args.crystal_engine)
    HIT_TABLE.write_text(dumps(hits))
    CLIENT_IDS.write_text(dumps(ids))
