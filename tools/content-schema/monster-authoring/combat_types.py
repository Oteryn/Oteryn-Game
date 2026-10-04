"""Reverse source CombatType_t values without choosing C++ iteration aliases.

Crystal 00ce02a57ca5a12e48f32a3476e37471167e4c3f:
src/creatures/creatures_definitions.hpp:818-819 declares FIRST=PHYSICALDAMAGE
and LAST=NEUTRALDAMAGE; src/lua/functions/core/game/lua_enums.cpp:322-335
registers the damage names, not those iteration aliases. A last-name-wins
reverse dictionary therefore invents a non-Lua name for actual physical casts.
Only those exact aliases, with equality checked in the supplied owning enum,
are canonicalized. This does not register aliases in the Lua probe or infer
unknown damage types.
"""

ITERATION_ALIASES = {'COMBAT_FIRST': 'COMBAT_PHYSICALDAMAGE',
                     'COMBAT_LAST': 'COMBAT_NEUTRALDAMAGE'}


def reverse_combat_types(values):
    reverse = {value: name for name, value in values.items()}
    for alias, canonical in ITERATION_ALIASES.items():
        if alias in values and canonical in values and values[alias] == values[canonical]:
            reverse[values[alias]] = canonical
    return reverse
