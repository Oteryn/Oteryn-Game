"""Prepare accepted typed fight draft fragments; never admit or qualify a runtime.

The packet is source evidence, not a parameter oracle. Each selected script must
match an entire small template. Existing Encounter samples supply fight scope;
new lifecycle, placement policy, exhaustion or native behaviour is never invented.
"""
import argparse
import hashlib
import json
import re
import subprocess
import tempfile
from pathlib import Path
import sys

ROOT = Path(__file__).resolve().parent
sys.path.insert(0, str(ROOT.parent / 'monster-authoring'))
import canary_batch as cb
import canary_encounters as ce
import prepare_custom_patterns as packets
import validate_encounter as ve
import validate_monster as vm
from jsonschema import Draft202012Validator
from lupa.luajit21 import LuaRuntime

BOUNDARIES = {
 'charge vortex': 'Ground identity and explicit revert-to-23049 differ from restore-original; map/item condition and placement proof missing.',
 'doctor marrow explosion': 'Captured-target position, distance-scaled damage, critical draw, repeated paralysis and callback master nil defect require accepted parameter contracts.',
 'energy pulse explosion': 'Combat plus unconditional caster removal needs a typed continuation/Encounter binding preserving caster damage context.',
 'eruption of destruction explosion': 'Delayed master healing, forced spawn, vocation filtering, tile iteration and removal require a complete accepted contract.',
 'foamsplash': 'Three independently scheduled Combat matrices need a typed sequence retaining the source variant/caster context.',
 "gaz'haragoth death": 'Delayed caster-position combat, four-vocation filter, tile iteration and two ordered messages need a complete accepted contract.',
 "gaz'haragoth summon": 'Mutable GazVariables cap/progress and spectator counts differ from caster summon_count; undefined sum ownership remains source uncertainty.',
 'generator': 'Typed core prepared; extended/forced placement and spawn-failure parity still require runtime qualification.',
 'glooth fairy healing': 'Health threshold plus regeneration condition SUBID gate and delayed heal needs a condition-specific cooldown contract.',
 'glooth-generator summon': 'Delayed spawn/say/remove needs retained triggering identity and source placement/failure semantics.',
 'greedy eye beam': 'Combat drown damage plus direct -1000 health loss for top-tile Poor Soul requires two different target filters and magnitudes.',
 'heal brain head': 'Fixed tile/top-creature proof and independent projectile/impact presentation cannot be replaced by all-role healing.',
 'icicle heal': 'Named-target damage is combined with spectator search and setTarget(string); source API defect/intended targeting remains open.',
 'lisa heal': 'Regeneration SUBID gate, 7% threshold, delay and ordered caster messages lack a complete accepted condition/cooldown contract.',
 'lisa skill reducer': 'Four-vocation branching and legacy per-tile creature enumeration cannot be replaced with one uniform attribute effect.',
 'maxxenteleport': 'Typed literal core prepared; native teleport failure/presentation sequence and whole fight admission remain unqualified.',
 'mazoran fire': 'Per-tile item exclusions, action IDs, forced summons and explicit restoration roster need map/Interaction contracts.',
 'megalomania blue': 'Zone scan is repeated per Combat tile, excludes ground409 and captures zone positions at load; cannot collapse to one area damage.',
 'minotaur cult prophet mass healing': 'One shared heal draw at script load must not become a per-cast or per-target Formula draw.',
 'omrafir beam': 'SUBID regeneration gate, delayed current-direction selection and ordered messages need a complete accepted condition/sequence contract.',
 'omrafir healing 2': 'Specific ground-item predicate, exact name and fractional health threshold require typed Item-presence reads.',
 'outburst explode': 'World storages, remembered health, delayed removal and mixed spectator teleport need fight/quest ownership and exact state bindings.',
 'plagirath bog': 'Immediate outfit plus delayed captured-target/range-gated combat requires a target-position/context continuation.',
 'ragiaz transform': 'Two existing creatures exchange current positions; capsule identity from fixed tile is not proven by its role.',
 'rotthingshaper': 'Two different Combat matrices execute in sequence; Ability.variants would incorrectly pick only one.',
 'rotthingwave': 'Two directional Combat matrices execute in sequence; Ability.variants would incorrectly pick only one.',
 'sapling explode': 'Combat plus 1ms scheduled removal lacks a complete reusable typed continuation contract.',
 'spider queen wrap': 'Player-only target, outfit, paralysis, delayed captured-player teleport and quest write need separate owners.',
 'summon challenge': 'doChallengeCreature duration8000 has no accepted monster Effect/taunt parameter contract.',
 'targetfirering': 'Undefined SHOOT_EFFECT maps nil key to CombatParam0 TYPE; source engine changes FIRE to UNDEFINEDDAMAGE. D25 intended wiki behaviour remains unknown.',
 'tenebris ultimate': 'Room player pull, move lock, four-vocation tile filter and delayed caster-position combat need a complete fight binding.',
 'the welter heal': 'First matching spectator consumption with two presentation branches and healing cannot become all-role remove/heal.',
 'the welter summon2': 'Counts nearby named creatures, not caster summons; source name case and forced placement need exact spectator-count semantics.',
 'time guardian': 'Existing-creature position exchange, current-health transfer and delayed spectator-dependent restoration need phase/identity proof.',
 'time guardian lost time': 'Typed floor/name/independent-offset spawn core prepared; occupied-tile failure, coordinate overflow and native placement parity remain unqualified. Other Time Guardian form exchanges stay unresolved.',
 'time guardiann': 'Existing-creature position exchange and one-sided health transfer differ from transformation and remembered-health semantics.',
 'tyrn heal': 'Health20% and regeneration SUBID88888 condition gate for15min need a complete typed condition/cooldown contract.',
 'walker skill reducer': 'Vocation predicates and legacy tile enumeration require per-target conditional attribute effects.',
 'zamulosh invisible': 'Fixed-room top-creature scan and condition application to named copies are not caster-only invisibility.',
 'zamulosh tp': 'Typed literal core prepared; native teleport failure/presentation sequence and whole fight admission remain unqualified.',
}
SELECTED = {'generator': 'professor_maxxen', 'maxxenteleport': 'professor_maxxen', 'zamulosh tp': 'zamulosh',
            'time guardian lost time': 'the_time_guardian'}
LOST_TIME_ROLES = {'the_freezing_time_guardian': 'lost_time', 'the_blazing_time_guardian': 'time_waster'}


def pinned(canary, path):
    return packets.pinned(canary, path)[0]


def normalized(text):
    # Formatting is insignificant outside strings; creature/spell identities
    # and spoken text inside quoted literals must remain byte-exact.
    return re.sub(r'"(?:\\.|[^"\\])*"|\'(?:\\.|[^\'\\])*\'|\s+',
                  lambda match: '' if match[0].isspace() else match[0], text)


def cast_body(text):
    match = re.search(r'function spell\.onCastSpell\(creature, var\)\s*(.*?)\nend', text, re.S)
    if not match:
        raise ValueError('whole selected onCastSpell template missing')
    return normalized(match[1])


def destinations(name, text):
    body = cast_body(text)
    if name == 'generator':
        expected = '''local rand = math.random(1, 4) local generators = generator[rand]
          local monster = Game.createMonster("glooth-generator", generators.pos, true, true)
          monster:say("THE GLOOTH GENERATOR CHARGES UP FOR A LETHAL EXPLOSION!", TALKTYPE_MONSTER_YELL) return'''
        if body != normalized(expected):
            raise ValueError('generator has actions outside the accepted exact template')
        points = [tuple(map(int, row)) for row in re.findall(r'Position\((\d+),\s*(\d+),\s*(\d+)\)', text)]
        if len(points) != 4 or len(set(points)) != 4:
            raise ValueError('generator needs exactly four source positions')
        return points
    prefix = 'creature:getPosition():sendMagicEffect(CONST_ME_POFF)creature:teleportTo(Position('
    suffix = '))creature:getPosition():sendMagicEffect(CONST_ME_TELEPORT)return'
    if not body.startswith(prefix) or not body.endswith(suffix):
        raise ValueError('teleport has actions outside the accepted exact template')
    coordinates = body[len(prefix):-len(suffix)]
    if name == 'maxxenteleport':
        match = re.fullmatch(r'math\.random\((\d+),(\d+)\),math\.random\((\d+),(\d+)\),(\d+)', coordinates)
        if not match:
            raise ValueError('teleport random bounds are not source literals')
        x0, x1, y0, y1, floor = map(int, match.groups())
        return [(x, y, floor) for x in range(x0, x1 + 1) for y in range(y0, y1 + 1)]
    if not re.fullmatch(r'\d+,\d+,\d+', coordinates):
        raise ValueError('teleport destination is not source literals')
    return [tuple(map(int, coordinates.split(',')))]


def ability(key, encounter=None, effects=None):
    # Same no-target/no-range flags as the accepted D45 bridge (absent source flags).
    return {'identity': cb.ident(key), 'kind': 'spell', 'range_tiles': 0,
            'needs_target': False, 'needs_direction': False,
            **({'encounter': ce.ref('Encounter', encounter)} if encounter else {'effects': effects})}


def lost_time_rules(text, position_text):
    """Exact registered source template, using only accepted D45/D46 vocabulary."""
    expected = '''local spell = Spell("instant")
function spell.onCastSpell(creature, var)
local pos = creature:getPosition()
if pos.z ~= 15 then return true end
if creature:getName():lower() == "the freezing time guardian" then
Game.createMonster("lost time", { x = creature:getPosition().x + math.random(-2, 2), y = creature:getPosition().y + math.random(-2, 2), z = creature:getPosition().z })
elseif creature:getName():lower() == "the blazing time guardian" then
Game.createMonster("time waster", { x = creature:getPosition().x + math.random(-2, 2), y = creature:getPosition().y + math.random(-2, 2), z = creature:getPosition().z })
end return true end
spell:name("time guardian lost time") spell:words("###439") spell:isAggressive(true)
spell:blockWalls(true) spell:needLearn(true) spell:needDirection(true) spell:register()'''
    if normalized(text) != normalized(expected):
        raise ValueError('lost time whole registered source template changed')
    if not all(re.search(r'uint16_t\s+' + axis + r'\s*=\s*0;', position_text) for axis in ('x', 'y')):
        raise ValueError('lost time pinned Position coordinate domain changed')
    rules = []
    for role, actor in LOST_TIME_ROLES.items():
        # Source draws x, then y independently. Each occupied tile stays a
        # candidate; offset_tiles would incorrectly choose only a free tile.
        choice = {'kind': 'one_of', 'branches': [{'weight': 1, 'actions': [
            {'kind': 'one_of', 'branches': [{'weight': 1, 'actions': [
                {'kind': 'spawn', 'creature': ce.ref('Creature', 'canary:creature/' + actor), 'count': 1,
                 'at': {'relative': {'x': x, 'y': y}}, 'owner': 'none', 'health': 'full'}]}
             for y in range(-2, 3)]}]} for x in range(-2, 3)]}
        rules.append({'key': role + '_casts_' + actor,
            'trigger': {'kind': 'ability_cast', 'role': role,
                        'ability': ce.ref('Ability', 'canary:ability/spell/time_guardian_lost_time')},
            'conditions': [{'kind': 'in_anchor', 'subject': {'role': role}, 'anchor': 'script_floor_15'}],
            'actions': [choice]})
    anchor = {'key': 'script_floor_15', 'kind': 'area',
              'description': 'Exact floor-only predicate over pinned uint16_t Position x/y domain; no assumed arena.',
              'location': {'boxes': [{'x': [0, 65535], 'y': [0, 65535], 'floor': 15}]}}
    return anchor, rules


def constant_oracle(canary):
    paths = ['src/creatures/creatures_definitions.hpp', 'src/utils/utils_definitions.hpp',
             'src/lua/functions/core/game/lua_enums.cpp', 'src/lua/functions/lua_functions_loader.hpp',
             'src/lua/functions/creatures/combat/combat_functions.cpp', 'src/creatures/combat/combat.cpp']
    sources = {path: pinned(canary, path).decode() for path in paths}
    snippets = []
    for path, enum in [(paths[0], 'CombatParam_t'), (paths[0], 'CombatType_t'), (paths[1], 'ShootType_t')]:
        match = re.search(r'enum ' + enum + r'(?:\s*:\s*\w+)?\s*\{.*?\};', sources[path], re.S)
        if not match:
            raise ValueError('source constant enum unavailable')
        snippets.append(match[0])
    if 'COMBAT_PARAM_SHOOT_EFFECT' in snippets[0]:
        raise ValueError('SHOOT_EFFECT source registration changed')
    required = [(paths[2], 'magic_enum::enum_values<CombatParam_t>()'),
                (paths[3], 'auto number = lua_tonumber(L, arg);'),
                (paths[4], 'Lua::getNumber<CombatParam_t>(L, 2)'),
                (paths[5], 'params.combatType = static_cast<CombatType_t>(value);')]
    if not all(phrase in sources[path] for path, phrase in required):
        raise ValueError('constant oracle source engine calls changed')
    code = '#include <cstdint>\n#include <cstdio>\n' + '\n'.join(snippets)
    code += '\nint main(){std::printf("%d %d %d %d %d\\n", COMBAT_PARAM_TYPE, CONST_ANI_FIRE, COMBAT_UNDEFINEDDAMAGE, COMBAT_FIREDAMAGE, COMBAT_PARAM_EFFECT);}\n'
    with tempfile.TemporaryDirectory() as tmp:
        cpp, exe = Path(tmp) / 'oracle.cpp', Path(tmp) / 'oracle'
        cpp.write_text(code)
        subprocess.run(['g++', '-std=c++20', str(cpp), '-o', str(exe)], check=True, capture_output=True)
        values = subprocess.run([str(exe)], check=True, capture_output=True, text=True).stdout.strip()
    if values != '0 4 4 1 1':
        raise ValueError('compiled source enum oracle no longer proves the source bug')
    # Execute the exact source declarations with numeric globals, rather than the
    # converter's symbolic missing-global stub. Only parameter calls are recorded.
    effects, _ = cb.load_effect_constants(canary / cb.EFFECT_CONSTANTS)
    lua = LuaRuntime(unpack_returned_tuples=True)
    lua.execute('''recorded = {}; calls = 0
      function Combat() return {setParameter=function(self,key,value)
        calls=calls+1; recorded[tonumber(key) or 0]=tonumber(value) or 0; return true end,
        setArea=function() end, execute=function() return true end} end
      function createCombatArea(area) return area end
      function Spell() return setmetatable({}, {__index=function() return function() end end}) end''')
    for name, value in [('COMBAT_PARAM_TYPE', 0), ('COMBAT_PARAM_EFFECT', 1), ('COMBAT_FIREDAMAGE', 1),
                        ('CONST_ANI_FIRE', 4), ('CONST_ME_EXPLOSIONHIT', effects['CONST_ME_EXPLOSIONHIT'])]:
        lua.globals()[name] = value
    script = 'data-otservbr-global/scripts/spells/monster/priestess_firering.lua'
    script_bytes = pinned(canary, script)
    lua.execute(script_bytes.decode())
    if lua.globals().calls != 3 or lua.globals().recorded[0] != 4:
        raise ValueError('exact source parameter calls no longer reproduce undefined damage')
    return {'classification': 'SOURCE_ENGINE_BUG_NOT_ADOPTED', 'compiled_enum_values': ' '.join(values.split()[:3]),
            'compiled_other_values': {'COMBAT_FIREDAMAGE': 1, 'COMBAT_PARAM_EFFECT': 1},
            'exact_source_lua_parameter_calls': 3, 'recorded_final_combat_type': 4,
            'source_script': {'path': script, 'blob_sha1': cb.blob_id(script_bytes)},
            'oracle_cpp_sha256': hashlib.sha256(code.encode()).hexdigest(),
            'mechanism': 'Missing enum global -> nil; lua_tonumber(nil)=0; key0=TYPE; projectile FIRE4 overwrites combat type with UNDEFINEDDAMAGE4.',
            'D25_intended_wiki_behaviour': 'UNKNOWN_NOT_ADOPTED', 'admission_authorized': False,
            'sources': [{'path': path, 'blob_sha1': cb.blob_id(text.encode())} for path, text in sources.items()]}


def prepare(canary, packet_path):
    packet = json.loads(packet_path.read_text())
    Draft202012Validator(json.loads(packets.SCHEMA.read_text())).validate(packet)
    for row in packet['spells']:
        actual = sum(binding['status'] in ('unresolved_semantics', 'unsupported_source_field')
                     for binding in row['current_monster_dispositions'])
        if row['current_open_references'] != actual:
            raise ValueError('custom spell open reference count mismatch: ' + row['spell'])
    open_rows = {row['spell'].lower(): row for row in packet['spells'] if row['current_open_references']}
    if set(open_rows) != set(BOUNDARIES):
        raise ValueError('current40 custom spell roster changed; review boundaries again')
    reference_count = sum(row['current_open_references'] for row in open_rows.values())
    if reference_count != 43:
        raise ValueError('current custom reference roster must contain exactly43 source bindings')
    for name in SELECTED:
        bindings = [r for r in open_rows[name]['current_monster_dispositions']
                    if r['status'] in ('unresolved_semantics', 'unsupported_source_field')]
        if name == 'time guardian lost time':
            expected = {(role, 'defenses[1]') for role in LOST_TIME_ROLES}
            actual = {(r['monster'], r['source_field']) for r in bindings}
            if len(bindings) != 2 or actual != expected:
                raise ValueError('lost time requires exactly two distinct source form schedules')
        elif len(bindings) != 1:
            raise ValueError('selected spell must have exactly one source schedule binding: ' + name)
    for row in open_rows.values():
        content = pinned(canary, row['source']['file'])
        if cb.blob_id(content) != row['source']['blob_sha1']:
            raise ValueError('source packet blob mismatch')
    candidates = {}
    schedules = []
    for fight in sorted(set(SELECTED.values())):
        baseline = ROOT / 'samples' / fight
        artifacts = {name: json.loads((baseline / name).read_text()) for name in ('encounter.json', 'manifest.json', 'catalog.json')}
        for source in artifacts['manifest.json']['sources']:
            if source['revision'] != ce.REVISION or cb.blob_id(pinned(canary, source['path'])) != source['blob_sha1']:
                raise ValueError('fight baseline source is not pinned')
        artifacts['dependencies.json'] = {key: [] for key in ('abilities', 'effects', 'formulas', 'documents', 'items', 'loot_tables')}
        artifacts['dependency-catalog.json'] = {'definitions': [ce.ref('Encounter', artifacts['encounter.json']['identity']['key'])], 'assets': []}
        candidates[fight] = artifacts
    for name, fight in SELECTED.items():
        row = open_rows[name]
        text = pinned(canary, row['source']['file']).decode()
        points = destinations(name, text) if name != 'time guardian lost time' else None
        for binding in row['current_monster_dispositions']:
            if binding['status'] not in ('unresolved_semantics', 'unsupported_source_field'):
                continue
            monster_path = canary / binding['source_file']
            source_bytes = pinned(canary, binding['source_file'])
            if monster_path.read_bytes() != source_bytes:
                raise ValueError('schedule source checkout differs from pinned Git')
            registration, raw, _ = cb.load_monster(monster_path)
            if cb.slug(registration) != binding['monster']:
                raise ValueError('selected schedule monster identity differs from actual registration')
            field, index = re.fullmatch(r'(attacks|defenses)\[(\d+)\]', binding['source_field']).groups()
            entries = raw[field]
            original = (entries['_list'] if isinstance(entries, dict) else entries)[int(index) - 1]
            if original['name'].lower() != name:
                raise ValueError('source schedule binds another registered spell')
            schedules.append({'spell': row['spell'], 'source_file': binding['source_file'],
                'source_blob_sha1': cb.blob_id(source_bytes), 'source_field': binding['source_field'],
                'raw_source_schedule': original,
                'typed_schedule': {'ability': ce.ref('Ability', 'canary:ability/spell/' + cb.slug(name)),
                    'interval_ms': int(original['interval']), 'chance_percent': original['chance']},
                'registered_source_calls': [{'method': method, 'source_arguments': arguments}
                    for method, arguments in re.findall(r'spell:(\w+)\(([^\n]*?)\)', text)],
                'admission_authorized': False})
        artifacts = candidates[fight]
        encounter, manifest, catalog, deps = [artifacts[key] for key in ('encounter.json', 'manifest.json', 'catalog.json', 'dependencies.json')]
        key = 'canary:ability/spell/' + cb.slug(name)
        deps['abilities'].append(ability(key, encounter['identity']['key']))
        catalog['definitions'].append(ce.ref('Ability', key))
        if name == 'time guardian lost time':
            position_path = 'src/game/movement/position.hpp'
            position_bytes = pinned(canary, position_path)
            anchor, rules = lost_time_rules(text, position_bytes.decode())
            deps['abilities'][-1]['needs_direction'] = True
            encounter['anchors'].append(anchor)
            encounter['rules'].extend(rules)
            for actor in LOST_TIME_ROLES.values():
                catalog['definitions'].append(ce.ref('Creature', 'canary:creature/' + actor))
            for path, source_bytes, resolution in [
                (row['source']['file'], text.encode(), BOUNDARIES[name]),
                (position_path, position_bytes, 'Full-floor box uses the source uint16_t x/y domain, not guessed room bounds.')]:
                manifest['sources'].append({'kind': 'git', 'repository': ce.REPOSITORY, 'revision': ce.REVISION,
                                           'path': path, 'blob_sha1': cb.blob_id(source_bytes)})
                manifest['entries'].append({'source_index': len(manifest['sources']) - 1,
                    'source_lines': list(range(1, len(source_bytes.decode().splitlines()) + 1)),
                    'status': 'unresolved_semantics', 'resolution': resolution + ' DRAFT ONLY: no covers or whole-fight admission.'})
            continue
        before_after = []
        if name != 'generator':
            for label in ('poff', 'teleport'):
                extra = key + '/' + label
                deps['effects'].append({'identity': cb.ident(extra + '/effect'), 'operation': 'presentation_only',
                    'presentation': {'impact_asset_binding': 'canary.appearance:effect/' + label}})
                deps['abilities'].append(ability(extra, effects=[ce.ref('Effect', extra + '/effect')]))
                artifacts['dependency-catalog.json']['assets'].append('canary.appearance:effect/' + label)
                catalog['definitions'].append(ce.ref('Ability', extra))
                before_after.append({'kind': 'cast', 'ability': ce.ref('Ability', extra), 'at': 'subject_position'})
        branches = []
        for i, (x, y, floor) in enumerate(points):
            anchor = cb.slug(name) + '_landing_' + str(i)
            encounter['anchors'].append({'key': anchor, 'kind': 'point', 'description': 'Literal pinned spell destination.',
                                          'location': {'x': x, 'y': y, 'floor': floor}})
            if name == 'generator':
                catalog['definitions'].append(ce.creature('glooth-generator'))
                actions = [{'kind': 'spawn', 'creature': ce.creature('glooth-generator'), 'count': 1,
                    'at': {'anchor': anchor}, 'owner': 'none', 'health': 'full'},
                    {'kind': 'say', 'subject': {'spawned': True},
                     'text': 'THE GLOOTH GENERATOR CHARGES UP FOR A LETHAL EXPLOSION!', 'mode': 'yell'}]
            else:
                actions = [{'kind': 'teleport', 'who': {'triggering': True}, 'to': anchor}]
            branches.append({'weight': 1, 'actions': actions})
        middle = branches[0]['actions'] if len(branches) == 1 else [{'kind': 'one_of', 'branches': branches}]
        if name == 'maxxenteleport':
            # Keep the two independent source draws (x then y), not one flat draw.
            by_x = {}
            for point, branch in zip(points, branches):
                by_x.setdefault(point[0], []).append(branch)
            middle = [{'kind': 'one_of', 'branches': [{'weight': 1,
                'actions': [{'kind': 'one_of', 'branches': choices}]} for choices in by_x.values()]}]
        encounter['rules'].append({'key': 'round3_' + cb.slug(name),
            'trigger': {'kind': 'ability_cast', 'role': fight, 'ability': ce.ref('Ability', key)},
            'conditions': [], 'actions': (before_after[:1] + middle + before_after[1:])})
        # Typed core preparation is not a covers claim or a resolved population disposition.
        source = {'kind': 'git', 'repository': ce.REPOSITORY, 'revision': ce.REVISION,
                  'path': row['source']['file'], 'blob_sha1': row['source']['blob_sha1']}
        manifest['sources'].append(source)
        manifest['entries'].append({'source_index': len(manifest['sources']) - 1,
            'source_lines': list(range(1, len(text.splitlines()) + 1)), 'status': 'unresolved_semantics',
            'resolution': BOUNDARIES[name] + ' DRAFT ONLY: do not add covers or admit the whole fight.'})
    for artifacts in candidates.values():
        catalog = artifacts['catalog.json']
        catalog['definitions'] = list({json.dumps(r, sort_keys=True): r for r in catalog['definitions']}.values())
        errors = ve.validate(artifacts['encounter.json'], catalog, artifacts['manifest.json'])
        errors += vm.structural('monster-dependencies.schema.json', artifacts['dependencies.json'])
        local_refs = {tuple((family, definition['identity']['key'], definition['identity']['revision']))
                      for section, family in [('abilities', 'Ability'), ('effects', 'Effect'), ('formulas', 'Formula')]
                      for definition in artifacts['dependencies.json'][section]}
        declared = local_refs | {tuple((r['family'], r['key'], r['revision'])) for r in artifacts['dependency-catalog.json']['definitions']}
        for _, value in vm.walk(artifacts['dependencies.json']):
            if isinstance(value, dict) and set(value) == {'family', 'key', 'revision'}:
                if (value['family'], value['key'], value['revision']) not in declared:
                    errors.append('dependency reference not declared exactly: ' + str(value))
        for effect in artifacts['dependencies.json']['effects']:
            for value in effect.get('presentation', {}).values():
                if value not in artifacts['dependency-catalog.json']['assets']:
                    errors.append('presentation asset not declared')
        if errors:
            raise ValueError('typed draft invalid: ' + '; '.join(errors))
    ledger = [{'spell': row['spell'], 'source': row['source'], 'open_references': row['current_open_references'],
               'preparation': 'ACCEPTED_TYPED_CORE_DRAFT' if name in SELECTED else 'CONTRACT_OR_OWNER_BOUNDARY',
               'remaining': BOUNDARIES[name], 'complete_spell_conversion_claimed': False,
               'runtime_qualified': False, 'admission_authorized': False} for name, row in sorted(open_rows.items())]
    return {'schema': 'OTERYN_CUSTOM_SPELL_TYPED_DRAFT_PREPARATION/v1', 'source_revision': ce.REVISION,
            'input_packet_sha256': hashlib.sha256(packet_path.read_bytes()).hexdigest(),
            'counts': {'open_spell_identities': len(open_rows), 'open_reference_rows': reference_count,
                       'typed_core_spell_drafts': len(SELECTED), 'fight_successors': len(candidates)},
            'ledger': ledger, 'candidates': candidates, 'source_schedule_bindings': schedules,
            'targetfirering_source_bug': constant_oracle(canary),
            'schema_and_encounter_semantics_valid': True, 'full_spell_conversion_claimed': False,
            'runtime_qualified': False, 'admission_authorized': False}


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--canary', type=Path, required=True)
    parser.add_argument('--packet', type=Path, required=True)
    parser.add_argument('--out', type=Path, required=True)
    args = parser.parse_args()
    if ROOT.parents[2] in (args.out.resolve(), *args.out.resolve().parents):
        raise ValueError('draft output must be external')
    result = prepare(args.canary, args.packet)
    args.out.parent.mkdir(parents=True, exist_ok=True)
    args.out.write_text(json.dumps(result, ensure_ascii=False, indent=2, sort_keys=True) + '\n')
    print(json.dumps(result['counts']))


if __name__ == '__main__':
    main()
