"""Evaluate Canary registered spell scripts into plain data for the converter (D10, D11, D12).

Evidence tooling only. A spell script runs in a stubbed LuaJIT sandbox: Combat, Condition, Spell and
createCombatArea only record their calls, and the spell's onCastSpell is then called with a stub
caster whose methods all return nil (a monster is not a player). Combat:execute records which Combat
ran. math.random is replaced so that every value of a small random range is tried, which turns a
random pick between Combats (P3) into an explicit list of variants. No Canary engine code runs.
"""
import hashlib
import re
import subprocess
from pathlib import Path

SPELL_LIB = 'data/scripts/lib/register_spells.lua'
ENGINE_DEFINITIONS = 'src/creatures/creatures_definitions.hpp'
SCRIPT_DIRS = ('data/scripts', 'data-otservbr-global/scripts')
SHARED_DIRS = ('data/scripts/spells/', 'data/scripts/runes/')
# Monster spells of another pinned OTS checkout (crystal_batch.py); a Canary registration of the same name wins.
EXTRA_SCRIPT_DIRS = ('data-global/scripts/spells/monster',)
MAX_RANDOM_RANGE = 512

LUA_SANDBOX = r'''
local rec = {combats = {}, conditions = {}, spells = {}, executed = {}, randoms = {}, positions = {}, zones = {}}
local function recorder(kind, list)
  local obj = {__kind = kind, __calls = {}, __n = #list}
  table.insert(list, obj)
  return setmetatable(obj, {__index = function(t, method)
    if method == 'execute' and kind == 'Combat' then
      return function(self) table.insert(rec.executed, self); return true end
    end
    return function(self, ...)
      local args = {...}
      args.__arity = select('#', ...)
      table.insert(rawget(t, '__calls'), {method, args})
      return self
    end
  end})
end
Combat = function() return recorder('Combat', rec.combats) end
Condition = function(kind, id) local c = recorder('Condition', rec.conditions); rawset(c, '__type', kind); return c end
Spell = function(kind) local s = recorder('Spell', rec.spells); rawset(s, '__type', kind); return s end
createCombatArea = function(area, ext) return {__kind = 'Area', north = area, ext = ext} end
Position = function(x, y, z)
  if type(x) == 'table' then x, y, z = x.x, x.y, x.z end
  assert(type(x) == 'number' and type(y) == 'number' and type(z) == 'number', 'Position requires literal coordinates')
  assert(#rec.positions < 10000, 'Position capture limit')
  local p = {__kind = 'Position', x = x, y = y, z = z}
  table.insert(rec.positions, p)
  return p
end
Zone = {getByName = function(name)
  assert(type(name) == 'string' and #rec.zones < 64, 'Zone capture limit')
  local z = {__kind = 'Zone', name = name}
  table.insert(rec.zones, z)
  z.getPositions = function(self)
    self.positions_requested = true
    return {__kind = 'DeferredZonePositions', zone = self.name}
  end
  return z
end}
setCombatCallback = function(combat, kind, name) return combat:setCallback(kind, name) end
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
# The chain picker body that keeps only players outside a protection zone (poison_chain.lua and its copies).
PLAYERS_ONLY_PICKER = ('if target:isPlayer() then if target:getPosition():isProtectionZoneTile() then return false end '
                       'return true end return false')


# The player chain picker bodies that keep every creature the caster may hit: not an NPC, not the caster, not in a
# protection zone (chained_penance.lua, forked_*.lua, lightning.lua, spiritual_outburst.lua). combat.cpp canDoCombat
# already rejects all three for an aggressive player cast, so the picker adds no filter.
CASTER_MAY_HIT_PICKERS = (
    'if target:isNpc() or creature == target or target:getTile():hasFlag(TILESTATE_PROTECTIONZONE) then return false '
    'end return true',
    'return not target:isNpc() and creature ~= target and not target:getTile():hasFlag(TILESTATE_PROTECTIONZONE)')


def chain_pickers(text, templates):
    """The CALLBACK_PARAM_CHAINPICKER functions of a script whose body is exactly one of the templates."""
    matched = set()
    for name in set(re.findall(r'setCallback\(\s*CALLBACK_PARAM_CHAINPICKER\s*,\s*"(\w+)"\s*\)', text)):
        body = re.search(r'function\s+' + name + r'\s*\(\s*\w+\s*,\s*target\s*\)(.*?)\nend\b', text, re.S)
        if body and re.sub(r'\s+', ' ', body.group(1)).strip() in templates:
            matched.add(name)
    return matched


def players_only_chain_pickers(text):
    """The CALLBACK_PARAM_CHAINPICKER functions of a script whose body is exactly the players-only template."""
    return chain_pickers(text, (PLAYERS_ONLY_PICKER,))


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


def _registration_masks(text):
    """Offset-preserving comment and string masks; quoted examples are not Lua code."""
    token = re.compile(r'--\[(=*)\[.*?\]\1\]|--[^\n]*|"(?:\\.|[^"\\])*"|\'(?:\\.|[^\'\\])*\'|\[(=*)\[.*?\]\2\]', re.S)
    comments, code = list(text), list(text)
    for match in token.finditer(text):
        start, end = match.span()
        blank = ['\n' if c == '\n' else ' ' for c in text[start:end]]
        code[start:end] = blank
        if match.group().startswith('--'):
            comments[start:end] = blank
    return ''.join(comments), ''.join(code)


def registrations(root, directories):
    found = {}
    for directory in directories:
        for path in sorted((root / directory).rglob('*.lua')):
            text = path.read_text(encoding='utf-8', errors='replace')
            if not re.search(r'\bSpell\s*\(', text):
                continue
            literal, code = _registration_masks(text)
            constructors = list(re.finditer(r'(\w+)\s*=\s*Spell\s*\(', code))
            for index, constructor in enumerate(constructors):
                variable = constructor.group(1)
                kind = re.match(r'\s*["\'](\w+)["\']', literal[constructor.end():])
                if not kind:
                    symbolic = re.match(r'\s*(SPELL_(?:INSTANT|RUNE))\s*\)', literal[constructor.end():])
                    if not symbolic:
                        continue
                kind_name = kind.group(1).lower() if kind else '@' + symbolic.group(1).lower()
                end = next((m.start() for m in constructors[index + 1:] if m.group(1) == variable), len(code))
                for call in re.finditer(r'\b' + re.escape(variable) + r'\s*:\s*name\s*\(', code[constructor.end():end]):
                    offset = constructor.end() + call.end()
                    name = re.match(r'\s*(["\'])(.*?)\1\s*\)', literal[offset:end])
                    if name:
                        found.setdefault(name.group(2).lower(), []).append((kind_name, path))
    return found


def index_spells(canary, extra_roots=()):
    """name -> (kind, path) of every Spell registration, rune spells first (Spells::getSpellByName). `extra_roots`
    (checkouts with the EXTRA_SCRIPT_DIRS) only add names that Canary does not register."""
    found = registrations(canary, SCRIPT_DIRS)
    for root in extra_roots:
        for name, entries in registrations(Path(root), EXTRA_SCRIPT_DIRS).items():
            found.setdefault(name, entries)
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
    if not isinstance(entries, list):
        return []
    result = []
    for method, arguments in entries:
        if isinstance(arguments, dict) and '__arity' in arguments:
            arity = arguments['__arity']
            if not isinstance(arity, int) or not 0 <= arity <= 1024:
                raise ValueError('recorded argument arity exceeds capture limit')
            args = [arguments.get(index) for index in range(1, arity + 1)]
        elif isinstance(arguments, list):
            args = list(arguments)
        else:
            args = []
        result.append((method, args))
    return result


def reference_argument(value):
    """Serialize declaration arguments without serializing or invoking Lua/Python functions."""
    if isinstance(value, dict):
        if value.get('__kind') in ('Combat', 'Condition', 'Spell'):
            return {k: value[k] for k in ('__kind', '__n', '__type') if k in value}
        return {k: reference_argument(v) for k, v in value.items()}
    if isinstance(value, (list, tuple)):
        return [reference_argument(v) for v in value]
    if value is None or isinstance(value, (str, int, float, bool)):
        return value
    return {'reference_type': 'unexecuted_function_or_userdata'}


class SpellScripts:
    def __init__(self, canary, player_chains=False, accepted_guards=None, extra_roots=(),
                 source_git=None, source_revision=None):
        """player_chains (player spells): a caster-may-hit chain picker adds no filter, and the chain value callback,
        which reads the player caster and its Wheel, is not called; the caller supplies the chain parameters.
        accepted_guards (player spells): {spell name: {onCastSpell body}}. A script whose whitespace-collapsed body is
        one of these exact texts and that declares exactly one Combat evaluates to that Combat (P2); the caller
        expresses the guard with spell fields. Any other body stays P4."""
        self.canary = Path(canary)
        self.player_chains = player_chains
        self.accepted_guards = accepted_guards or {}
        self.extra_roots = tuple(Path(r) for r in extra_roots)
        self.index = index_spells(self.canary, self.extra_roots)
        self.areas = area_constants(self.canary)
        self.enums = engine_enums(self.canary)
        self.cache = {}
        self.registration_cache = {}
        self.source_git = Path(source_git) if source_git else None
        self.source_revision = source_revision
        self.dependency_cache = {}

    def _verified_dependency(self, root, relative):
        """Read a bounded dependency only when its bytes equal the immutable local Git blob."""
        cache = self.__dict__.setdefault('dependency_cache', {})
        cache_key = (str(root.resolve()), relative)
        if cache_key in cache:
            return cache[cache_key]
        path = (root / relative).resolve()
        if root.resolve() not in path.parents:
            raise ValueError('dependency outside source snapshot: ' + relative)
        checkout = self.source_git or root
        revision = self.source_revision or subprocess.check_output(
            ['git', '-C', str(checkout), 'rev-parse', 'HEAD'], text=True).strip()
        if not re.fullmatch(r'[0-9a-f]{40}', revision):
            raise ValueError('dependency requires exact Git source revision')
        blob = subprocess.check_output(['git', '-C', str(checkout), 'show', revision + ':' + relative])
        if len(blob) > 262144:
            raise ValueError('dependency exceeds capture size limit')
        if path.is_file():
            data = path.read_bytes()
        elif self.source_git and self.source_revision:
            data = blob
        else:
            raise ValueError('dependency missing from source snapshot: ' + relative)
        if blob != data:
            raise ValueError('dependency differs from pinned Git blob: ' + relative)
        answer = data.decode('utf-8'), {'path': relative, 'revision': revision,
                                      'sha256': hashlib.sha256(data).hexdigest(), 'bytes': len(data)}
        cache[cache_key] = answer
        return answer

    def evaluate_registration(self, name, kind, path):
        """Capture one explicit registration without changing winner selection or its cache."""
        path = Path(path).resolve()
        roots = (self.canary, *self.extra_roots)
        if not any(root.resolve() in path.parents for root in roots):
            raise ValueError('registration outside source roots')
        key = (name.lower(), kind, str(path))
        if key not in self.registration_cache:
            self.registration_cache[key] = self._evaluate(name.lower(), (kind, path))
        return self.registration_cache[key]

    def evaluate(self, name):
        """Plain-data view of the registered spell `name`, or None when no spell has that name."""
        key = name.lower()
        if key not in self.index:
            return None
        if key not in self.cache:
            self.cache[key] = self._evaluate(key)
        return self.cache[key]

    def _evaluate(self, key, registration=None):
        from lupa.luajit21 import LuaRuntime
        kind, path = registration or self.index[key]
        root = next((r for r in self.extra_roots if r in path.parents), self.canary)
        relative = str(path.relative_to(root))
        result = {'name': key, 'kind': kind, 'script': relative, 'shared': relative.startswith(SHARED_DIRS)}
        if root != self.canary:
            result['extra_root'] = True
        def deny_python_attributes(obj, attribute, setting):
            raise AttributeError('Python attributes are unavailable during reference capture')
        lua = LuaRuntime(unpack_returned_tuples=True, max_memory=67108864,
                         register_eval=False, register_builtins=False,
                         attribute_filter=deny_python_attributes)
        rec, cast, _ = lua.execute(LUA_SANDBOX)
        # Loading declarations must not have arbitrary OS/filesystem/Python access.
        lua.execute('python=nil; os=nil; io=nil; package=nil; require=nil; loadfile=nil; jit.off(); '
                    'local n=0; debug.sethook(function() n=n+1; if n>2000 then error("capture instruction limit") end end,"",1000); debug=nil')
        text = path.read_text(encoding='utf-8', errors='replace')
        dependencies = []
        try:
            if 'VOCATION.BASE_ID' in text:
                dependency, receipt = self._verified_dependency(root, 'data/libs/functions/vocation.lua')
                match = re.search(r'^VOCATION\s*=\s*(\{.*?^\})', dependency, re.M | re.S)
                if not match or not re.fullmatch(r'[\s\w{},=\d]*', match.group(1)):
                    raise ValueError('VOCATION initializer is not a literal table')
                lua.execute('VOCATION = ' + match.group(1))
                dependencies.append(receipt)
            # The single accepted include is a same-pack literal declaration, not arbitrary dofile.
            pack = relative.split('/')[0]
            include_path = pack + '/scripts/spells/monster/gaz_functions.lua'
            loaded = set()
            def include(request):
                if request != include_path or request in loaded:
                    raise ValueError('include not allowlisted or repeated: ' + str(request))
                dependency, receipt = self._verified_dependency(root, request)
                if not re.fullmatch(r'\s*GazVariables\s*=\s*\{\s*MinionsNow\s*=\s*\d+\s*,\s*MaxSummons\s*=\s*\d+\s*,?\s*\}\s*', dependency):
                    raise ValueError('gaz_functions include is not the literal declaration')
                loaded.add(request)
                dependencies.append(receipt)
                return lua.execute(dependency)
            lua.globals()['DATA_DIRECTORY'] = pack
            lua.globals()['dofile'] = include
            lua.execute(self.areas)
            lua.execute(text)
        except Exception as exc:
            captured = {c['__n']: self._combat(lua, c, reference_only=True) for c in rec['combats'].values()}
            return {**result, 'error': 'load: ' + str(exc).splitlines()[0][:160],
                    'reference_combats': captured, 'reference_dependencies': dependencies,
                    'reference_capture_complete': False}
        instances = list(rec['spells'].values())
        combats = list(rec['combats'].values())
        result['reference_dependencies'] = dependencies
        result['reference_positions'] = [to_python(p) for p in rec['positions'].values()]
        result['reference_zones'] = [dict((k, to_python(v)) for k, v in z.items() if k != 'getPositions')
                                     for z in rec['zones'].values()]
        result['reference_combats'] = {c['__n']: self._combat(lua, c, reference_only=True) for c in combats}
        result['reference_capture_complete'] = True
        result['reference_spell_instances'] = [
            {'instance_index': s['__n'], 'kind': s['__type'],
             'call_sequence': [{'method': m, 'args': reference_argument(a)} for m, a in calls(s)]} for s in instances]
        matching = [s for s in instances
                    if any(m == 'name' and a and str(a[0]).lower() == key for m, a in calls(s))]
        if len(matching) > 1:
            return {**result, 'error': 'ambiguous multiple Spell instances with this name',
                    'reference_capture_complete': True,
                    'reference_combats': {c['__n']: self._combat(lua, c, reference_only=True)
                                          for c in rec['combats'].values()}}
        spell = matching[0] if matching else None
        if spell is None or spell['onCastSpell'] is None:
            return {**result, 'error': 'no onCastSpell for this name'}
        result['spell_calls'] = {m: a for m, a in calls(spell) if m not in ('name', 'words', 'register')}
        result['spell_instance_index'] = spell['__n']
        result['spell_call_sequence'] = [{'method': m, 'args': reference_argument(a)} for m, a in calls(spell)]
        callbacks = {str(a[0]).lstrip('@') for c in combats for m, a in calls(c) if m == 'setCallback' and a}
        text = path.read_text(encoding='utf-8', errors='replace')
        players_only = players_only_chain_pickers(text)
        neutral = chain_pickers(text, CASTER_MAY_HIT_PICKERS) if self.player_chains else set()
        pickers = {str(a[1]) for c in combats for m, a in calls(c)
                   if m == 'setCallback' and len(a) >= 2 and str(a[0]).lstrip('@') == 'CALLBACK_PARAM_CHAINPICKER'}
        if pickers and pickers <= players_only | neutral:
            # Every chain picker of the script is the players-only template; any other picker keeps the script P4.
            callbacks.discard('CALLBACK_PARAM_CHAINPICKER')
        result['tier'], result['tier_reasons'] = body_tier(text, result['shared'], callbacks)
        if kind.startswith('@') or path.name.startswith('#'):
            result['tier'] = 'P4'
            result['tier_reasons'].append('source-disabled/example or symbolic constructor: reference capture only')
            result['source_disabled_reference_only'] = True
            return result
        if result['tier'] == 'P4' and cast_body(text) in self.accepted_guards.get(key, ()) and len(combats) == 1:
            result['tier'], result['tier_reasons'] = 'P2', ['accepted guard, expressed by the caller: ' + cast_body(text)[:120]]
            result['variants'] = [0]
            try:
                result['combats'] = {0: self._combat(lua, combats[0])}
            except Exception as exc:
                result.pop('variants', None)
                result['error'] = 'combat evaluation: ' + str(exc).splitlines()[0][:160]
            return result
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
            if low == high:
                # LuaJIT math.random(m, n) returns floor(r * (n - m + 1)) + m, which is m itself when m == n.
                values = [low]
            elif not (isinstance(low, int) and isinstance(high, int)) or high - low > MAX_RANDOM_RANGE:
                return {**result, 'error': f'random range {ranges[0]} is not a small integer range'}
            else:
                values = range(low, high + 1)
            for value in values:
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
        try:
            result['combats'] = {n: self._combat(lua, combats[n]) for n in sorted(set(variants))}
        except Exception as exc:
            result.pop('variants', None)
            return {**result, 'error': 'combat evaluation: ' + str(exc).splitlines()[0][:160]}
        for combat in result['combats'].values():
            if combat['callbacks'].get('CALLBACK_PARAM_CHAINPICKER') in players_only:
                combat['chain_target_filter'] = 'players'
            elif combat['callbacks'].get('CALLBACK_PARAM_CHAINPICKER') in neutral:
                del combat['callbacks']['CALLBACK_PARAM_CHAINPICKER']
        return result

    def _combat(self, lua, combat, reference_only=False):
        data = {'params': {}, 'param_calls': [], 'callbacks': {}, 'conditions': [], 'area': None, 'formula': None}
        data['call_sequence'] = [{'method': method, 'args': reference_argument(args)} for method, args in calls(combat)]
        for method, args in calls(combat):
            if method == 'setParameter' and len(args) >= 2:
                data['params'][str(args[0]).lstrip('@')] = args[1].lstrip('@') if isinstance(args[1], str) else args[1]
                data['param_calls'].append([str(args[0]).lstrip('@'), args[1].lstrip('@') if isinstance(args[1], str) else args[1]])
            elif method == 'setArea' and args and isinstance(args[0], dict):
                data['area'] = {'north': args[0].get('north'), 'diagonal': args[0].get('ext')}
            elif method == 'setCallback' and len(args) >= 2:
                callback = str(args[0]).lstrip('@')
                data['callbacks'][callback] = args[1]
                if callback == 'CALLBACK_PARAM_CHAINVALUE' and not self.player_chains and not reference_only:
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
