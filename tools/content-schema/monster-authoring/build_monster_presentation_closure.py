"""Close source presentation/condition evidence for every donor monster spell slot.

Runs pinned readSpell helpers in a recording Lua sandbox, never a donor engine.
Donor numeric IDs are reference values, not allocations of native Game IDs.
"""
import argparse
from collections import Counter
import gzip
import hashlib
import json
from pathlib import Path
import re
import subprocess

import canary_batch as cb

HELPER = 'data/scripts/lib/register_monster_type.lua'
HEADER = 'src/creatures/creatures_definitions.hpp'
UTILS = 'src/utils/utils_definitions.hpp'


def sha(data):
    return hashlib.sha256(data).hexdigest()


def enum(text, name, prefix=''):
    match = re.search(r'enum\s+' + re.escape(name) + r'\b[^\{]*\{(.*?)\};', text, re.S)
    if not match:
        raise ValueError('missing enum ' + name)
    values, current = {}, -1
    for part in re.sub(r'/\*.*?\*/|//[^\n]*', '', match[1], flags=re.S).split(','):
        item = re.fullmatch(r'\s*(\w+)\s*(?:=\s*(\w+))?\s*', part)
        if not item:
            if part.strip():
                raise ValueError('unsupported enum expression ' + part.strip())
            continue
        key, expression = item.groups()
        current = current + 1 if expression is None else values.get(expression)
        if current is None:
            current = int(expression, 0)
        values[key] = current
    return {prefix + key: value for key, value in values.items()}


def source_file(root, relative, revision):
    path = root / relative
    data = path.read_bytes()
    blob = subprocess.check_output(['git', '-C', str(root), 'rev-parse', revision + ':' + relative], text=True).strip()
    if cb.blob_id(data) != blob:
        raise ValueError('source differs from pinned Git blob: ' + relative)
    return {'path': relative, 'revision': revision, 'git_blob': blob, 'sha256': sha(data)}


class HelperCapture:
    def __init__(self, helper_text):
        from lupa.luajit21 import LuaRuntime
        self.lua = LuaRuntime(unpack_returned_tuples=True, register_eval=False, register_builtins=False, max_memory=16 * 1024 * 1024)
        self.reset_budget = self.lua.execute('''
jit.off()
jit=nil
local sethook = debug.sethook
local function reset()
  local remaining = 100
  sethook(function() remaining = remaining - 1; if remaining <= 0 then error('capture instruction budget exceeded') end end, '', 10000)
end
os=nil; io=nil; package=nil; debug=nil; require=nil; dofile=nil; loadfile=nil; load=nil; loadstring=nil; python=nil
return reset
''')
        self.reset_budget()
        self.lua.execute('''
MonsterType = {}
logger = {warn = function() end}
registeredSpellName = nil
Spell = function(name) if name == registeredSpellName then return {} end return nil end
local function object()
  local result = {calls = {}}
  return setmetatable(result, {__index = function(t, method)
    return function(self, ...) table.insert(t.calls, {method, {...}}) end
  end})
end
MonsterSpell = object
setmetatable(_G, {__index = function(_, k) return '@' .. k end})
forcedRandom = 1
math.random = function(a, b) return math.min(forcedRandom, b or a) end
''')
        # Export the actual lexical default selector without rewriting its body.
        self.defaults = self.lua.execute(helper_text + '\nreturn loadSpellSoundType')

    def table(self, value):
        if isinstance(value, dict):
            return self.lua.table_from({k: self.table(v) for k, v in value.items()})
        if isinstance(value, list):
            return self.lua.table_from([self.table(v) for v in value])
        return value

    def capture(self, parameters, target_distance, registered=False):
        self.lua.globals().registeredSpellName = parameters.get("name") if registered else None
        self.reset_budget()
        mtype = self.lua.eval('function(distance) return {targetDistance=function() return distance end, name=function() return "qualification" end} end')(target_distance)
        captures, default_pairs = [], []
        # Defaults randomly choose between three close or two ranged melee sounds.
        for choice in range(1, 4):
            self.lua.globals().forcedRandom = choice
            pair = cb.lua_value(self.defaults(self.table(parameters), mtype))
            calls = cb.lua_value(self.lua.globals().readSpell(self.table(parameters), mtype)['calls'])
            if calls not in captures:
                captures.append(calls)
            if pair not in default_pairs:
                default_pairs.append(pair)
        return {'default_sound_variants': default_pairs, 'actual_helper_call_variants': captures}


def resolve(value, constants):
    if value is False:
        return {'status': 'source_explicitly_suppressed', 'value': False}
    if value is None:
        return {'status': 'source_absent'}
    name = value.lstrip('@') if isinstance(value, str) else None
    if name in constants:
        return {'status': 'source_resolved', 'symbol': name, 'donor_numeric_id': constants[name]}
    if isinstance(value, int) and not isinstance(value, bool) and value in constants.values():
        return {'status': 'source_resolved_numeric', 'donor_numeric_id': value}
    return {'status': 'source_unresolved', 'value': value}


def close_slot(slot, helper, constants, target_distance, registration):
    captured = helper.capture(slot['source_parameters'], target_distance, registered=slot['resolution'] == 'registered')
    variants = []
    for calls in captured['actual_helper_call_variants']:
        last = {method: args for method, args in calls}
        variants.append({
            'impact_visual': resolve((last.get('setCombatEffect') or [None])[0], constants),
            'projectile_visual': resolve((last.get('setCombatShootEffect') or [None])[0], constants),
            'cast_sound': resolve((last.get('castSound') or [None])[0], constants),
            'impact_sound': resolve((last.get('impactSound') or [None])[0], constants),
            'condition_calls': [call for call in calls if call[0].startswith('setCondition')],
        })
    conversion = (registration or {}).get('conversion') or {}
    combats = conversion.get('combats') or {}
    script_variants = []
    spell_calls = conversion.get('spell_calls') or {}
    registration_sounds = {field: resolve((spell_calls.get(method) or [None])[0], constants)
                           for field, method in (('cast_sound', 'castSound'), ('impact_sound', 'impactSound'))}
    for index, combat in sorted(combats.items()):
        params = combat.get('params', {})
        script_variants.append({'combat_index': index,
            'impact_visual': resolve(params.get('COMBAT_PARAM_EFFECT'), constants),
            'projectile_visual': resolve(params.get('COMBAT_PARAM_DISTANCEEFFECT'), constants),
            'cast_sound': resolve(params.get('COMBAT_PARAM_CASTSOUND'), constants),
            'impact_sound': resolve(params.get('COMBAT_PARAM_IMPACTSOUND'), constants),
            'conditions': combat.get('conditions', []), 'area': combat.get('area'),
            'callbacks': combat.get('callbacks', {})})
    return {'candidate_id': slot['candidate_id'], 'group': slot['group'],
        'source_slot_index': slot['source_slot_index'], 'source': slot['source'],
        'source_resolution': slot['resolution'],
        'engine_presentation_precedence': 'registered_script_over_inline_helper' if slot['resolution'] == 'registered' else 'inline_helper',
        'conversion_status': slot['conversion_status'],
        'source_parameters': slot['source_parameters'], 'target_distance': target_distance,
        'helper_variants': variants, 'helper_default_sound_variants': captured['default_sound_variants'],
        'registered_combat_variants': script_variants,
        'registered_spell_sounds': registration_sounds,
        'registered_script_capture_status': ('captured' if script_variants else 'unresolved_custom_or_no_combat') if slot['resolution'] == 'registered' else 'not_registered',
        'engine_status': 'not_activated_source_evidence_only',
        'source_monster_sha256': slot['monster_source']['sha256'],
        'helper_condition_scope': 'helper_recorded_settings_not_applied_engine_conditions',
        'source_registered_sha256': (slot.get('registered_source') or {}).get('sha256'),
        'native_sound_asset_binding': None, 'native_visual_runtime_verified': False}


def build(import_root, sources, output):
    slots = json.loads((import_root / 'monster-spell-slots.json').read_text())
    registrations = json.loads((import_root / 'all-registered-spells.json').read_text())
    reg_index = {(r['source'], r['provenance']['path'], r['name']): r for r in registrations}
    output.mkdir(parents=True, exist_ok=False)
    provenance, helpers, constants, distance_cache = {}, {}, {}, {}
    for source, root in sources.items():
        revisions = {s['monster_source']['revision'] for s in slots if s['source'] == source}
        if len(revisions) != 1:
            raise ValueError('mixed source revisions')
        revision = revisions.pop()
        provenance[source] = [source_file(root, p, revision) for p in (HELPER, HEADER, UTILS)]
        helpers[source] = HelperCapture((root / HELPER).read_text())
        effects, missiles = cb.load_effect_constants(root / UTILS)
        constants[source] = {**effects, **missiles, **enum((root / HEADER).read_text(), 'SoundEffect_t', 'SOUND_EFFECT_TYPE_')}
    counters = Counter()
    payload = []
    for slot in slots:
        source = slot['source']
        path = slot['monster_source']['path']
        key = source, path
        if key not in distance_cache:
            root = sources[source]
            source_path = root / path
            if not source_path.exists():
                source_path = import_root / 'source-inputs' / source / path
            data = source_path.read_bytes()
            if sha(data) != slot['monster_source']['sha256']:
                raise ValueError('monster source mismatch ' + path)
            matches = re.findall(r'\btargetDistance\s*=\s*(\d+)\b', data.decode())
            if len(set(matches)) > 1:
                raise ValueError('ambiguous targetDistance ' + path)
            if not matches and re.search(r'\btargetDistance\s*=', data.decode()):
                raise ValueError('nonliteral targetDistance ' + path)
            distance = int(matches[0]) if matches else 1
            distance_cache[key] = distance
        pr = slot.get('registered_source')
        registration = reg_index.get((source, pr['path'], slot['source_parameters']['name'].lower())) if pr else None
        if registration:
            script_path = sources[source] / pr['path']
            if not script_path.exists():
                script_path = import_root / 'source-inputs' / source / pr['path']
            if sha(script_path.read_bytes()) != pr['sha256']:
                raise ValueError('registered source mismatch')
        result = close_slot(slot, helpers[source], constants[source], distance_cache[key], registration)
        counters['slots'] += 1
        counters['slots_with_captured_registered_combat'] += bool(result['registered_combat_variants'])
        counters['slots_with_registered_sound'] += any(v['status'].startswith('source_resolved') for v in result['registered_spell_sounds'].values())
        counters['slots_with_condition_calls'] += any(v['condition_calls'] for v in result['helper_variants']) or any(v['conditions'] for v in result['registered_combat_variants'])
        for field in ('impact_visual', 'projectile_visual', 'cast_sound', 'impact_sound'):
            variants = result['helper_variants'] + result['registered_combat_variants']
            counters['slots_with_nonzero_donor_' + field] += any(v[field].get('donor_numeric_id', 0) > 0 for v in variants)
            counters['slots_with_' + field] += any(v[field]['status'].startswith('source_resolved') for v in variants)
            counters['slots_unresolved_' + field] += any(v[field]['status'] == 'source_unresolved' for v in variants)
        payload.append(json.dumps(result, ensure_ascii=False, sort_keys=True, separators=(',', ':')) + '\n')
    raw = ''.join(payload).encode()
    (output / 'monster-slot-presentation.jsonl.gz').write_bytes(gzip.compress(raw, mtime=0))
    report = {'schema_version': 1, 'counts': dict(counters), 'source_provenance': provenance,
        'input_sha256': {p: sha((import_root / p).read_bytes()) for p in ('monster-spell-slots.json', 'all-registered-spells.json')},
        'payload_sha256': sha(raw), 'runtime_activation': False,
        'limits': ['All source slots retained; donor IDs are never native Game allocations.',
            'Helpers route default impact sounds through castSound; actual setter results and semantic defaults are separate.',
            'Registered combat capture preserves conditions and callbacks but does not claim engine execution.',
            'Resolved sound IDs are reference settings, including silence if present; not audible client playback.',
            'Sound asset bindings and client playback remain unverified; missing source values are separate from unsupported runtime.']}
    (output / 'summary.json').write_text(json.dumps(report, indent=2) + '\n')
    print(json.dumps(report['counts']))


if __name__ == '__main__':
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--import-root', type=Path, required=True)
    parser.add_argument('--canary', type=Path, required=True)
    parser.add_argument('--crystal', type=Path, required=True)
    parser.add_argument('--output', type=Path, required=True)
    args = parser.parse_args()
    build(args.import_root, {'canary': args.canary, 'crystal': args.crystal}, args.output)
