# M03: selected anniversary entrance and source return

The canonical `25_years_of_tibia_quest` chosen recipe s1 selects level20,
Premium, all five regular blessings and promotion as entrance requirements.
This is the owner's chosen approximation: the historical source checks
blessings/promotion later in the tour. Original source holds remain unchanged.
Neither native entrance binding completes the quest or certifies later stages.

The native USE binding selects the committed machine part25098 at
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
