"""Focused cases for the SW-1 `Ability.windup` template match; synthetic scripts, needs lupa==2.8."""
import tempfile
import unittest
from pathlib import Path

import canary_batch as cb
import spell_probes

# The soulwars_fear.lua shape (Canary 47dfd51f); GUARD and CAST are replaced per case.
SCRIPT = '''
local combat = Combat()
combat:setParameter(COMBAT_PARAM_EFFECT, CONST_ME_BLUE_GHOST)
local spell = Spell("instant")
local function executeCombat(cid, var)
  local creature = Creature(cid)
  GUARD
  return combat:execute(creature, var)
end
function spell.onCastSpell(creature, var)
  CAST
  return true
end
spell:name("probe case")
spell:register()
'''
GUARD = 'if not creature then return end'
CAST = 'creature:getPosition():sendMagicEffect(CONST_ME_GHOST_SMOKE) addEvent(executeCombat, 2000, creature:getId(), var)'


def match(guard=GUARD, cast=CAST):
    with tempfile.TemporaryDirectory() as directory:
        (Path(directory) / 'case.lua').write_text(SCRIPT.replace('GUARD', guard).replace('CAST', cast), encoding='utf-8')
        probe = spell_probes.Probe(directory, 'case.lua', '', {})
    return cb.match_windup(probe)


class Windup(unittest.TestCase):
    def test_canary_template_matches(self):
        effect, delay, _ = match()
        self.assertEqual((effect, delay), ('CONST_ME_GHOST_SMOKE', 2000))

    def test_missing_caster_guard_stays_unresolved(self):
        with self.assertRaises(cb.SpellUnresolved):
            match(guard='')

    def test_changed_cast_body_stays_unresolved(self):
        with self.assertRaises(cb.SpellUnresolved):
            match(cast=CAST + ' combat:execute(creature, var)')


if __name__ == '__main__':
    unittest.main()
