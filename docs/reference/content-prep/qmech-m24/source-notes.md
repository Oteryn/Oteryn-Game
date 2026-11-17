# M24 native death progress sources

This package adds native death consumers, not whole quest completion.

- Original room recipients: retained Crystal revision
  `201189f871d23597b82caf88fd611454a2ddefb8`,
  [HeartBossDeath](https://github.com/zimbadev/crystalserver/blob/201189f871d23597b82caf88fd611454a2ddefb8/data-global/scripts/quests/heart_of_destruction/creaturescripts_heart_boss_death.lua#L26-L138)
  and [Court Warlock](https://github.com/zimbadev/crystalserver/blob/201189f871d23597b82caf88fd611454a2ddefb8/data-global/scripts/quests/the_order_of_the_stag/court_warlock_fight.lua#L323-L329).
  Exact room players are eligible independently of their social party or damage.
- Original damage recipients:
  [Magma Bubble](https://github.com/zimbadev/crystalserver/blob/201189f871d23597b82caf88fd611454a2ddefb8/data-global/scripts/quests/the_primal_ordeal/creaturescripts_magma_bubble_death.lua#L12-L23)
  and [Goblin Leader](https://github.com/zimbadev/crystalserver/blob/201189f871d23597b82caf88fd611454a2ddefb8/data-global/scripts/quests/tower_defence_quest/creaturescripts_goblin_leader_kill.lua#L3-L8).
  Source bodies were read from the repository's retained donor corpus.
- Cake: the accepted chosen recipe is exactly **20 personal Cake Golem kills**
  (`quests-00200-00299.json`, record 68, stage s2). Its references are the pinned
  [base](https://tibia.fandom.com/index.php?curid=50850&oldid=1090661)
  and [spoiler](https://tibia.fandom.com/index.php?curid=50857&oldid=1164627).
  This is explicitly a chosen Oteryn approximation, not the donor's shared
  world total of 200. The existing Cake Golem source profile is retained under
  `prey-content-1/cake-golem/`; no combat statistics are invented here. The
  owner-unavailable bridge 48227 remains unavailable.
- Ancient Tombs: the accepted chosen recipe binds kill stages s2/s3/s5/s6/s8/
  s10/s12 to Omruc, Thalas, Dipthrah, Mahrdis, Vashresamun, Morguthis and Rahemos.
  `Diphtrah` is the recipe's display spelling; `dipthrah` is the canonical
  creature key. References: pinned
  [base](https://tibia.fandom.com/index.php?curid=3792&oldid=756365),
  [spoiler](https://tibia.fandom.com/index.php?curid=5073&oldid=1174796),
  [crosscheck](https://www.tibiawiki.com.br/index.php?curid=4521&oldid=417247).
  This does not replace the existing tomb puzzles or mint helmet pieces.

`qmech_m24_kill_bindings.py` copies exact chosen stage keys, counts, predecessor
conditions and track bounds from the canonical generated completion candidate.
The native producer must supply actual damaging-player credit. Kill counters
use the predecessor stage as their captured mission epoch; other queued kills'
counter increments do not invalidate that epoch. Existing definition-hash
mismatches remain refused rather than silently upgraded.
