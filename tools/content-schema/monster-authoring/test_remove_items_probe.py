"""Focused probe cases for D18 `remove_items` `top_item_first_tile` (SW-2); synthetic scripts, needs lupa==2.8."""
import tempfile
import unittest
from pathlib import Path

import canary_batch as cb
import spell_probes

# The destroy_magic_walls.lua scan (Canary 47dfd51f); REMOVE is replaced per case.
SCRIPT = '''
local ids = {ITEM_MAGICWALL_SAFE, ITEM_MAGICWALL, ITEM_WILDGROWTH_SAFE, ITEM_WILDGROWTH}
local spell = Spell("instant")
function spell.onCastSpell(creature, var)
  local position = creature:getPosition()
  for x = -2, 2 do
    for y = -2, 2 do
      local tile = Tile(position.x + x, position.y + y, position.z)
      if tile then
        local item = tile:getTopVisibleThing()
        if item and table.contains(ids, item:getId()) then
          REMOVE
        end
      end
    end
  end
  return true
end
spell:name("probe case")
spell:register()
'''
CANARY = 'item:remove() position:sendMagicEffect(CONST_ME_POFF) return true'


def probe(remove):
    with tempfile.TemporaryDirectory() as directory:
        (Path(directory) / 'case.lua').write_text(SCRIPT.replace('REMOVE', remove), encoding='utf-8')
        loaded = spell_probes.Probe(directory, 'case.lua', '', cb.PROBE_ITEM_CONSTANTS)
    lua_spell = loaded.spell('probe case')
    loaded.world['items'] = loaded.lua.table()
    caster = loaded.make('monster', 'caster', None, 0)
    loaded.creatures['caster'] = caster
    _, empty = loaded.run(lua_spell['onCastSpell'], caster, loaded.lua.table())
    return cb.probe_top_item_first_tile(loaded, lua_spell, empty)


class TopItemFirstTile(unittest.TestCase):
    def test_canary_scan_resolves(self):
        order, listed, effects = probe(CANARY)
        self.assertEqual(order, [(x, y, 0) for x in range(-2, 3) for y in range(-2, 3)])
        self.assertEqual(listed, [10181, 2128, 10182, 2130])
        self.assertEqual(len(effects), 1)

    def test_more_than_one_removal_stays_unresolved(self):
        with self.assertRaises(cb.SpellUnresolved):
            probe('item:remove() item:remove() return true')

    def test_scan_continuing_after_removal_stays_unresolved(self):
        with self.assertRaises(cb.SpellUnresolved):
            probe('item:remove()')


if __name__ == '__main__':
    unittest.main()
