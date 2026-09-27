"""Evaluate one declarative Canary/Crystal NPC Lua file in a stubbed LuaJIT sandbox (lupa).

Evidence tooling only. The sandbox loads the pinned `keyword_handler.lua` (a pure data-structure
library from the same checkout), the pinned `string.lua` helpers and the pinned `custom_modules.lua`
(greet/farewell/spell keyword helpers, evaluated against the stubs), then runs the NPC file with
every engine global replaced by a recording stub:

- `Game.createNpcType(name)` returns a recorder; `npcType:register(config)` captures the config.
- `NpcHandler:new(...)` records `setMessage`, `setCallback` and `addModule`; every other method is a no-op.
- `Position(x, y, z)` returns a plain position table.
- any other global (StdModule.*, Storage.*, MESSAGE_*, SPEECHBUBBLE_*, ...) resolves to a symbol that
  remembers its dotted name, so `StdModule.travel` is recorded as the symbol `StdModule.travel`.

Only top-level file code runs. Callback bodies (onSay, creatureSayCallback, keyword conditions and
actions) are never called; they are recorded as opaque Lua functions. `os`, `io`, `require`,
`dofile`, `loadfile`, `package` and `debug` are removed from the environment, the JIT is off and an
instruction-count hook aborts a file that runs longer than a fixed budget.
"""
from pathlib import Path

PRELUDE = r'''
local lib_keyword, lib_string, lib_custom = ...
if jit then jit.off() end
debug.sethook(function() error("instruction budget exceeded") end, "", 20000000)
os, io, require, dofile, loadfile, package, debug = nil, nil, nil, nil, nil, nil, nil

logger = { error = function() end, warn = function() end, info = function() end, debug = function() end }

local SYM = {}
local function sym(name)
  return setmetatable({ __sym = name }, SYM)
end
local function symname(v) return rawget(v, "__sym") end
local function text(v)
  if type(v) == "table" and getmetatable(v) == SYM then return "<" .. symname(v) .. ">" end
  return tostring(v)
end
SYM.__index = function(t, k) return sym(symname(t) .. "." .. tostring(k)) end
SYM.__call = function(t, ...) return sym(symname(t) .. "()") end
SYM.__concat = function(a, b) return text(a) .. text(b) end
SYM.__tostring = function(t) return "<" .. symname(t) .. ">" end
local function expr() return sym("expr") end
SYM.__add, SYM.__sub, SYM.__mul, SYM.__div, SYM.__mod, SYM.__pow, SYM.__unm = expr, expr, expr, expr, expr, expr, expr
SYM.__len = function() return 0 end

lib_string()
lib_keyword()

local record = { configs = {}, npc_type_names = {}, events = {}, type_calls = {}, messages = {},
                 callbacks = {}, modules = {}, handlers = {}, positions = {} }

function Position(x, y, z, stackpos)
  local p = { x = x, y = y, z = z, __position = true }
  return p
end

local function recorder(kind)
  local obj = {}
  return setmetatable(obj, {
    __index = function(t, k)
      return function(self, ...)
        record.type_calls[#record.type_calls + 1] = { kind = kind, name = k, args = { ... } }
        return nil
      end
    end,
  })
end

Game = setmetatable({
  createNpcType = function(name)
    record.npc_type_names[#record.npc_type_names + 1] = name
    local npcType = {}
    return setmetatable(npcType, {
      __index = function(t, k)
        if k == "register" then
          return function(self, config) record.configs[#record.configs + 1] = config end
        end
        return function(self, ...)
          record.type_calls[#record.type_calls + 1] = { kind = "npcType", name = k, args = { ... } }
        end
      end,
      __newindex = function(t, k, v) record.events[#record.events + 1] = k end,
    })
  end,
}, { __index = function(t, k) return sym("Game." .. tostring(k)) end })

NpcHandler = {}
function NpcHandler:new(keywordHandler)
  local handler = { keywordHandler = keywordHandler }
  record.handlers[#record.handlers + 1] = handler
  return setmetatable(handler, {
    __index = function(t, k)
      if k == "setMessage" then
        return function(self, id, message) record.messages[#record.messages + 1] = { id = id, text = message } end
      elseif k == "setCallback" then
        return function(self, id, fn) record.callbacks[#record.callbacks + 1] = { id = id, fn = fn } end
      elseif k == "addModule" then
        return function(self, module, ...) record.modules[#record.modules + 1] = { module = module, args = { ... } } end
      end
      return function() return nil end
    end,
  })
end

setmetatable(_G, { __index = function(t, k) return sym(k) end })
lib_custom()

return record, sym, SYM
'''


def _lua_value(value, lua_type, sym_meta, depth=0, seen=None):
    """Convert a Lua value into plain Python. Symbols become {'symbol': name}; positions become
    {'x','y','z'}; functions become {'lua_function': True}; tables keep array or map shape."""
    if seen is None:
        seen = set()
    kind = lua_type(value)
    if kind in ('nil', None):
        return None
    if kind in ('number', 'string', 'boolean'):
        if isinstance(value, float) and value.is_integer():
            return int(value)
        return value
    if kind == 'function':
        return {'lua_function': True}
    if kind != 'table':
        return {'lua_value': kind}
    if depth > 24 or id(value) in seen:
        return {'cycle': True}
    seen = seen | {id(value)}
    rawget = sym_meta
    if rawget(value, '__sym') is not None:
        return {'symbol': rawget(value, '__sym')}
    if rawget(value, '__position'):
        return {'x': value['x'], 'y': value['y'], 'z': value['z']}
    keys = list(value.keys())
    if keys and all(isinstance(k, int) for k in keys) and sorted(keys) == list(range(1, len(keys) + 1)):
        return [_lua_value(value[k], lua_type, sym_meta, depth + 1, seen) for k in sorted(keys)]
    result = {}
    for key in keys:
        if key in ('npcHandler', 'parent'):
            continue
        result[str(key) if not isinstance(key, str) else key] = _lua_value(value[key], lua_type, sym_meta, depth + 1, seen)
    return result


def _keyword_tree(node, lua_type, sym_meta, depth=0):
    children = []
    for index in sorted(node['children'].keys()) if node['children'] else []:
        child = node['children'][index]
        children.append({
            'keywords': _lua_value(child['keywords'], lua_type, sym_meta),
            'callback': _lua_value(child['callback'], lua_type, sym_meta),
            'parameters': _lua_value(child['parameters'], lua_type, sym_meta),
            'condition': lua_type(child['condition']) == 'function',
            'action': lua_type(child['action']) == 'function',
            'children': _keyword_tree(child, lua_type, sym_meta, depth + 1) if depth < 16 else [],
        })
    return children


def load_npc(path, npclib_dir, string_lib):
    """Evaluate one NPC file. Returns a dict with configs, keyword trees, messages, callbacks,
    modules, events and recorded npcType calls. Raises on Lua errors before `register`."""
    from lupa.luajit21 import LuaRuntime
    lua = LuaRuntime(unpack_returned_tuples=True)
    lib_keyword = lua.compile(Path(npclib_dir, 'keyword_handler.lua').read_text(encoding='utf-8'))
    lib_string = lua.compile(Path(string_lib).read_text(encoding='utf-8'))
    lib_custom = lua.compile(Path(npclib_dir, 'custom_modules.lua').read_text(encoding='utf-8'))
    record, _sym, _meta = lua.execute(PRELUDE, lib_keyword, lib_string, lib_custom)
    lua_type = lua.globals().type
    sym_meta = lua.globals().rawget
    error = None
    try:
        lua.execute(Path(path).read_text(encoding='utf-8'))
    except Exception as exc:  # a file that already registered keeps its registration, as in Canary
        error = str(exc).splitlines()[0][:200]
        if not record['configs'] or len(record['configs']) == 0:
            raise
    handlers = [record['handlers'][k] for k in sorted(record['handlers'].keys())]
    return {
        'npc_type_names': _lua_value(record['npc_type_names'], lua_type, sym_meta) or [],
        'configs': _lua_value(record['configs'], lua_type, sym_meta) or [],
        'keywords': [_keyword_tree(h['keywordHandler']['root'], lua_type, sym_meta)
                     for h in handlers if lua_type(h['keywordHandler']) == 'table' and h['keywordHandler']['root'] is not None],
        'messages': _lua_value(record['messages'], lua_type, sym_meta) or [],
        'callbacks': _lua_value(record['callbacks'], lua_type, sym_meta) or [],
        'modules': _lua_value(record['modules'], lua_type, sym_meta) or [],
        'events': _lua_value(record['events'], lua_type, sym_meta) or [],
        'type_calls': _lua_value(record['type_calls'], lua_type, sym_meta) or [],
        'error_after_register': error,
    }
