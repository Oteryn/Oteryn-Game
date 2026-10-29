# T-SOULCAP — soul maximum and suspended promotion

Official evidence (`CIPSOFT_OFFICIAL`): the owner capture
`/home/mole/wave-a/tibiacom-1/entries/76.txt:14` (2026-10-29), summarized by
`/home/mole/wave-a/tibiacom-1/REPORT.md:925` (F081), assigns 100 soul to
non-promoted characters and 200 to promoted characters. The official manual notes
in `docs/reference/tibia-manual/characters.md:49` agree; line 80 says Premium
expiry suspends all promotion benefits without deleting the permanent promotion.
Thus stored promotion plus current Premium gives 200; either alone gives 100.

Implementation cross-check (`OTS_HYPOTHESIS_ONLY`, not independent official proof):

- Crystal `201189f871d23597b82caf88fd611454a2ddefb8`, inspected locally at
  `/home/mole/oteryn-cache/ref/crystal-201189f`: `data/XML/vocations.xml:23,75`
  assigns sorcerer 100 and master sorcerer 200. Its base monk entry at line 139
  has 200, conflicting with the official general rule; the official rule wins.
- Crystal `data/scripts/creaturescripts/player/login.lua:44-54` restores a stored
  promotion on Premium login and demotes a promoted vocation on Free login.
  `src/creatures/players/player.cpp:125-135` changes vocation/regeneration without
  changing current soul; `src/io/functions/iologindata_load_player.cpp:181`
  loads stored soul directly. Therefore dropping the maximum must not itself
  clamp current soul. A later positive gain clamps to the vocation maximum
  (`src/creatures/players/player.cpp:6692-6703`).
- Canary snapshot `47dfd51f45280a59a1d3e50ba7edd573d7234446`, locally at
  `/home/mole/oteryn-cache/ref/canary-47dfd51f`: `data/XML/vocations.xml:15,63`
  has the same sorcerer maxima (monk/exalted monk: lines 123,135).
  `data/scripts/creaturescripts/player/login.lua:24-34` suspends/restores promotion;
  `src/io/functions/iologindata_load_player.cpp:152` loads soul directly and
  `src/creatures/players/player.cpp:6773-6781` caps a positive gain.

Promotion remains durable. Premium refresh must use the fenced promotion read,
not infer promotion from Premium status or from the previous soul maximum.
