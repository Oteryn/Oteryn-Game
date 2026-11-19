# No Rest for the Wicked

- Crystal `201189f871d23597b82caf88fd611454a2ddefb8`
  `data-global/scripts/quests/no_rest_for_the_wicked/no_rest_for_the_wicked_rewards.lua`
  maps UIDs 57237–57247 to the eleven base weapons. The retained World
  placements at (34087–34107, 32346, 14) contain eleven chests without UIDs.
  The native binding assigns the Crystal choices in ascending UID order from
  west to east. The last chest follows an extra top-level scenery item.
- Crystal `data-global/npc/the_forgemaster.lua` grants access when the
  Forgemaster door stage is 2, writes the access flag and advances the stage
  to 3. The Fandom quest spoiler archive confirms the ten hard-win threshold.
- Crystal `data-global/scripts/quests/no_rest_for_the_wicked/arbaziloth_lever.lua`
  supplies the staging cells, room bounds and starting monsters. Its retained
  World stack at (34058, 32394, 14) puts appearance 8911 at top-level ordinal 2.
  The shared Crystal BossLever does not transform the clicked object.
- Crystal `data-global/monster/quests/no_rest_for_the_wicked/the_forgemaster.lua`
  and `enraged_arbaziloth.lua` supply the phase creatures. The Fandom archive
  confirms 10,500 and 360,000 HP, respectively. The archive quest spoiler
  confirms the purple-fire phase and the spare-Forgemaster hard-mode route.
  The exact purple-fire object is 49362 in Crystal `arbaziloth_fight.lua`
  and `appearances.dat`. The 15.30 appearance membership also contains it.
