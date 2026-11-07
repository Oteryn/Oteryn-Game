"""Probe the behaviour of custom-logic Canary spell scripts against stub worlds (D18).

Evidence tooling only. A script is loaded in a LuaJIT sandbox whose Combat/Condition/Spell objects record
their calls (as in spell_scripts.py) and whose world API is a set of stubs: the caster, players, player
summons, monsters, named monsters, tiles and Game.createMonster only record what the script does with
them. Each probe runs one entry point (onCastSpell or a Combat callback) against one prepared situation;
the caller derives authoring data only when the recorded behaviour matches a known model exactly.
No Canary engine code runs.
"""
from pathlib import Path

import spell_scripts as ss

LUA_WORLD = r'''
local log = {}
local function record(...) table.insert(log, {...}) end
local mode = {random = 'low'}
math.random = function(a, b)
  if b == nil then
    if a == nil then return 0 end
    a, b = 1, a
  end
  record('random', a, b)
  if mode.random == 'high' then return b end
  return a
end
table.contains = function(t, v)
  for _, x in pairs(t) do if x == v then return true end end
  return false
end
local function position(x, y, z)
  if type(x) == 'table' then x, y, z = x.x, x.y, x.z end
  return setmetatable({x = x, y = y, z = z}, {__index = {
    sendMagicEffect = function(self, effect) record('effect', self.x, self.y, self.z, effect) end}})
end
Position = position
local creatures = {}
local function creature(kind, name, master, summons)
  -- State lives outside the proxy table so an absent field (a nil master) never reaches __index.
  local s = {kind = kind, name = name, master = master, summons = summons or 0, pos = position(1000, 1000, 7)}
  local c = {}
  local methods = {
    isPlayer = function(self) return s.kind == 'player' end,
    isMonster = function(self) return s.kind == 'monster' end,
    getMaster = function(self) return s.master end,
    getName = function(self) return s.name end,
    getPosition = function(self) return s.pos end,
    getId = function(self) return 1 end,
    getSummons = function(self) local t = {} for i = 1, s.summons do t[i] = {} end return t end,
    addHealth = function(self, amount) record('addHealth', s.name, amount) end,
    say = function(self, text) record('say', text) end,
    remove = function(self) record('remove', s.name) end,
    setMaster = function(self, master) record('setMaster', s.name, master == creatures.caster) end,
  }
  return setmetatable(c, {__index = function(t, k)
    if methods[k] then return methods[k] end
    return function(...) record('call', k); return nil end
  end})
end
-- items.top is the id of the top visible item; top_at = {x, y, z} limits it to that one tile (relative to the caster).
local world = {top = nil, items = {}, top_at = nil}
Tile = function(x, y, z)
  local p = position(x, y, z)
  record('tile', p.x - 1000, p.y - 1000, p.z - 7)
  return {
    getTopCreature = function(self) return world.top end,
    getItemById = function(self, id)
      record('getItemById', p.x - 1000, p.y - 1000, p.z - 7, id)
      if world.items[id] then
        return {remove = function() record('removeItem', p.x - 1000, p.y - 1000, p.z - 7, id) end, getId = function() return id end}
      end
      return nil
    end,
    getTopVisibleThing = function(self)
      local id, at = world.items.top, world.top_at
      if at and (at[1] ~= p.x - 1000 or at[2] ~= p.y - 1000 or at[3] ~= p.z - 7) then id = nil end
      if id then return {getId = function() return id end, remove = function() record('removeItem', p.x - 1000, p.y - 1000, p.z - 7, id) end} end
      return nil
    end,
  }
end
Game = {createMonster = function(name, pos)
  record('createMonster', name, pos.x - 1000, pos.y - 1000, pos.z - 7)
  return creature('monster', name, nil)
end}
doTargetCombatHealth = function(cid, target, kind, min, max, effect)
  record('combatHealth', target:getName(), tostring(kind), min, max)
end
addEvent = function() record('addEvent') end
Creature = function() return nil end
return log, mode, world, creature, creatures
'''


class Probe:
    """One loaded spell script with its stub world."""

    def __init__(self, canary, relative, areas, constants):
        from lupa.luajit21 import LuaRuntime
        self.lua = LuaRuntime(unpack_returned_tuples=True)
        self.rec, self.cast, _ = self.lua.execute(ss.LUA_SANDBOX)
        self.log, self.mode, self.world, self.creature, self.creatures = self.lua.execute(LUA_WORLD)
        self.lua.execute(areas)
        for name, value in constants.items():
            self.lua.globals()[name] = value
        self.source = (Path(canary) / relative).read_text(encoding='utf-8', errors='replace')
        self.lua.execute(self.source)

    def spell(self, name):
        return next((s for s in self.rec['spells'].values()
                     if any(m == 'name' and a and str(a[0]).lower() == name for m, a in ss.calls(s))), None)

    def entries(self):
        return [ss.to_python(e) for e in self.log.values()]

    def reset(self):
        for key in list(self.log.keys()):
            self.log[key] = None

    def run(self, function, *args, random='low'):
        """Call `function` with `args`; returns (ok, recorded world calls)."""
        self.reset()
        self.mode['random'] = random
        self.rec['executed'] = self.lua.table()
        try:
            function(*args)
            ok = True
        except Exception:  # a stub the script needs is missing: the behaviour is not modelled
            ok = False
        return ok, self.entries()

    def make(self, kind, name, master=None, summons=0):
        return self.creature(kind, name, master, summons)
