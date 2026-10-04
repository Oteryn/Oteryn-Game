"""Bounded source observations for nineteen unresolved inline combat slots.

Undefined donor names stay undefined; this packet never repairs donor intent or
qualifies runtime execution. Full pinned helper files are provenance evidence.
"""
import copy
from functools import lru_cache
import gzip
import hashlib
import json
import os
from pathlib import Path
import subprocess

PINS = {'canary': '04b83b512114bfd888000d6e1433ed8ecaec7c5b',
        'crystal': '00ce02a57ca5a12e48f32a3476e37471167e4c3f'}
SOURCE_ROOT = Path('/workspace/spell-sources')
LINKS = Path(__file__).resolve().parents[3] / 'docs/reference/spells/r37-source-closure/unresolved-monster-spell-links.jsonl.gz'
LINKS_SHA256 = 'db2c17a6825a7caa33b2fe9a06504d22a8f48a9af175c1ed417e57c93b2b052d'
UNDEFINED_NAMES = {'@COMBAT_LIFEDRAINDAMAGE', '@COMBAT_MANADRAINDAMAGE'}
PROOF_PATHS = {
    'data/scripts/lib/register_monster_type.lua': 'readSpell: conditional type setter and declaration transfer',
    'src/creatures/monsters/monsters.hpp': 'MonsterSpell member defaults',
    'src/creatures/monsters/monsters.cpp': 'Monsters::deserializeSpell: bounds, area, type and condition construction',
    'src/lua/functions/creatures/monster/monster_spell_functions.cpp': 'MonsterSpellFunctions Lua numeric and boolean setters',
    'src/lua/functions/core/game/lua_enums.cpp': 'registered valid drain enums; no typo aliases',
    'src/creatures/creatures_definitions.hpp': 'CombatType_t values',
    'src/map/map_const.hpp': 'viewport limit for spell range cap',
}


@lru_cache(maxsize=32)
def read(donor, path):
    return subprocess.check_output(['git', '-C', str(SOURCE_ROOT / donor), 'show',
                                    PINS[donor] + ':' + path],
                                   env={**os.environ, 'GIT_NO_LAZY_FETCH': '1'})


@lru_cache(maxsize=2)
def source_proofs(donor):
    files = {path: read(donor, path) for path in PROOF_PATHS}
    checks = {
        'data/scripts/lib/register_monster_type.lua': ['if incomingLua.type then', 'spell:setCombatType(incomingLua.type)'],
        'src/creatures/monsters/monsters.hpp': ['conditionType = CONDITION_NONE', 'combatType = COMBAT_UNDEFINEDDAMAGE'],
        'src/creatures/monsters/monsters.cpp': ['std::min(spell->minCombatValue, spell->maxCombatValue)',
            'std::max(spell->minCombatValue, spell->maxCombatValue)',
            'combatPtr->setParam(COMBAT_PARAM_TYPE, spell->combatType)',
            'if (spell->conditionType != CONDITION_NONE)', 'spell->needDirection = true'],
        'src/lua/functions/core/game/lua_enums.cpp': ['COMBAT_UNDEFINEDDAMAGE', 'COMBAT_LIFEDRAIN', 'COMBAT_MANADRAIN'],
        'src/map/map_const.hpp': ['MAP_MAX_CLIENT_VIEW_PORT_X = 8', 'MAP_MAX_CLIENT_VIEW_PORT_X + 3'],
    }
    for path, markers in checks.items():
        text = files[path].decode()
        if any(marker not in text for marker in markers):
            raise ValueError('current inline helper shape differs: ' + path)
    enum_text = files['src/lua/functions/core/game/lua_enums.cpp'].decode()
    if any(symbol[1:] in enum_text for symbol in UNDEFINED_NAMES):
        raise ValueError('previously undefined donor enum became registered')
    return [{'source': donor, 'revision': PINS[donor], 'path': path,
             'scope': scope, 'sha256': hashlib.sha256(raw).hexdigest()}
            for path, raw in files.items() for scope in [PROOF_PATHS[path]]]


@lru_cache(maxsize=1)
def exact_cohort():
    encoded = LINKS.read_bytes()
    if hashlib.sha256(encoded).hexdigest() != LINKS_SHA256:
        raise ValueError('immutable inline slot links differ')
    rows = [json.loads(line) for line in gzip.decompress(encoded).splitlines()]
    return {json.dumps(row['slot_identity'], sort_keys=True): row
            for row in rows if row['registered_source'] is None}


def build_inline(slot):
    original = exact_cohort().get(json.dumps(slot['slot_identity'], sort_keys=True))
    if original is None or any(slot.get(key) != original[key] for key in (
            'source', 'monster_source', 'source_parameters', 'original_slot_sha256')):
        raise ValueError('exact inline slot source identity or declarations differ')
    donor = slot['source']
    raw = slot['source_parameters']
    if donor not in PINS or slot['monster_source']['revision'] != PINS[donor]:
        raise ValueError('inline source revision differs')
    if slot.get('registered_source') is not None or raw.get('name') != 'combat':
        raise ValueError('outside bounded inline combat cohort')
    raw_type = raw.get('type')
    if raw_type is not None and raw_type not in UNDEFINED_NAMES:
        raise ValueError('outside missing or undefined combat type cohort')
    if 'condition' in raw:
        raise ValueError('condition closure requires a different cohort')
    allowed = {'name','type','chance','interval','minDamage','maxDamage','range','target','effect','shootEffect','length','spread','radius'}
    if set(raw) - allowed:
        raise ValueError('unrepresented inline declaration')
    if type(raw.get('minDamage')) is not int or type(raw.get('maxDamage')) is not int:
        raise ValueError('exact declared integer damage bounds required')
    source_file = read(donor, slot['monster_source']['path'])
    if hashlib.sha256(source_file).hexdigest() != slot['monster_source']['sha256']:
        raise ValueError('monster source hash differs')
    length, radius = raw.get('length', 0), raw.get('radius', 0)
    area_steps = []
    if length > 0:
        area_steps.append({'shape': 'length_spread', 'length': length, 'spread': max(0, raw.get('spread', 0))})
    if radius > 0:
        area_steps.append({'shape': 'radius', 'radius': radius})
    return {
        'slot_identity': copy.deepcopy(slot['slot_identity']),
        'source_parameters': copy.deepcopy(raw),
        'source_type_resolution': {'declared_type': raw_type,
            'status': 'absent_type_preserves_default' if raw_type is None else 'unregistered_symbol_preserves_default',
            'effective_type': 'COMBAT_UNDEFINEDDAMAGE',
            'valid_drain_alias_substitution_used': False,
            'unregistered_symbol_scope': 'pinned native enum registry; no external Lua environment overrides qualified'},
        'normalized_combat': {'name': 'combat', 'interval_ms': raw.get('interval', 2000),
            'chance_percent': min(raw.get('chance', 100), 100), 'range_tiles': min(raw.get('range', 0), 22),
            'min_combat_value': min(raw['minDamage'], raw['maxDamage']),
            'max_combat_value': max(raw['minDamage'], raw['maxDamage']),
            'need_target': raw.get('target', False), 'need_direction': length > 0,
            'area_construction_order': area_steps, 'condition_type': 'CONDITION_NONE',
            'condition_appended': False, 'physical_block_armor_branch': False,
            'healing_nonaggressive_branch': False,
            'combat_formula': 'COMBAT_FORMULA_DAMAGE',
            'effect': raw.get('effect', '@CONST_ME_NONE'),
            'shoot_effect': raw.get('shootEffect', '@CONST_ANI_NONE')},
        'source_proofs': copy.deepcopy(source_proofs(donor)),
        'limitations': ['Undefined damage is a donor source default, not verified intended life/mana drain or ice/energy element.',
            'Source metadata bounds and type are retained; final health/resource execution, events, immunity and native admission remain unqualified.',
            'Sound helper overwrite behavior is preserved in the existing source inventory and is not recomputed by this bounded helper.',
            'No melee, condition or unknown-name source exists in these nineteen selected slots.'],
        'complete_spell_candidate': False, 'runtime_activation': False, 'native_admission': False,
    }


def strict_schema(records):
    """Exact source-evidence schema; no free-form executable extension points."""
    def fixed(value):
        if isinstance(value, dict):
            return {'type': 'object', 'additionalProperties': False,
                    'properties': {key: fixed(item) for key, item in value.items()},
                    'required': list(value)}
        if isinstance(value, list):
            if not value:
                return {'type': 'array', 'maxItems': 0}
            return {'type': 'array', 'prefixItems': [fixed(item) for item in value],
                    'minItems': len(value), 'maxItems': len(value)}
        return {'const': value}
    return {'$schema': 'https://json-schema.org/draft/2020-12/schema',
            '$id': 'urn:oteryn:monster-inline-source:1', 'oneOf': [fixed(row) for row in records]}
