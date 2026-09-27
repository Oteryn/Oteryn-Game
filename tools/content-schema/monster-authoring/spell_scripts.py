"""Evaluate Canary registered spell scripts into plain data for the converter (D10, D11, D12).

Evidence tooling only. A spell script runs in a stubbed LuaJIT sandbox: Combat, Condition, Spell and
createCombatArea only record their calls, and the spell's onCastSpell is then called with a stub
caster whose methods all return nil (a monster is not a player). Combat:execute records which Combat
ran. math.random is replaced so that every value of a small random range is tried, which turns a
random pick between Combats (P3) into an explicit list of variants. No Canary engine code runs.
"""
import re
from pathlib import Path

SPELL_LIB = 'data/scripts/lib/register_spells.lua'
ENGINE_DEFINITIONS = 'src/creatures/creatures_definitions.hpp'
SCRIPT_DIRS = ('data/scripts', 'data-otservbr-global/scripts')
SHARED_DIRS = ('data/scripts/spells/', 'data/scripts/runes/')
MAX_RANDOM_RANGE = 64

LUA_SANDBOX = r'''
local rec = {combats = {}, conditions = {}, spells = {}, executed = {}, randoms = {}}
local function recorder(kind, list)
  local obj = {__kind = kind, __calls = {}, __n = #list}
  table.insert(list, obj)
  return setmetatable(obj, {__index = function(t, method)
    if method == 'execute' and kind == 'Combat' then
      return function(self) table.insert(rec.executed, self); return true end
    end
    return function(self, ...)
      table.insert(rawget(t, '__calls'), {method, {...}})
      return self
    end
  end})
end
Combat = function() return recorder('Combat', rec.combats) end
Condition = function(kind, id) local c = recorder('Condition', rec.conditions); rawset(c, '__type', kind); return c end
Spell = function(kind) local s = recorder('Spell', rec.spells); rawset(s, '__type', kind); return s end
createCombatArea = function(area, ext) return {__kind = 'Area', north = area, ext = ext} end
local forced = nil
math.random = function(a, b)
  table.insert(rec.randoms, {a, b})
  if forced ~= nil then return forced end
  if b == nil then return 1 end
  return a
end
setmetatable(_G, {__index = function(_, k) return '@' .. k end})
local caster = setmetatable({}, {__index = function() return function() return nil end end})
local function cast(spell, value)
  forced = value
  rec.executed = {}
  rec.randoms = {}
  local ok, result = pcall(spell.onCastSpell, caster, {})
  forced = nil
  return ok, result
end
return rec, cast, caster
'''


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


def body_tier(text, shared, callbacks):
    """P1/P2/P3/P4/NOOP of a spell script from its onCastSpell body and its Combat callback kinds."""
    body = cast_body(text) or ''
    if shared and re.match(PLAYER_ONLY, body):
        return 'NOOP', ['script returns false for a non-player caster: ' + body[:120]]
    reasons = []
    body = re.sub(GUARD, '', body)
    if re.match(VOICE, body):
        body = re.sub(VOICE, '', body)
        reasons.append('cast voice line before the combat')
    lua_callbacks = sorted(set(callbacks) & set(LUA_CALLBACKS))
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
    if lua_callbacks:
        return 'P4', reasons + ['per-target Lua combat callback: ' + ', '.join(lua_callbacks)]
    return tier, reasons


ENUMS = ('ConditionParam_t', 'ConditionType_t', 'CombatParam_t', 'CallBackParam_t', 'CombatType_t')


def enum_values(text, enum):
    """name -> value of a C++ enum: explicit integer values, otherwise the previous value plus one."""
    body = re.search(r'enum ' + enum + r'[^{]*\{(.*?)\};', text, re.S).group(1)
    values, current = {}, -1
    for line in body.splitlines():
        line = line.split('//')[0].strip().rstrip(',')
        match = re.match(r'([A-Z][A-Z0-9_]+)\s*(?:=\s*(.+))?$', line)
        if not match:
            continue
        name, expression = match.groups()
        if expression is None:
            current += 1
        elif expression in values:
            current = values[expression]
        else:
            shift = re.fullmatch(r'1\s*<<\s*(\d+)', expression)
            current = 1 << int(shift.group(1)) if shift else int(expression, 0)
        values[name] = current
    return values


def engine_enums(canary):
    """name -> value of the Canary enums a spell script passes to Combat and Condition."""
    text = (canary / ENGINE_DEFINITIONS).read_text(encoding='utf-8')
    return {enum: enum_values(text, enum) for enum in ENUMS}


def area_constants(canary):
    """Top-level `NAME = { ... }` table blocks of the Canary spell library (AREA_* and friends)."""
    text = (canary / SPELL_LIB).read_text(encoding='utf-8', errors='replace')
    return '\n'.join(re.findall(r'^[A-Z]\w* = \{.*?^\}', text, re.M | re.S))


def index_spells(canary):
    """name -> (kind, path) of every Spell registration, rune spells first (Spells::getSpellByName)."""
    found = {}
    for directory in SCRIPT_DIRS:
        for path in sorted((canary / directory).rglob('*.lua')):
            text = path.read_text(encoding='utf-8', errors='replace')
            if 'Spell(' not in text:
                continue
            for variable, kind in re.findall(r'(\w+)\s*=\s*Spell\(\s*"(\w+)"', text):
                for name in re.findall(re.escape(variable) + r':name\(\s*"([^"]+)"\s*\)', text):
                    found.setdefault(name.lower(), []).append((kind.lower(), path))
    return {name: sorted(entries, key=lambda e: (e[0] != 'rune', str(e[1])))[0] for name, entries in found.items()}


def to_python(value):
    if hasattr(value, 'items'):
        keys = list(value.keys())
        if keys and all(isinstance(k, int) for k in keys) and sorted(keys) == list(range(1, len(keys) + 1)):
            return [to_python(value[k]) for k in sorted(keys)]
        return {k: to_python(v) for k, v in value.items()}
    return value


def calls(obj):
    entries = to_python(obj['__calls']) if obj['__calls'] else []
    return [(c[0], list(c[1]) if isinstance(c[1], list) else []) for c in entries] if isinstance(entries, list) else []


class SpellScripts:
    def __init__(self, canary):
        self.canary = Path(canary)
        self.index = index_spells(self.canary)
        self.areas = area_constants(self.canary)
        self.enums = engine_enums(self.canary)
        self.cache = {}

    def evaluate(self, name):
        """Plain-data view of the registered spell `name`, or None when no spell has that name."""
        key = name.lower()
        if key not in self.index:
            return None
        if key not in self.cache:
            self.cache[key] = self._evaluate(key)
        return self.cache[key]

    def _evaluate(self, key):
        from lupa.luajit21 import LuaRuntime
        kind, path = self.index[key]
        relative = str(path.relative_to(self.canary))
        result = {'name': key, 'kind': kind, 'script': relative, 'shared': relative.startswith(SHARED_DIRS)}
        lua = LuaRuntime(unpack_returned_tuples=True)
        rec, cast, _ = lua.execute(LUA_SANDBOX)
        try:
            lua.execute(self.areas)
            lua.execute(path.read_text(encoding='utf-8', errors='replace'))
        except Exception as exc:
            return {**result, 'error': 'load: ' + str(exc).splitlines()[0][:160]}
        spell = next((s for s in rec['spells'].values()
                      if any(m == 'name' and a and str(a[0]).lower() == key for m, a in calls(s))), None)
        if spell is None or spell['onCastSpell'] is None:
            return {**result, 'error': 'no onCastSpell for this name'}
        result['spell_calls'] = {m: a for m, a in calls(spell) if m not in ('name', 'words', 'register')}
        combats = list(rec['combats'].values())
        callbacks = {str(a[0]).lstrip('@') for c in combats for m, a in calls(c) if m == 'setCallback' and a}
        result['tier'], result['tier_reasons'] = body_tier(path.read_text(encoding='utf-8', errors='replace'), result['shared'], callbacks)
        if result['tier'] in ('P4', 'NOOP'):
            return result

        ok, _ = cast(spell, None)
        if not ok:
            return {**result, 'error': 'onCastSpell raised with a stub caster'}
        ranges = [to_python(r) for r in rec['randoms'].values()]
        executed = [c['__n'] for c in rec['executed'].values()]
        variants = []
        if ranges:
            if len(ranges) != 1:
                return {**result, 'error': f'{len(ranges)} random draws per cast'}
            low, high = (1, ranges[0][0]) if len(ranges[0]) == 1 else ranges[0]
            if not (isinstance(low, int) and isinstance(high, int)) or high - low > MAX_RANDOM_RANGE:
                return {**result, 'error': f'random range {ranges[0]} is not a small integer range'}
            for value in range(low, high + 1):
                ok, _ = cast(spell, value)
                ran = [c['__n'] for c in rec['executed'].values()]
                if not ok or len(ran) != 1:
                    return {**result, 'error': f'random value {value} ran {len(ran)} combats'}
                variants.append(ran[0])
        elif len(executed) == 1:
            variants = executed
        else:
            return {**result, 'error': f'a cast ran {len(executed)} combats'}
        result['variants'] = variants
        result['combats'] = {n: self._combat(lua, combats[n]) for n in sorted(set(variants))}
        return result

    def _combat(self, lua, combat):
        data = {'params': {}, 'param_calls': [], 'callbacks': {}, 'conditions': [], 'area': None, 'formula': None}
        for method, args in calls(combat):
            if method == 'setParameter' and len(args) >= 2:
                data['params'][str(args[0]).lstrip('@')] = args[1].lstrip('@') if isinstance(args[1], str) else args[1]
                data['param_calls'].append([str(args[0]).lstrip('@'), args[1].lstrip('@') if isinstance(args[1], str) else args[1]])
            elif method == 'setArea' and args and isinstance(args[0], dict):
                data['area'] = {'north': args[0].get('north'), 'diagonal': args[0].get('ext')}
            elif method == 'setCallback' and len(args) >= 2:
                callback = str(args[0]).lstrip('@')
                data['callbacks'][callback] = args[1]
                if callback == 'CALLBACK_PARAM_CHAINVALUE':
                    function = lua.globals()[args[1]]
                    data['chain'] = list(function(None)) if function else None
            elif method == 'setFormula':
                data['formula'] = args
            elif method == 'addCondition' and args:
                condition = args[0]
                data['conditions'].append({'type': str(condition.get('__type', '')).lstrip('@'),
                                           'calls': [(m, [a.lstrip('@') if isinstance(a, str) else a for a in v])
                                                     for m, v in calls(condition)]})
        return data
