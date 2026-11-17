# M03 source entrance evidence

- Crystal revision `201189f871d23597b82caf88fd611454a2ddefb8`,
  `data-global/scripts/quests/25_years_of _tibia_quest/actions_time_machine.lua:1-11,26-49`:
  https://github.com/zimbadev/crystalserver/blob/201189f871d23597b82caf88fd611454a2ddefb8/data-global/scripts/quests/25_years_of%20_tibia_quest/actions_time_machine.lua
  gives the two three-by-three machine areas and both destinations. The retained
  corpus provides the same file. Every machine part is bound to its actual B3
  stack ordinal; some return parts have an extra preceding decoration.
- Pits of Inferno entrance: https://www.tibiawiki.com.br/index.php?title=Pits_of_Inferno_Quest&oldid=429025
  explicitly identifies `32795,32335,8` as the entrance through which the player
  reaches Goshnar's tomb. The committed B3 has one bare ground354 at ordinal0.
  This is the selected first-stage exploration anchor, not an inferred Area box.
- Requirements are the accepted chosen recipes, not assertions that the Lua
  donor checks them: `content/quests/definitions/quests-00200-00299.json:1`
  record67 and `quests-00000-00099.json:1` record89. Both are Premium; the first
  requires level20, promotion and all regular blessings; PoI requires level80.
  `rulesets/character/blessings/catalogue.json:1` defines exactly five regular
  blessings. Enhanced blessings and Twist of Fate are excluded.
- Rope/Shovel are route tools in PoI's chosen requirements. This hook does not
  manufacture tools or assert inventory ownership after reaching the anchor.
  It supplies the missing live Premium predicate and the existing level check.
- Machine return uses the donor's unconditional exit. Losing blessings or
  Premium after entering must not prevent returning. Re-entry raises the same
  bounded first-stage counter without resetting later quest progress.
