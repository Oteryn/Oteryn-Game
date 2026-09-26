"""Census of Canary monster spell scripts: which ones the monster authoring schema can express.

Evidence tooling only. Every monster file is read with the converter's stubbed sandbox and only its
`attacks` and `defenses` entries are used. A spell name resolves as in Canary
`Monsters::deserializeSpell`: `Spells::getSpellByName` (rune spells first, then instant spells,
case-insensitive) wins over the built-in spell kinds, so a registered name shadows an inline kind.
Each spell script is evaluated in a stubbed LuaJIT sandbox whose Combat, Condition, Spell and area
helpers only record calls; no Canary engine code runs. Each referenced registered spell is classified:

  P1 shared spell   a player spell or rune (data/scripts/spells, data/scripts/runes) that ends in one
                    Combat execution; player-only branches never run for a monster caster. The engine
                    takes damage from the monster entry, shape/effects from the script
  P2 declarative    onCastSpell only executes one Combat (after an optional nil guard or cast voice line)
  P3 parametric     a table of Combats, onCastSpell only executes one picked by math.random
  P4 custom logic   anything else (summons, targeting, loops, timers, teleports, storage, per-target
                    Lua combat callbacks, ...)
  NOOP              a player-only spell whose script returns false for a non-player caster
  MISSING           referenced name with no registered spell and no built-in kind

and the Combat/Condition primitives each spell uses are counted so that schema extensions can be
chosen by frequency. Usage: python spell_census.py --canary <Canary checkout> [--out FILE]
"""
import argparse
import json
import re
from collections import Counter, defaultdict
from pathlib import Path

import canary_batch as cb

ROOT = Path(__file__).resolve().parent
SCRIPT_DIRS = ('data/scripts', 'data-otservbr-global/scripts')
SHARED_DIRS = ('data/scripts/spells/', 'data/scripts/runes/')
INLINE = {'melee', 'combat', 'condition', 'speed', 'outfit', 'invisible', 'drunk', 'firefield', 'poisonfield',
          'energyfield', 'strength', 'effect', 'fear', 'soulwars fear'}

LUA_STUBS = r'''
local calls = {}
local function recorder(kind)
  local obj = {__kind = kind, __calls = {}}
  table.insert(calls, obj)
  return setmetatable(obj, {
    __index = function(t, method)
      return function(self, ...)
        local args = {...}
        local flat = {}
        for i = 1, select('#', ...) do
          local v = args[i]
          if type(v) == 'table' and v.__kind then flat[i] = '<' .. v.__kind .. '>'
          elseif type(v) == 'table' then flat[i] = '<table>'
          elseif type(v) == 'function' then flat[i] = '<function>'
          else flat[i] = tostring(v) end
        end
        table.insert(rawget(t, '__calls'), {method, flat})
        return self
      end
    end,
    __newindex = function(t, k, v) rawset(t, '__field_' .. k, type(v)) end,
  })
end
setmetatable(_G, {__index = function(_, k) return '@' .. k end})
Combat = function() return recorder('Combat') end
Condition = function(kind) local c = recorder('Condition'); c:type(kind); return c end
Spell = function(kind) local s = recorder('Spell'); s:kind(kind); return s end
createCombatArea = function(area, extended) return {__kind = 'Area', __calls = {}} end
return calls
'''


def run_script(path):
    from lupa.luajit21 import LuaRuntime
    lua = LuaRuntime(unpack_returned_tuples=True)
    calls = lua.execute(LUA_STUBS)
    try:
        lua.execute(path.read_text(encoding='utf-8', errors='replace'))
    except Exception as exc:  # scripts with unsupported top-level logic
        return None, str(exc).splitlines()[0][:160]
    objects = []
    for obj in calls.values():
        record = {'kind': obj['__kind'], 'calls': [], 'fields': []}
        for call in obj['__calls'].values():
            args = [v for _, v in sorted(call[2].items())] if call[2] else []
            record['calls'].append((call[1], args))
        for key in obj.keys():
            if isinstance(key, str) and key.startswith('__field_'):
                record['fields'].append(key[8:])
        objects.append(record)
    return objects, None


def cast_body(text):
    match = re.search(r'function\s+[\w.:]*onCastSpell\s*\([^)]*\)(.*?)\nend\b', text, re.S)
    return re.sub(r'\s+', ' ', match.group(1)).strip() if match else None


EXECUTE = r'return (\w+):execute\((?:creature|cid), (?:var|variant)\)'
RANDOM_PICK = (r'return \w+\[math\.random\([^)]*\)\]:execute\((?:creature|cid), (?:var|variant)\)',
               r'local (\w+) = \w+\[math\.random\([^)]*\)\] return \1:execute\((?:creature|cid), (?:var|variant)\)')
PLAYER_ONLY = r'local player = creature:getPlayer\(\) if not (?:creature or not )?player then return false end'


GUARD = r'^if not creature then return(?: false)? end '
VOICE = r'^creature:say\("[^"]*", TALKTYPE_MONSTER_(?:SAY|YELL)\) '
LUA_CALLBACKS = ('CALLBACK_PARAM_TARGETCREATURE', 'CALLBACK_PARAM_TARGETTILE', 'CALLBACK_PARAM_CHAINPICKER')


def classify(text, objects, error, shared):
    if error or objects is None:
        return 'P4', ['load_error: ' + (error or '')]
    body = cast_body(text) or ''
    if shared and re.match(PLAYER_ONLY, body):
        return 'NOOP', ['script returns false for a non-player caster: ' + body[:120]]
    reasons = []
    body = re.sub(GUARD, '', body)
    if re.match(VOICE, body):
        body = re.sub(VOICE, '', body)
        reasons.append('cast voice line before the combat')
    callbacks = sorted({args[0].lstrip('@') for o in objects if o['kind'] == 'Combat'
                        for method, args in o['calls'] if method == 'setCallback' and args} & set(LUA_CALLBACKS))
    tier = None
    final = re.search(EXECUTE + r'$', body)
    if shared and final:
        prefix = body[:final.start()]
        if not prefix or re.search(r'getPlayer\(\)|familiar\(\)', prefix):
            tier = 'P1'
            if prefix:
                reasons.append('player/familiar-only branches ignored for a monster caster')
    if tier is None and re.fullmatch(EXECUTE, body):
        tier = 'P2'
    if tier is None and any(re.fullmatch(pattern, body) for pattern in RANDOM_PICK):
        tier = 'P3'
    if tier is None:
        return 'P4', ['onCastSpell: ' + body[:160]]
    if callbacks:
        return 'P4', reasons + ['per-target Lua combat callback: ' + ', '.join(callbacks)]
    return tier, reasons


def primitives(objects):
    found = Counter()
    for obj in objects or []:
        for method, args in obj['calls']:
            if obj['kind'] == 'Combat' and method == 'setParameter' and args:
                found['combat.' + args[0].lstrip('@')] += 1
            elif obj['kind'] == 'Combat' and method in ('setArea', 'setCallback', 'setFormula', 'addCondition', 'setOrigin'):
                found['combat.' + method + ('.' + args[0].lstrip('@') if method == 'setCallback' and args else '')] += 1
            elif obj['kind'] == 'Condition' and method == 'type' and args:
                found['condition.' + args[0].lstrip('@')] += 1
            elif obj['kind'] == 'Condition' and method == 'setParameter' and args:
                found['condition_param.' + args[0].lstrip('@')] += 1
            elif obj['kind'] == 'Condition' and method in ('addDamage', 'setFormula', 'setOutfit', 'setTicks'):
                found['condition.' + method] += 1
    return found


def load_monster(path):
    """Like canary_batch.load_monster, but keeps a monster registered before a later top-level error."""
    from lupa.luajit21 import LuaRuntime
    lua = LuaRuntime(unpack_returned_tuples=True)
    registered, _ = lua.execute(cb.LUA_PRELUDE)
    try:
        lua.execute(path.read_text(encoding='utf-8'))
        error = None
    except Exception as exc:
        error = str(exc).splitlines()[0][:160]
    monster = registered['monster']
    if monster is None:
        raise ValueError(error or 'no monster registered')
    return cb.lua_value(monster), error


def index_spells(canary):
    """name -> [(kind, path, text)] for every Spell(...) registration, rune spells first."""
    index = defaultdict(list)
    for directory in SCRIPT_DIRS:
        for path in sorted((canary / directory).rglob('*.lua')):
            text = path.read_text(encoding='utf-8', errors='replace')
            if 'Spell(' not in text:
                continue
            for variable, kind in re.findall(r'(\w+)\s*=\s*Spell\(\s*"(\w+)"', text):
                for name in re.findall(re.escape(variable) + r':name\(\s*"([^"]+)"\s*\)', text):
                    index[name.lower()].append((kind.lower(), path, text))
    for name in index:
        index[name].sort(key=lambda entry: (entry[0] != 'rune', str(entry[1])))
    return index


def main():
    parser = argparse.ArgumentParser(description=__doc__.split('\n')[0])
    parser.add_argument('--canary', required=True, type=Path)
    parser.add_argument('--out', type=Path, default=ROOT / 'samples' / 'spell-census-canary-47dfd51f.json')
    args = parser.parse_args()

    scripts = index_spells(args.canary)
    references = Counter()
    users = defaultdict(set)
    shadowed = Counter()
    unreadable = []
    partial = []
    monster_files = sorted((args.canary / cb.MONSTER_DIR).rglob('*.lua'))
    for path in monster_files:
        try:
            source, late_error = load_monster(path)
            if late_error:
                partial.append({'file': str(path.relative_to(args.canary)), 'error_after_register': late_error})
        except Exception as exc:  # monster files with unsupported top-level logic
            unreadable.append({'file': str(path.relative_to(args.canary)), 'error': str(exc).splitlines()[0][:160]})
            continue
        for block in ('attacks', 'defenses'):
            entries = source.get(block) or []
            for entry in entries.values() if isinstance(entries, dict) else entries:
                if not isinstance(entry, dict):
                    continue
                name = str(entry.get('name', '')).lower()
                if name in scripts:
                    if name in INLINE:
                        shadowed[name] += 1
                    references[name] += 1
                    users[name].add(path.stem)
                elif name not in INLINE:
                    references[name] += 1
                    users[name].add(path.stem)

    spells = []
    tiers = Counter()
    tier_refs = Counter()
    prim_by_tier = defaultdict(Counter)
    for name, count in sorted(references.items()):
        entry = {'name': name, 'monster_references': count, 'example_monsters': sorted(users[name])[:5]}
        if name not in scripts:
            tier, reasons, found = 'MISSING', ['no registered spell and no built-in spell kind with this name'], Counter()
        else:
            kind, path, text = scripts[name][0]
            shared = str(path.relative_to(args.canary)).startswith(SHARED_DIRS)
            objects, error = run_script(path)
            tier, reasons = classify(text, objects, error, shared)
            found = primitives(objects)
            entry.update(kind=kind, script=str(path.relative_to(args.canary)))
            if len(scripts[name]) > 1:
                entry['other_registrations'] = [str(p.relative_to(args.canary)) for _, p, _ in scripts[name][1:]]
            if name in INLINE:
                reasons = reasons + ['registered spell shadows the built-in spell kind of the same name']
        tiers[tier] += 1
        tier_refs[tier] += count
        prim_by_tier[tier].update(found.keys())
        entry.update(tier=tier, primitives=sorted(found), reasons=reasons)
        spells.append(entry)

    report = {'source': {'repository': cb.REPOSITORY, 'revision': cb.REVISION},
              'resolution_rule': 'Monsters::deserializeSpell -> Spells::getSpellByName: rune spells, then instant spells, '
                                 'case-insensitive, before built-in kinds',
              'monster_files_scanned': len(monster_files), 'monster_files_unreadable': unreadable,
              'monster_files_registered_before_error': partial,
              'spell_registrations_indexed': len(scripts), 'distinct_referenced_spells': len(references),
              'shadowed_builtin_kinds': dict(sorted(shadowed.items())),
              'tiers_by_spell': dict(sorted(tiers.items())),
              'tiers_by_monster_reference': dict(sorted(tier_refs.items())),
              'primitive_spell_counts_by_tier': {t: dict(sorted(c.items(), key=lambda kv: (-kv[1], kv[0])))
                                                 for t, c in sorted(prim_by_tier.items())},
              'spells': spells}
    args.out.write_text(json.dumps(report, ensure_ascii=False, indent=2) + '\n', encoding='utf-8', newline='\n')
    print(json.dumps({k: report[k] for k in ('monster_files_scanned', 'spell_registrations_indexed', 'distinct_referenced_spells',
                                              'shadowed_builtin_kinds', 'tiers_by_spell', 'tiers_by_monster_reference')}))
    print('unreadable monster files:', len(unreadable))


if __name__ == '__main__':
    main()
