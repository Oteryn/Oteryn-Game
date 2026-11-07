"""Census of the registered player spells and runes of Canary and Crystal Server.

Evidence tooling only. Every `*.lua` file under `data/scripts/spells/**` and `data/scripts/runes/**`
is loaded in a stubbed LuaJIT sandbox: `Spell`, `Combat`, `Condition` and `createCombatArea` only
record their calls, and every other global resolves to its own name. Nothing of the engine runs.

For each registered spell the census records the registrar calls (words, group, vocations, level,
mana, soul, cooldowns, rune item, charges, ...), the Combat objects the file builds (parameters,
area, callbacks, conditions) and how `onCastSpell` behaves:

- `plain_combat`  the body only executes one Combat;
- `random_combat` the body executes one Combat picked by `math.random`;
- `conjure`       the body only calls `creature:conjureItem(reagent, result, count[, effect])`;
- `custom`        anything else (native behaviour candidate, like monster D13).

Damage and healing formulas (`CALLBACK_PARAM_LEVELMAGICVALUE` and `CALLBACK_PARAM_SKILLVALUE`) are
evaluated with symbolic arguments: arithmetic on them builds an expression tree instead of a number,
so the tree is the exact source formula in source operation order (constants folded by LuaJIT, as
the engine does). `calculateBaseDamageHealing(level)` stays one named node (`base_damage_healing`);
the other Crystal library helpers are loaded and traced through. A formula that branches on a
runtime value, or calls something unknown, stays unresolved with the Lua error.

Usage:
    python spell_census.py --canary <checkout> --crystal <checkout> --out samples/spell-census.json
    python spell_census.py self-test
"""
import argparse
import hashlib
import json
import re
import sys
from collections import Counter
from pathlib import Path

SPELL_ROOTS = ('data/scripts/spells', 'data/scripts/runes')
SPELL_LIB = 'data/scripts/lib/register_spells.lua'
ENUM_HEADERS = ('src/creatures/creatures_definitions.hpp', 'src/utils/utils_definitions.hpp')
EXCLUDED = {'data/scripts/spells/#example.lua': 'example script shipped for documentation'}
SOURCES = {
    # S14 (owner, 2026-09-28): the Tibia 15.30 branch dudantas/fix-tibia-15-30-regressions, not yet in main.
    'canary': {'repository': 'https://github.com/opentibiabr/canary',
               'branch': 'dudantas/fix-tibia-15-30-regressions',
               'revision': '99902524e052f37574194466c2949c576e4ab269'},
    'crystal': {'repository': 'https://github.com/zimbadev/crystalserver',
                'revision': 'ff7ede593c69d4c658b382c97443e8155926924a'},
}
# Library helpers that stay one named node instead of being traced (a world rule, not a spell fact).
NAMED_FUNCTIONS = {'calculateBaseDamageHealing': 'base_damage_healing'}
FORMULA_CALLBACKS = {
    'CALLBACK_PARAM_LEVELMAGICVALUE': ('level', 'magic_level', 'base_power'),
    'CALLBACK_PARAM_SKILLVALUE': ('attack_skill', 'attack_value', 'attack_factor', 'base_power'),
}
CAST_TIERS = ('plain_combat', 'random_combat', 'conjure', 'custom')

LUA_SANDBOX = r'''
local unpack = unpack or table.unpack
local Sym = {}
local function isSym(v) return type(v) == 'table' and getmetatable(v) == Sym end
local function wrap(v)
  if isSym(v) then return v end
  if type(v) ~= 'number' then error('symbolic arithmetic on ' .. type(v)) end
  return setmetatable({op = 'const', value = v}, Sym)
end
local function node(op, ...)
  local args = {}
  for i, v in ipairs({...}) do args[i] = wrap(v) end
  return setmetatable({op = op, args = args}, Sym)
end
local function var(name) return setmetatable({op = 'var', name = name}, Sym) end
Sym.__add = function(a, b) return node('add', a, b) end
Sym.__sub = function(a, b) return node('sub', a, b) end
Sym.__mul = function(a, b) return node('mul', a, b) end
Sym.__div = function(a, b) return node('div', a, b) end
Sym.__mod = function(a, b) return node('mod', a, b) end
Sym.__pow = function(a, b) return node('pow', a, b) end
Sym.__unm = function(a) return node('neg', a) end
Sym.__lt = function() error('branch on a runtime value') end
Sym.__le = function() error('branch on a runtime value') end
Sym.__concat = function() error('string use of a runtime value') end
Sym.__index = function(t, k)
  if k == 'op' or k == 'args' or k == 'name' or k == 'value' then return nil end
  return function(self, ...) return var('runtime:' .. tostring(k)) end
end

local real = {floor = math.floor, ceil = math.ceil, sqrt = math.sqrt, abs = math.abs,
              max = math.max, min = math.min, random = math.random}
for _, name in ipairs({'floor', 'ceil', 'sqrt', 'abs'}) do
  math[name] = function(x) if isSym(x) then return node(name, x) end return real[name](x) end
end
for _, name in ipairs({'max', 'min'}) do
  math[name] = function(...)
    for _, v in ipairs({...}) do if isSym(v) then return node(name, ...) end end
    return real[name](...)
  end
end

local rec = {combats = {}, conditions = {}, spells = {}, executed = {}, randoms = {}, player_calls = {}}
local function recorder(kind, list)
  local obj = {__kind = kind, __calls = {}, __n = #list + 1}
  table.insert(list, obj)
  return setmetatable(obj, {__index = function(t, method)
    if method == 'execute' and kind == 'Combat' then
      return function(self) table.insert(rec.executed, rawget(self, '__n')); return true end
    end
    return function(self, ...)
      local args = {...}
      if kind == 'Combat' and method == 'setCallback' and type(args[2]) == 'string' then
        args[3] = rawget(_G, args[2])
      end
      table.insert(rawget(t, '__calls'), {method, args})
      return self
    end
  end})
end
Combat = function() return recorder('Combat', rec.combats) end
Condition = function(kind, id) local c = recorder('Condition', rec.conditions); rawset(c, '__type', kind); rawset(c, '__id', id); return c end
Spell = function(kind) local s = recorder('Spell', rec.spells); rawset(s, '__type', kind); return s end
createCombatArea = function(area, ext) return {__kind = 'Area', north = area, diagonal = ext} end
-- Engine userdata constructors used at file scope; they only need to exist.
local inert = setmetatable({}, {__index = function() return function() return nil end end,
                               __call = function(self) return self end})
for _, name in ipairs({'Position', 'Game', 'ItemType', 'MonsterType', 'Tile', 'Item', 'Player', 'Creature',
                       'Monster', 'Npc', 'House', 'Party', 'Vocation', 'Group', 'Town', 'Variant', 'Storage',
                       'Container', 'Outfit', 'Teleport', 'Zone', 'EventCallback', 'CreatureEvent',
                       'GlobalEvent', 'Action', 'MoveEvent', 'TalkAction', 'Weapon', 'Imbuement', 'Mount',
                       'Familiar', 'Loot', 'Guild', 'NetworkMessage', 'ModalWindow', 'Raid', 'Charm',
                       'Spdlog', 'logger', 'Result', 'db', 'configManager', 'kv', 'Bestiary', 'Blessings'}) do
  _G[name] = inert
end

local forced = nil
math.random = function(a, b)
  table.insert(rec.randoms, {a, b})
  if forced ~= nil then return forced end
  if b == nil then return 1 end
  return a
end

local player = setmetatable({}, {__index = function(_, method)
  return function(self, ...)
    table.insert(rec.player_calls, method)
    if method == 'getLevel' then return var('level') end
    if method == 'getMagicLevel' or method == 'getBaseMagicLevel' then return var('magic_level') end
    if method == 'getEffectiveSkillLevel' or method == 'getSkillLevel' then
      return var('skill:' .. string.gsub(tostring((...)), '^@', ''))
    end
    if method == 'calculateFlatDamageHealing' then return node('fn:flat_damage_healing', var('level')) end
    return var('runtime:' .. method)
  end
end})

local function formula(fn, names)
  local args = {}
  for i, name in ipairs(names) do args[i] = var(name) end
  local ok, a, b = pcall(fn, player, unpack(args))
  if not ok then return {error = tostring(a)} end
  return {minimum = a, maximum = b}
end

local function cast(spell, value)
  forced = value
  rec.executed, rec.randoms, rec.player_calls = {}, {}, {}
  local ok, result = pcall(spell.onCastSpell, player, {}, false)
  forced = nil
  return ok, result
end

local function install(named)
  for luaName, nodeName in pairs(named) do
    rawset(_G, luaName, function(...) return node('fn:' .. nodeName, ...) end)
  end
end

setmetatable(_G, {__index = function(_, k) return '@' .. k end})
return rec, cast, formula, install, isSym
'''


def git_blob_sha1(data):
    return hashlib.sha1(b'blob %d\0' % len(data) + data).hexdigest()


def enum_names(root):
    names = set()
    for header in ENUM_HEADERS:
        path = root / header
        if not path.exists():
            continue
        text = path.read_text(encoding='utf-8', errors='replace')
        for body in re.findall(r'enum(?:\s+class)?\s+\w+[^{;]*\{(.*?)\};', text, re.S):
            names.update(re.findall(r'^\s*([A-Z][A-Z0-9_]+)\b', body, re.M))
    return names


def library(root):
    """Area tables and top-level helper functions of the spell library, as Lua source."""
    text = (root / SPELL_LIB).read_text(encoding='utf-8', errors='replace')
    areas = re.findall(r'^[A-Z]\w* = \{.*?^\}', text, re.M | re.S)
    helpers = [block for block in re.findall(r'^function [a-zA-Z_]\w*\(.*?^end', text, re.M | re.S)
               if re.match(r'function (\w+)', block).group(1) not in NAMED_FUNCTIONS]
    return '\n'.join(areas), helpers


def to_python(value, is_sym=None, as_list=False):
    if is_sym is not None and is_sym(value):
        return expression(value, is_sym)
    if hasattr(value, 'items'):
        keys = list(value.keys())
        if as_list and all(isinstance(k, int) and k > 0 for k in keys):
            return [to_python(value[k], is_sym) if k in keys else None for k in range(1, max(keys, default=0) + 1)]
        if keys and all(isinstance(k, int) for k in keys) and sorted(keys) == list(range(1, len(keys) + 1)):
            return [to_python(value[k], is_sym) for k in sorted(keys)]
        if not keys:
            return []
        return {str(k): to_python(v, is_sym) for k, v in value.items() if not str(k).startswith('__')}
    if callable(value) and not isinstance(value, (str, bytes)):
        return '<function>'
    return value


def number(value):
    """Shortest round-trip decimal of a Lua number (the source literal for ordinary literals)."""
    if isinstance(value, bool):
        raise ValueError('boolean is not a number')
    if isinstance(value, int) or float(value).is_integer():
        return str(int(value))
    return repr(float(value))


def expression(sym, is_sym):
    op = sym['op']
    if op == 'const':
        return {'const': number(sym['value'])}
    if op == 'var':
        return {'var': sym['name']}
    args = [expression(sym['args'][k], is_sym) for k in sorted(sym['args'].keys())]
    if op.startswith('fn:'):
        return {'fn': op[3:], 'args': args}
    return {'op': op, 'args': args}


def to_expression(value, is_sym):
    if is_sym(value):
        return expression(value, is_sym)
    if isinstance(value, (int, float)) and not isinstance(value, bool):
        return {'const': number(value)}
    raise ValueError(f'formula returned {type(value).__name__}')


def negate(expr):
    """-expr with the negation pushed inward where exact (for damage magnitudes)."""
    if 'const' in expr:
        return {'const': number(-float(expr['const'])) if '.' in expr['const'] or 'e' in expr['const'] else str(-int(expr['const']))}
    op = expr.get('op')
    if op == 'neg':
        return expr['args'][0]
    if op in ('mul', 'div'):
        left, right = expr['args']
        if left.get('op') == 'neg' or 'const' in left and left['const'].startswith('-'):
            return {'op': op, 'args': [negate(left), right]}
        if right.get('op') == 'neg' or 'const' in right and right['const'].startswith('-'):
            return {'op': op, 'args': [left, negate(right)]}
    if op == 'floor' and expr['args'][0].get('op') == 'neg':
        return {'op': 'ceil', 'args': [expr['args'][0]['args'][0]]}
    if op == 'ceil' and expr['args'][0].get('op') == 'neg':
        return {'op': 'floor', 'args': [expr['args'][0]['args'][0]]}
    return {'op': 'neg', 'args': [expr]}


def variables(expr, found=None):
    found = set() if found is None else found
    if 'var' in expr:
        found.add(expr['var'])
    for arg in expr.get('args', []):
        variables(arg, found)
    return found


def functions(expr, found=None):
    found = set() if found is None else found
    if 'fn' in expr:
        found.add(expr['fn'])
    if expr.get('op') in ('floor', 'ceil', 'sqrt', 'abs', 'max', 'min', 'mod', 'pow'):
        found.add(expr['op'])
    for arg in expr.get('args', []):
        functions(arg, found)
    return found


def cast_body(text, variable):
    match = re.search(r'function\s+' + re.escape(variable) + r'[.:]onCastSpell\s*\([^)]*\)(.*?)\nend\b', text, re.S)
    if not match:
        match = re.search(r'function\s+[\w.:]*onCastSpell\s*\([^)]*\)(.*?)\nend\b', text, re.S)
    return re.sub(r'\s+', ' ', match.group(1)).strip() if match else None


EXECUTE = r'return (\w+):execute\((?:creature|cid|player), (?:var|variant)\)'
RANDOM_PICK = (r'return \w+\[math\.random\([^)]*\)\]:execute\((?:creature|cid|player), (?:var|variant)\)',
               r'local (\w+) = \w+\[math\.random\([^)]*\)\] return \1:execute\((?:creature|cid|player), (?:var|variant)\)')
CONJURE = r'return (?:creature|player|cid):conjureItem\(([^)]*)\)'


# Pattern hints for `custom` bodies (a body may carry several). They group the work; they are not a
# resolution: every custom spell still needs a native behaviour decision (monster D13 rule).
CUSTOM_PATTERNS = (
    ('wheel_of_destiny', r'WOD\(|getWheelSpell|WheelOfDestiny|upgradeSpellsWOD|revelationStageWOD'),
    ('party', r'getParty\(|addPartyCondition'),
    ('house', r'getHouse\(|setEditHouse|house_|HouseDoor|kickPlayer'),
    ('familiar', r'CreateFamiliarSpell|familiar'),
    ('summons_share_condition', r'getSummons\(\)'),
    ('player_parameter', r'variant:getString\(\)|var:getString\(\)'),
    ('monk_harmony_virtue', r'Harmony|Serene|setVirtue|Virtue_'),
    ('stance', r'getElementalStance|getStance\(|setStance\('),
    ('equipment_dependent', r'getSlotItem\(|getEquippedShield|ElementalBond'),
    ('delayed_or_repeated', r'addEvent\('),
    ('target_position', r'var:getPosition\(\)|variant:getPosition\(\)'),
    ('item_grant', r'addItem\('),
    ('world_query', r'Game\.getSpectators|Tile\(|getTopDownItem|getItems\(\)|ForgeMonster'),
    ('conditional_self_state', r'getCondition\(|removeCondition\('),
    ('target_default', r'getTarget\(\)'),
    ('extra_presentation_only', r'^creature:getPosition\(\):sendMagicEffect\(\w+\) return \w+:execute\(creature, (?:var|variant)\)$'
     r'|^if not \w+:execute\(creature, var\) then return false end creature:getPosition\(\):sendMagicEffect\(\w+\) return true$'),
    ('caster_restriction', r'getVocation\(\)|Monster\(var:getNumber|var:getNumber\(\) == creature:getId\(\)'),
)


def custom_patterns(body):
    return [name for name, pattern in CUSTOM_PATTERNS if body and re.search(pattern, body)] or ['other']


def cast_tier(body):
    if body is None:
        return 'custom', 'no onCastSpell body found'
    if re.fullmatch(EXECUTE, body):
        return 'plain_combat', None
    if any(re.fullmatch(pattern, body) for pattern in RANDOM_PICK):
        return 'random_combat', None
    match = re.fullmatch(CONJURE, body)
    if match:
        return 'conjure', None
    return 'custom', body[:240]


def conjure_arguments(body):
    match = re.fullmatch(CONJURE, body or '')
    if not match:
        return None
    parts = [p.strip() for p in match.group(1).split(',')]
    values = []
    for part in parts:
        values.append(int(part) if re.fullmatch(r'-?\d+', part) else part)
    keys = ('reagent_item_id', 'result_item_id', 'count', 'effect')
    return {k: v for k, v in zip(keys, values)}


def registrar(calls):
    """method -> list of argument lists, in call order; `vocation` is flattened."""
    out = {}
    for method, args in calls:
        if method in ('register', 'onCastSpell'):
            continue
        out.setdefault(method, []).append(args)
    fields = {}
    for method, occurrences in out.items():
        if method == 'vocation':
            fields['vocation'] = [a for args in occurrences for a in args]
        elif len(occurrences) == 1:
            args = occurrences[0]
            fields[method] = args[0] if len(args) == 1 else args
        else:
            fields[method] = occurrences
    return fields


def constant(value):
    return value[1:] if isinstance(value, str) and value.startswith('@') else value


def combat_view(combat, lua_formula, is_sym, known, undefined):
    params, callbacks, conditions, areas, formulas = {}, [], [], [], []
    for method, args in [(c[0], c[1]) for c in combat['calls']]:
        if method == 'setParameter' and len(args) >= 2:
            key, value = constant(args[0]), constant(args[1])
            for name in (key, value):
                if isinstance(name, str) and re.fullmatch(r'[A-Z][A-Z0-9_]+', name) and name not in known:
                    undefined.add(name)
            params[key] = value
        elif method == 'setArea':
            area = args[0] if args else None
            areas.append(area)
        elif method == 'setCallback':
            kind = constant(args[0]) if args else None
            entry = {'kind': kind, 'function': args[1] if len(args) > 1 else None}
            if kind in FORMULA_CALLBACKS:
                fn = combat['raw_callbacks'].get(len(callbacks))
                if fn is None:
                    entry['formula'] = {'error': 'callback function not defined when registered'}
                else:
                    result = lua_formula(fn, FORMULA_CALLBACKS[kind])
                    entry['formula'] = resolve_formula(result, kind, is_sym)
            callbacks.append(entry)
        elif method == 'addCondition':
            conditions.append(args[0] if args else None)
        elif method == 'setFormula':
            formulas.append([constant(a) for a in args])
        else:
            params.setdefault('__other', []).append([method, [constant(a) for a in args]])
    view = {'parameters': params}
    if areas:
        view['areas'] = areas
    if callbacks:
        view['callbacks'] = callbacks
    if conditions:
        view['conditions'] = conditions
    if formulas:
        view['set_formula'] = formulas
    return view


def resolve_formula(result, kind, is_sym):
    if 'error' in result:
        error = str(result['error']).splitlines()[0]
        if 'attempt to compare' in error or 'branch on a runtime value' in error:
            error = 'branches on a runtime value (' + error.split(': ', 1)[-1][:120] + ')'
        return {'status': 'unresolved', 'error': error[:200]}
    try:
        minimum = to_expression(result['minimum'], is_sym)
        maximum = to_expression(result['maximum'], is_sym)
    except (ValueError, KeyError, TypeError) as exc:
        return {'status': 'unresolved', 'error': str(exc)[:200]}
    used = variables(minimum) | variables(maximum)
    runtime = sorted(v for v in used if v.startswith('runtime:'))
    view = {'status': 'unresolved' if runtime else 'resolved', 'inputs': sorted(used),
            'minimum': minimum, 'maximum': maximum,
            'functions': sorted(functions(minimum) | functions(maximum))}
    if runtime:
        view['error'] = 'uses runtime values: ' + ', '.join(runtime)
    return view


class Census:
    def __init__(self, root, source):
        self.root = Path(root)
        self.source = source
        self.areas, self.helpers = library(self.root)
        self.known = enum_names(self.root)

    def files(self):
        for base in SPELL_ROOTS:
            for path in sorted((self.root / base).rglob('*.lua')):
                yield path

    def run(self):
        records, excluded, errors = [], [], []
        for path in self.files():
            relative = path.relative_to(self.root).as_posix()
            data = path.read_bytes()
            if relative in EXCLUDED:
                excluded.append({'file': relative, 'blob': git_blob_sha1(data), 'reason': EXCLUDED[relative]})
                continue
            result = self.evaluate(relative, data)
            if 'error' in result:
                errors.append(result)
            else:
                records.extend(result['spells'])
        return records, excluded, errors

    def evaluate(self, relative, data):
        from lupa.luajit21 import LuaRuntime
        text = data.decode('utf-8', errors='replace')
        base = {'source': self.source, 'file': relative, 'blob': git_blob_sha1(data)}
        lua = LuaRuntime(unpack_returned_tuples=True)
        rec, cast, formula, install, is_sym = lua.execute(LUA_SANDBOX)

        def lua_formula(fn, names):
            return formula(fn, lua.table_from(list(names)))
        try:
            lua.execute(self.areas)
        except Exception as exc:  # the library areas are plain tables; a failure is a tool defect
            return {**base, 'error': 'library areas: ' + str(exc).splitlines()[0][:160]}
        for helper in self.helpers:
            try:
                lua.execute(helper)
            except Exception:
                pass
        install(lua.table_from(NAMED_FUNCTIONS))
        try:
            lua.execute(text)
        except Exception as exc:
            return {**base, 'error': 'load: ' + str(exc).splitlines()[0][:200]}
        combats = []
        undefined = set()
        for index in sorted(rec['combats'].keys()):
            combat = rec['combats'][index]
            calls = [(c[1], to_python(c[2], is_sym, True) if c[2] is not None else []) for c in
                     (combat['__calls'][k] for k in sorted(combat['__calls'].keys()))]
            raw_callbacks, count = {}, 0
            for k in sorted(combat['__calls'].keys()):
                call = combat['__calls'][k]
                if call[1] == 'setCallback':
                    args = call[2]
                    raw_callbacks[count] = args[3] if args is not None and 3 in args.keys() else None
                    count += 1
            combats.append(combat_view({'calls': calls, 'raw_callbacks': raw_callbacks}, lua_formula, is_sym,
                                       self.known, undefined))
        conditions = []
        for index in sorted(rec['conditions'].keys()):
            condition = rec['conditions'][index]
            calls = [[c[1], [constant(a) for a in to_python(c[2], is_sym, True)] if c[2] is not None else []] for c in
                     (condition['__calls'][k] for k in sorted(condition['__calls'].keys()))]
            conditions.append({'type': constant(condition['__type']), 'calls': calls})
        spells = []
        for index in sorted(rec['spells'].keys()):
            spell = rec['spells'][index]
            calls = [(c[1], [constant(a) for a in to_python(c[2], is_sym, True)] if c[2] is not None else []) for c in
                     (spell['__calls'][k] for k in sorted(spell['__calls'].keys()))]
            if not any(method == 'register' for method, _ in calls):
                continue
            fields = registrar(calls)
            variable = spell_variable(text, index)
            body = cast_body(text, variable) if variable else cast_body(text, 'spell')
            tier, reason = cast_tier(body)
            ok, _ = cast(spell, None) if spell['onCastSpell'] is not None else (False, None)
            executed = sorted(set(to_python(rec['executed']) or []))
            player_calls = sorted(set(to_python(rec['player_calls']) or []))
            record = {**base, 'spell_type': str(spell['__type']).lower(), 'name': fields.get('name'),
                      'registrar': fields, 'cast': {'tier': tier}, 'combats': combats}
            if reason:
                record['cast']['body'] = reason
            if tier == 'custom':
                record['cast']['patterns'] = custom_patterns(body)
            if tier == 'conjure':
                record['cast']['conjure'] = conjure_arguments(body)
            if executed:
                record['cast']['executed_combats'] = executed
            if player_calls:
                record['cast']['player_calls'] = player_calls
            if not ok:
                record['cast']['stub_run'] = 'failed'
            if conditions:
                record['conditions'] = conditions
            if undefined:
                record['undefined_constants'] = sorted(undefined)
            spells.append(record)
        if not spells:
            return {**base, 'error': 'no registered spell in file'}
        return {**base, 'spells': spells}


def spell_variable(text, index):
    """Lua variable name of the index-th `X = Spell("...")` in the file (1-based)."""
    names = re.findall(r'(\w+)\s*=\s*Spell\(', text)
    return names[index - 1] if 0 < index <= len(names) else None


COMPARED = ('words', 'group', 'vocation', 'level', 'magicLevel', 'mana', 'manaPercent', 'soul', 'cooldown',
            'groupCooldown', 'isPremium', 'range', 'needTarget', 'needDirection', 'needCasterTargetOrDirection',
            'blockWalls', 'isAggressive', 'isSelfTarget', 'needLearn', 'needWeapon', 'runeId', 'charges',
            'allowFarUse', 'isBlocking', 'id', 'basePower')


def spell_key(record):
    return (record['spell_type'], str(record.get('name') or '').lower())


def compare(canary, crystal):
    left = {spell_key(r): r for r in canary}
    right = {spell_key(r): r for r in crystal}
    rows, field_diffs, formula_diffs = [], Counter(), 0
    for key in sorted(set(left) | set(right)):
        a, b = left.get(key), right.get(key)
        if a is None or b is None:
            rows.append({'spell_type': key[0], 'name': key[1], 'only': 'canary' if b is None else 'crystal'})
            continue
        diffs = {}
        for field in COMPARED:
            va, vb = a['registrar'].get(field), b['registrar'].get(field)
            if va != vb:
                diffs[field] = {'canary': va, 'crystal': vb}
                field_diffs[field] += 1
        fa = [c.get('formula') for combat in a['combats'] for c in combat.get('callbacks', [])]
        fb = [c.get('formula') for combat in b['combats'] for c in combat.get('callbacks', [])]
        if fa != fb:
            diffs['formula'] = 'differs'
            formula_diffs += 1
        if a['cast']['tier'] != b['cast']['tier']:
            diffs['cast_tier'] = {'canary': a['cast']['tier'], 'crystal': b['cast']['tier']}
        if diffs:
            rows.append({'spell_type': key[0], 'name': key[1], 'differences': diffs})
    return rows, dict(sorted(field_diffs.items())), formula_diffs


def summary(records, excluded, errors):
    types = Counter(r['spell_type'] for r in records)
    groups = Counter(f"{r['spell_type']}:{r['registrar'].get('group')}" for r in records)
    tiers = Counter(r['cast']['tier'] for r in records)
    formulas = Counter()
    inputs = Counter()
    fns = Counter()
    callback_kinds = Counter()
    for r in records:
        for combat in r['combats']:
            for cb in combat.get('callbacks', []):
                callback_kinds[str(cb['kind'])] += 1
                if 'formula' in cb:
                    formulas[cb['formula']['status']] += 1
                    for name in cb['formula'].get('inputs', []):
                        inputs[name] += 1
                    for name in cb['formula'].get('functions', []):
                        fns[name] += 1
    methods = Counter(m for r in records for m in r['registrar'])
    patterns = Counter(p for r in records for p in r['cast'].get('patterns', []))
    undefined = Counter(n for r in records for n in r.get('undefined_constants', []))
    return {'registered_spells': len(records), 'by_type': dict(sorted(types.items())),
            'by_type_group': dict(sorted(groups.items())), 'cast_tiers': dict(sorted(tiers.items())),
            'custom_patterns': dict(sorted(patterns.items())),
            'formula_callbacks': dict(sorted(formulas.items())), 'formula_inputs': dict(sorted(inputs.items())),
            'formula_functions': dict(sorted(fns.items())), 'callback_kinds': dict(sorted(callback_kinds.items())),
            'registrar_methods': dict(sorted(methods.items())), 'undefined_constants': dict(sorted(undefined.items())),
            'excluded_files': len(excluded), 'unreadable_files': len(errors)}


def write(out, document):
    """One spell per line so that a regenerated census diffs by spell."""
    lines = ['{']
    keys = list(document)
    for i, key in enumerate(keys):
        value = document[key]
        tail = ',' if i + 1 < len(keys) else ''
        if isinstance(value, list):
            lines.append(f'  {json.dumps(key)}: [')
            for j, item in enumerate(value):
                lines.append('    ' + json.dumps(item, ensure_ascii=False, sort_keys=True) + (',' if j + 1 < len(value) else ''))
            lines.append('  ]' + tail)
        else:
            lines.append(f'  {json.dumps(key)}: ' + json.dumps(value, ensure_ascii=False, sort_keys=True) + tail)
    lines.append('}')
    Path(out).write_text('\n'.join(lines) + '\n', encoding='utf-8', newline='\n')


def main(argv=None):
    parser = argparse.ArgumentParser(description=__doc__.splitlines()[0])
    parser.add_argument('command', nargs='?', default='census', choices=('census', 'self-test'))
    parser.add_argument('--canary', type=Path)
    parser.add_argument('--crystal', type=Path)
    parser.add_argument('--out', type=Path, default=Path(__file__).resolve().parent / 'samples' / 'spell-census.json')
    args = parser.parse_args(argv)
    if args.command == 'self-test':
        return self_test()
    if not args.canary or not args.crystal:
        parser.error('--canary and --crystal checkouts are required')
    document = {'schema': 'OTERYN_SPELL_SOURCE_CENSUS/v1',
                'note': 'Evidence only: OTS registrations are OtsHypothesisOnly; nothing here is Game truth.',
                'sources': SOURCES}
    results = {}
    for source, root in (('canary', args.canary), ('crystal', args.crystal)):
        records, excluded, errors = Census(root, source).run()
        results[source] = records
        document[f'{source}_summary'] = summary(records, excluded, errors)
        document[f'{source}_excluded'] = excluded
        document[f'{source}_unreadable'] = errors
    rows, field_diffs, formula_diffs = compare(results['canary'], results['crystal'])
    shared = {spell_key(r) for r in results['canary']} & {spell_key(r) for r in results['crystal']}
    document['comparison_summary'] = {
        'matched': len(shared),
        'only_canary': sum(1 for r in rows if r.get('only') == 'canary'),
        'only_crystal': sum(1 for r in rows if r.get('only') == 'crystal'),
        'with_differences': sum(1 for r in rows if 'differences' in r),
        'field_differences': field_diffs, 'formula_differences': formula_diffs}
    document['comparison'] = rows
    document['canary'] = results['canary']
    document['crystal'] = results['crystal']
    args.out.parent.mkdir(parents=True, exist_ok=True)
    write(args.out, document)
    print(json.dumps({k: document[k] for k in ('canary_summary', 'crystal_summary', 'comparison_summary')}, indent=1))
    return 0


def self_test():
    """Sandbox checks on inline scripts; needs lupa but no checkout."""
    import tempfile
    root = Path(tempfile.mkdtemp())
    (root / 'data/scripts/lib').mkdir(parents=True)
    (root / 'data/scripts/spells/attack').mkdir(parents=True)
    (root / SPELL_LIB).write_text('AREA_X = {\n{1, 3, 1}\n}\n'
                                  'function calculateBaseDamageHealing(level)\n\treturn level\nend\n'
                                  'function spellMagicDamage(basePower, level, maglevel)\n'
                                  '\tlocal levelBonus = calculateBaseDamageHealing(level)\n'
                                  '\treturn levelBonus + (basePower / 25) * maglevel + basePower / 6\nend\n')
    (root / 'data/scripts/spells/attack/a.lua').write_text(
        'local combat = Combat()\ncombat:setParameter(COMBAT_PARAM_TYPE, COMBAT_ICEDAMAGE)\n'
        'combat:setArea(createCombatArea(AREA_X))\n'
        'function onGetFormulaValues(player, level, maglevel)\n\tlocal min = (level / 5) + (maglevel * 1.403) + 8\n'
        '\treturn -min, -math.floor(spellMagicDamage(45, level, maglevel))\nend\n'
        'combat:setCallback(CALLBACK_PARAM_LEVELMAGICVALUE, "onGetFormulaValues")\n'
        'local spell = Spell("instant")\nfunction spell.onCastSpell(creature, var)\n\treturn combat:execute(creature, var)\nend\n'
        'spell:name("Ice Strike")\nspell:words("exori frigo")\nspell:vocation("druid;true", "sorcerer;true")\n'
        'spell:level(15)\nspell:register()\n')
    (root / 'data/scripts/spells/attack/b.lua').write_text(
        'local spell = Spell("instant")\nfunction spell.onCastSpell(creature, variant)\n'
        '\treturn creature:conjureItem(3147, 3155, 3)\nend\nspell:name("Sudden Death Rune")\nspell:register()\n'
        'function onGetFormulaValues(player, level, maglevel)\n\tif level > 5 then return 1, 2 end\nend\n')
    records, _, errors = Census(root, 'test').run()
    assert not errors, errors
    by_name = {r['name']: r for r in records}
    ice = by_name['Ice Strike']
    assert ice['cast']['tier'] == 'plain_combat', ice['cast']
    assert ice['registrar']['vocation'] == ['druid;true', 'sorcerer;true']
    formula = ice['combats'][0]['callbacks'][0]['formula']
    assert formula['status'] == 'resolved', formula
    assert formula['inputs'] == ['level', 'magic_level'], formula
    assert negate(formula['minimum'])['op'] == 'add', formula['minimum']
    assert 'base_damage_healing' in formula['functions'], formula
    assert ice['combats'][0]['areas'][0]['north'] == [[1, 3, 1]], ice['combats'][0]
    conj = by_name['Sudden Death Rune']
    assert conj['cast'] == {'tier': 'conjure', 'conjure': {'reagent_item_id': 3147, 'result_item_id': 3155, 'count': 3},
                            'player_calls': ['conjureItem']}, conj['cast']
    assert negate({'op': 'neg', 'args': [{'var': 'level'}]}) == {'var': 'level'}
    assert negate({'op': 'floor', 'args': [{'op': 'neg', 'args': [{'var': 'x'}]}]}) == {'op': 'ceil', 'args': [{'var': 'x'}]}
    print('spell_census self-test: ok')
    return 0


if __name__ == '__main__':
    sys.exit(main())
