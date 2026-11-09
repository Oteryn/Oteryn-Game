# M03: selected anniversary entrance and source return

The canonical `25_years_of_tibia_quest` chosen recipe s1 selects level20,
Premium, all five regular blessings and promotion as entrance requirements.
This is the owner's chosen approximation: the historical source checks
blessings/promotion later in the tour. Original source holds remain unchanged.
Neither native entrance binding completes the quest or certifies later stages.

The original typed USE binding selects the committed machine part25098 at
(32430,32166,8), root ordinal1, and travels to (33453,31030,8).
The auxiliary `s1-return` operation selects part25098 at (33453,31029,8)
and returns to (32430,32167,8), preserving the s1 track. It has no Premium,
promotion, blessing or level gate, so expiry cannot prevent leaving.
The entrance raises s1 to1 without lowering it, allowing later re-entry.
Native floors are negative; source coordinates above use positive map floors.

Checked source evidence:

- Crystal blob `fa09caaf3b5227d8571e6d406bf159d881dd67b8`,
  `data-global/scripts/quests/25_years_of _tibia_quest/actions_time_machine.lua`,
  lines1–12: both USE areas and exact destinations; lines25–36: direction dispatch.
  https://api.github.com/repos/zimbadev/crystalserver/git/blobs/fa09caaf3b5227d8571e6d406bf159d881dd67b8
- Crystal commit `201189f871d23597b82caf88fd611454a2ddefb8`,
  `data/items/items.xml:48329–48332`: machine parts25091–25100.
  Body SHA256 `8a4a526c6e45b5a282679334eb1e1e25a5b72af13aa92c8a0d409755ebdbc509`.
  https://raw.githubusercontent.com/zimbadev/crystalserver/201189f871d23597b82caf88fd611454a2ddefb8/data/items/items.xml
- Wiki lines30–38: five common blessings, promotion and the Thais tower entry.
  Body SHA256 `ce33373baa5a450cc64e61784a0b9714d3af74fe2d3a224de42938e0b4315669`.
  https://www.tibiawiki.com.br/index.php?title=25_Years_of_Tibia_Quest&action=raw
- Committed `region-z08-x126-y125.b3`, SHA256
  `6336314451523d64e6bace5ce8f56146592a9c0ff5282a93e6b5cfb6db55fc1d`:
  entrance part25098, ordinal1, bare attributes, outside a house.
  The return part is retained in `region-z08-x130-y121.b3`.

No World placements, Item definitions, commercial facts or migrations are added.

The counterpart expansion covers all nine retained machine parts in each
source rectangle (`actions_time_machine.lua:40-50`, same Crystal blob above).
All 18 counterparts belong to the same immutable authored binding document;
they never register a second global owner for the shared progress counter.
The central part25098 routes remain the authored inputs. The generator adds
eight entrances and eight free returns; return
ordinals come from B3 because several tiles carry another decoration before
the machine part. Each new entrance explicitly declares the same 0..1 s1
counter, and returns preserve it. No entrance bypasses the chosen live guards.

Scripted landing policy is explicit (`effects.scripted_teleport`). The retained
pad25100 is blocked for ordinary movement; changing source destinations or
removing the pad would hide this compatibility boundary. The new sealed
script proof permits a present blocked landing while retaining map/session
fences, PZ restrictions, closed doors and other-actor occupancy. Its default
still delegates to the ordinary strict teleport proof. Initial admission in
the PG scene starts beside the pad, at retained bare floor(32429,32167,8).

Crystal's Lua `creature:teleportTo` delegates to `internalTeleport`:
- [creature_functions.cpp:877–894](https://raw.githubusercontent.com/zimbadev/crystalserver/201189f871d23597b82caf88fd611454a2ddefb8/src/lua/functions/creatures/creature_functions.cpp)
- [game.cpp:3179–3193](https://raw.githubusercontent.com/zimbadev/crystalserver/201189f871d23597b82caf88fd611454a2ddefb8/src/game/game.cpp)
The latter requires a present tile and calls `queryAdd` with `FLAG_NOLIMIT`.
game.cpp body SHA256: `5f7ba67a158af607bacb9d75451ca490817f189ca7d3dcf9e774d55dc1ccaab6`.
creature_functions.cpp body SHA256: `fb8d8a65fa24edb11d4c068047839be29429981da3803f7a197b8ed97d2623ac`.

Admission qualification (2026-11-09):
`m03_every_machine_counterpart_native_use_guards_replay_and_free_return_postgres`
passed on PostgreSQL 18.6 in 113.724511589 seconds. It exercises every entrance
and paired return, including return ordinals 1 and 2, through network USE.
Every entrance checks level, promotion, regular blessings and live Premium;
every return succeeds after promotion, blessings and Premium are removed.
Exact request replays preserve the movement revision and receipt count.
The nine round trips leave one progress track at 1, eighteen receipts and no
quest completion. Actor pre-positioning uses the sealed movement owner over
unchanged B3 tiles. Qualification covers the admission boundary.

The canonical entry/return case passed in 169.998617116 seconds and the Pits
native STEP case passed in 99.909705642 seconds in the same focused run.
