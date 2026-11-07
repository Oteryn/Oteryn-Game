# Selected Paradox journey

Scope is the selected recipe for `oteryn:quest.the_paradox_tower_quest` in
`content/quests/definitions/quests-00200-00299.json`, classified
`CHOSEN_OTERYN_RECIPE_ONLY` / `OTERYN_AUTHORED_APPROXIMATION`.

The recipe requires level30 and premium, the ordered Oldrak/Zoltan/Padreia/Lubo
story, the prisoner's mathemagics rule, one complete selected tower climb and
the fixed Riddler answers. Selected reward intents are32 talons and Mathemagician.
The first Oldrak binding is an engine consumer; it alone does not qualify this
whole quest. Keep failure teleports and other reward alternatives omitted as
specified by the selected recipe. Preserve all original source claims/holds.

Minimum-level authority is reused from PC3 commit
`add7faf928c344a1d90309e1777dd43ee9eecb85`, which reads locked progression after
historical receipt lookup. Reuse PC3's `SetWithBinding` encoding from
`f673adfebd7018b6c3e82026af3f5ed23e849fa4`, applied only to newly gated callbacks
so published ungated definitions retain their hashes. Bound effects must match
the registered callback's exact admission terms; renamed/stripped bindings fail
before writes.

Pinned repository donor evidence: Crystal
`9f5a72c64b87b222a0c8f7c130dadf8e2f125c6d`, existing donor-source corpus manifest.
`data-global/world/world-npc.xml` SHA256
`3cc9a769cda450a0e5e0c60bcc65973031efe8b94e5ea6a4256473d3e45fd306`:
Riddler32479,31901,z2(line55), Zoltan33266,31847,z4(line151),
Oldrak32817,32261,z6(line805), Padreia32302,31814,z7(line1525),
Lubo32489,32119,z7(line1678), A Prisoner32394,32138,z13(line2875).
These centers agree with existing promotion candidates; no invented placement
is necessary. Source scope promotion and the complete route remain in progress.

The tower mechanism work retains the following source puzzle rules. Coordinates
are legacy z in this table; runtime callback content uses negative floors.

| Puzzle | Source rule | Source script |
| --- | --- | --- |
| First floor | Six outside grass tiles3696 must all be grown; transform stone1791 at32478,31902,z7 to stair1947 | movement_first_floor_stair.lua:1-42 |
| Second floor | Six lever states at x32476..32481,y31900,z6 must be2773,2773,2772,2772,2773,2772; main lever controls ladder32476,31904,z6 | action_second_floor_lever.lua:1-42 |
| Third floor | A monster presses the source plate; transform431 to430 and create ladder32478,31904,z5; release removes only its ladder | movement_third_floor_ghoul.lua:1-37 |
| Fourth floor | Six exact foods3593,3587,3590,3585,3592,3589 at x32476..32481,y31900,z4 are consumed when opening ladder32476,31904,z4 | action_four_floor_lever.lua:1-49 |
| Fifth floor | Foods3537 and3543 at32478/32479,31903,z3 open ladder32479,31904,z3 | action_fifth_floor_lever.lua:1-41 |

All paths are under `data-global/scripts/quests/the_paradox_tower/` in the
pinned9f5a72c source corpus. Their SHA256, in table order:

- 09f59bfa0327d9df550a1fe2b9f09b5d2be1e4204ac4f6292a6a4c527f9d4829
- 7bbde6f5e8f8d3028aae11f81f2509b053a51c0cf31660f193896ebdcad8a16c
- e9cc04623815e716b9158617a7bdb5c600d85fe264b873318a9047f3d80a04c5
- b9fe9cb411f48e287267131147452b31589262b4ecc2c7bee264e0a25dd98d56
- 37fe306fec50a7f3638b131f1d0367268c14af4a55073a7017ba49bf548479af

The retained B3 tower regions do not carry the source UIDs. Bindings must use
actual native positions/appearances and document any conservative choice of
plate position, rather than adding invented historical UID attributes. The
shared mechanism PG fixtures qualify owner behavior separately from the native
multifloor route; they are not evidence of full tower traversal.

The ordered dialogue source is pinned Crystal201189f871d23597b82caf88fd611454a2ddefb8.
Public source captures (not copied into this repository):

- [oldrak.lua](https://raw.githubusercontent.com/zimbadev/crystalserver/201189f871d23597b82caf88fd611454a2ddefb8/data-global/npc/oldrak.lua):159-170, SHA256 `dc124275de082d9f39bc2f802b31f823646548153fe0e248417dc54f7b0eba40`.
- [zoltan.lua](https://raw.githubusercontent.com/zimbadev/crystalserver/201189f871d23597b82caf88fd611454a2ddefb8/data-global/npc/zoltan.lua):61-70, SHA256 `c184ead30d167e379ccd628713028fd9cc8acc24713bb7c6943deefce14032d1`.
- [padreia.lua](https://raw.githubusercontent.com/zimbadev/crystalserver/201189f871d23597b82caf88fd611454a2ddefb8/data-global/npc/padreia.lua):112-121, SHA256 `1b521a3ff677c3c4db585886f43d233f999a945dcd0b6a11fb0fd1c3ce19f63a`.
- [lubo.lua](https://raw.githubusercontent.com/zimbadev/crystalserver/201189f871d23597b82caf88fd611454a2ddefb8/data-global/npc/lubo.lua):126-142, SHA256 `a20ddf0044230dafc4c7c372caac1ec252b594cc0ff791fe5f8f622a1410e73d`.
- [a_prisoner.lua](https://raw.githubusercontent.com/zimbadev/crystalserver/201189f871d23597b82caf88fd611454a2ddefb8/data-global/npc/a_prisoner.lua):105-132, SHA256 `a3b582c47aabc693835fe18959ba03e58100602779df215315627e613502e9e4`.

The prisoner awards Mathemagician upon learning the rule in the source. The
Riddler's final question is1 plus1, and the source accepts1 only after that
lesson. This is not an arithmetic generator: the selected recipe uses the
fixed source sequence and learned-rule prerequisite. See pinned
`data-global/npc/riddler.lua:118-144`, SHA256
198907507cb4bf3c40b7c00abdf75a5ebb0e260e6e1b18683a39b652b711dc8a,
<https://raw.githubusercontent.com/zimbadev/crystalserver/201189f871d23597b82caf88fd611454a2ddefb8/data-global/npc/riddler.lua>.
