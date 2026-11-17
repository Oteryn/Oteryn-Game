# Newhaven / Viola / Corruptor evidence — 2026-11-06

**Resolution:** Use current TibiaWiki item identities and quest rewards, the pinned
15.30 client file for appearance IDs, and pinned Crystal for the dialogue/state
flow and intended one-shot vocation branch. Keep those numeric namespaces separate.
Wiki citations below are page ID/revision from the normal MediaWiki API; donor paths
are from `zimbadev/crystalserver` branch `main` at
`201189f871d23597b82caf88fd611454a2ddefb8`. Evidence classes: Fandom facts are
`TIBIAWIKI_STRUCTURED`; Crystal mechanics are `CRYSTAL_OTS` / `OTS_HYPOTHESIS_ONLY`;
the owner-pinned 15.30 client file supplies rendering-identity evidence only.
Server Item IDs and client appearance IDs are separate namespaces
(`docs/agents/programs/OTERYN_REFERENCE_INVESTIGATION_SOURCE_REGISTRY_20260910.md:389-400`).

## Full Viola path and arming

1. Newhaven's quest page says the Newhaven Quest is assigned after arrival following
   vocation selection. Its Lost Wisdom step is Viola `mission` then `yes`, retrieving
   her book from the northeast mines and returning it for a Scarf plus 800 XP
   (Newhaven Quest/Spoiler p111294/r1205117; Newhaven Quest p111293/r1205104).
2. Crystal's vocation-selection step teleports the player, grants starter equipment,
   marks the corrupted mines, and sets `newhavenCitizen`, `newhavenTutorialHunting`,
   and `newhavenNewLootTheCorruptor` to 1
   (`data-global/scripts/quests/newhaven/movement_select_vocation.lua:21-47`). The
   starter kit is a separate grant; its Monk weapon is Light Jo Staff 50166 in the
   left hand, not Simple Jo Staff (`data-global/lib/others/newhaven.lua:78-94,97-144`).
3. The current Book of Wisdom page gives Wiki itemid 51977 and says it is found in a
   chest at the mapped location, gatherable once every five minutes (p109842/r1143205).
   Crystal's Viola script instead checks and removes 44774
   (`data-global/npc/viola.lua:57-58,144-155`). The current 15.30 client appearance
   object 51977 is named `book of wisdom`, is takeable, and has sprite 201015
   (`appearances-2dfa943b548472a1ddc7bc5afe97945bc75e14f1f41d74f728f8e622f5dae7e2.dat#object=51977`, decoded by `tools/content-schema/item-authoring/engine_items.py:531-540`).
   Select 51977 for the current book identity; retain Crystal's dialogue/state flow.
4. On return, Crystal consumes the configured book ID, sets the done storage, and
   gives a vocation item; it does not award the Wiki Scarf or 800 XP
   (`data-global/npc/viola.lua:94-123,144-155`). Current Wiki says the Scarf is item
   3572, neck slot, armor 1, weight 2.00 (Scarf p2473/r1194228), and the quest gives
   it with 800 XP (p111294/r1205117). Select that current Wiki reward. The Wiki Viola
   page says she retired and stopped teaching spells in update 15.22; Crystal's NPC
   script still calls her a spell trainer and configures spells, so use current Wiki
   status for spell teaching and Crystal for the Lost Wisdom state transitions
   (Viola p109305/r1204048; `data-global/npc/viola.lua:60-62,84-92,134-173`).

## Corruptor vocation branch and callback

The current Corruptor Wiki page places it in the Corrupted Mines, describes loot as
depending on the vocation of the player who dealt the most damage, and lists the
named gear below among its drops (The Corruptor p109336/r1200696). Crystal's intended
branch uses the corpse owner as selector, adds vocation gear to the corpse, then
clears `newhavenNewLootTheCorruptor`
(`data-global/scripts/quests/newhaven/#ondroploot_the_corruptor.lua:12-15,35-79`).
The current Wiki item IDs and properties below supersede stale donor reward numbers;
the Crystal grouping is retained as the behavior mapping. **Reconciliation choice:**
read the armed, consumed flag as a one-time special branch alongside the Wiki's
ordinary loot list. The Wiki lists its ordinary rows as Gold Coin always; Health
Potion and Mana Potion semi-rare; White Mushroom and Bone Sword rare. Vocation gear
is listed separately as very rare. The Crystal candidate branch sets each vocation
reward's chance to 100% before adding it (`The Corruptor` p109336/r1200696;
`#ondroploot_the_corruptor.lua:41-66,69-76`). This is an inference about intended
behavior, not proof that the disabled callback runs in Crystal.

| Vocation | Current Wiki named gear and relevant fields |
|---|---|
| Knight | Brass Armor 3359: armor 8, weight 80.00; Wiki leaves level/vocation blank (p1615/r1186985). Longsword 3285: attack 17, defense 14, level 0, one hand, weight 42.00; no range/vocation field (p2404/r1186983). Soldier Helmet 3375: armor 5, weight 32.00; Wiki leaves level/vocation blank (p2304/r1147311). Crystal branch: `#ondroploot_the_corruptor.lua:41-46`. |
| Paladin | Ranger's Cloak 3571: armor 7, Paladins, weight 48.00; level blank (p25353/r1186981). Refined Bow 51760: range 6, level 0, two hands, hit modifier +1, weight 32.50; no attack/defense/vocation field (p109332/r1148726). Crystal branch: `#ondroploot_the_corruptor.lua:52-56`. |
| Monk | Harmony Amulet 50195: armor 2, Monk, level 0, fist fighting +1, weight 5.10 (p108150/r1194222). Simple Jo Staff 51166: attack 12, defense 8, Monk, level 0, two hands, fist fighting +1, weight 18.00 (p108393/r1186982). Crystal branch: `#ondroploot_the_corruptor.lua:47-51`. |
| Druid | Snakebite Rod 3066: Druids, level 6, range 3, earth damage 8–18, weight 19.00 (p4937/r1147165). Magician's Robe 7991: armor 6, Sorcerers and Druids, weight 25.00; level blank (p25343/r1186986). Crystal branch: `#ondroploot_the_corruptor.lua:57-61`. |
| Sorcerer | Wand of Vortex 3074: Sorcerers, level 6, range 3, energy damage 13 (8–18), weight 19.00 (p4939/r1186984). Magician's Robe 7991 as above (p25343/r1186986). Crystal branch: `#ondroploot_the_corruptor.lua:62-66`. |

The Crystal candidate callback file is **disabled**, not an active callback: its filename starts
with `#`, and the pinned loader skips every `.lua` file whose basename starts with
that character before execution (`src/lua/scripts/scripts.cpp:123-137`; callback
registration is at `#ondroploot_the_corruptor.lua:1-3,82`). Its `corpse:addItem`
calls place rewards in the corpse; they do not equip the player
(`#ondroploot_the_corruptor.lua:69-76`). The current Wiki still supplies the Corruptor
vocation rule and named loot; the Crystal file supplies only an inactive branch
pattern. The pinned monster's `monster.loot` rows are the actual ordinary-loot source
(`data-global/monster/newhaven_update_2025/bosses/the_corruptor.lua:66-78`).

## Ordinary loot identities and corpse chain

The current Wiki's ordinary Corruptor rows and item IDs are Gold Coin 3031 (always;
p2119/r1175624), Health Potion 266 (semi-rare; p20308/r1183252), Mana Potion 268
(semi-rare; p20228/r1183192), White Mushroom 3723 (rare; p1335/r1113687), and Bone
Sword 3338 (rare; p2400/r1147274). Its separate very-rare vocation list includes
Brass Armor 3359 (p1615/r1186985), Soldier Helmet 3375 (p2304/r1147311), Wand of
Vortex 3074 (p4939/r1186984), Magician's Robe 7991 (p25343/r1186986), and Refined
Bow 51760 (p109332/r1148726), among the per-vocation gear above (`The Corruptor`
p109336/r1200696). Select the current Wiki names and IDs for item identity; preserve
the donor's literal chance/count fields as source behavior, without equating those
numeric chances with Wiki rarity labels.

The exact donor rows are `{id=3031,chance=100000,minCount=13,maxCount=46}`;
`{id=7618,chance=23810,minCount=1,maxCount=3}`;
`{id=7620,chance=23810,minCount=1,maxCount=3}`; `{id=3338,chance=14290}`;
`{id=3359,chance=4760}`; `{id=8820,chance=4760}`; `{id=3375,chance=4760}`;
`{id=3074,chance=4760}`; `{id=3723,chance=4760,maxCount=2}`; and
`{id=44773,chance=4760}` (`data-global/monster/newhaven_update_2025/bosses/the_corruptor.lua:68-77`).
Only rows 68-70 and 76 contain explicit count fields in this source. Donor comments
identify 7618/7620/8820/44773 as Health Potion/Mana Potion/Magician's Robe/Refined
Bow, so resolve those comments to current Wiki IDs 266/268/7991/51760. The pinned
donor `data/items/items.xml` has no rows for 7618, 7620, or 44773; those are
comment-only identities in this pin. It defines 8820 as Morik's helmet
(`data/items/items.xml:23502-05`), and the current 15.30 appearance object 8820 is
also named `Morik's helmet`, while 7991 is named `magician's robe` and 7992 is named
`mage hat` (`appearances-2dfa943b548472a1ddc7bc5afe97945bc75e14f1f41d74f728f8e622f5dae7e2.dat#object=8820`, `#object=7991`, `#object=7992`). Thus the 8820 comment resolves to Wiki robe 7991, not Mage Hat 7992; donor ID 3338 is Bone Sword and 3723 is White Mushroom, not Bone Club or Salmon (Wiki p2400/r1147274, p1335/r1113687; current 15.30 objects `#object=3338`, `#object=3723`).

The pinned monster definition sets corpse ID 52113
(`data-global/monster/newhaven_update_2025/bosses/the_corruptor.lua:16-20`). The
donor XML chain is 52113 → 52114 (600 s, container size 48), 52114 → 52115 (600 s,
size 48), 52115 → 52116 (10 s, size 48), and 52116 → 0 (600 s, no container-size
attribute) (`data/items/items.xml:87366-84`). In the pinned 15.30 client appearance
file, object 52113 has sprite 277071 and flags `container`, `usable`, `forceuse`,
`unmove`, and `corpse`; 52114 has sprite 277071 and flags `container`, `usable`,
`corpse`; 52115 has sprite 277072 and those same three flags; 52116 has sprite
202575 and `corpse`. These appearance records have no name (`appearances-2dfa943b548472a1ddc7bc5afe97945bc75e14f1f41d74f728f8e622f5dae7e2.dat#object=52113` through `#object=52116`; digest/version and decoder: `tools/content-schema/item-authoring/client_appearance_census.py:45-55,78-88`, `tools/content-schema/item-authoring/engine_items.py:531-540`). The donor capacity is 48 on its first three stages; 15.30 corpse/container flags do not establish a native Oteryn Item profile or replace native admission capacity.

## ID and appearance reconciliation

- Current Wiki Simple Jo Staff is itemid 51166, a Monk-only, both-hands fist weapon
  (attack 12, defense 8, fist fighting +1, weight 18.00; p108393/r1186982). The
  owner-pinned Tibia 15.30 appearance file has object 51119 named `simple jo staff`,
  takeable/clothing-slot 0, sprite 239746; object 51166 is unnamed, immovable, sprite
  239765 (`appearances-2dfa943b548472a1ddc7bc5afe97945bc75e14f1f41d74f728f8e622f5dae7e2.dat#object=51119` and `#object=51166`; file digest/version is pinned by
  `tools/content-schema/item-authoring/client_appearance_census.py:45-55,78-88`,
  decoded by `engine_items.py:531-540`). Thus current Wiki itemid 51166 is not the
  current client appearance ID; bind the 15.30 presentation to appearance 51119.
- Crystal's server item 51119 is named Simple Jo Staff and specifies fist weapon,
  two-handed, attack 12/defense 8, Monk proficiency
  (`data/items/items.xml:86054-67`); its pinned `data/items/appearances.dat#object=51119`
  is also named `simple jo staff` (sprite 238626). This donor server ID happens to
  equal the donor and current client appearance ID; preserve the namespace distinction.
- Crystal's candidate callback instead calls Harmony Amulet 48847 and Simple Jo Staff
  48845, and Paladin Ranger's Cloak 10321 / Refined Bow 48843
  (`#ondroploot_the_corruptor.lua:47-56`). Crystal `items.xml` names 10321 Mammoth
  Tusk (`data/items/items.xml:26975`), while current Wiki names/IDs are Harmony 50195,
  Simple Jo Staff 51166, Ranger's Cloak 3571, and Refined Bow 51760 (pages/revisions
  in the table above). Current Wiki names and IDs win; do not propagate those stale
  callback numbers.
- Current Wiki Book of Wisdom itemid 51977 (p109842/r1143205) is also the named 15.30
  client appearance object 51977. Crystal's quest script's literal 44774 remains a
  donor-side identifier; the current route uses 51977 and the Wiki Scarf/XP reward.

## Local world projection

The current world projection places The Corruptor at (32614,32422,7), respawn 60 s
(`content/world/spawns/spawns-12000-13999.json#oteryn:spawn.x32611_y32427_z7`). The
current NPC-placement projection has no Viola record
(`content/world/npc-placements/placements-00000-00032.json#/records`; exact name
scan returns no rows). These are projection facts, not a reason to discard the
selected Wiki-plus-donor source resolution.

## Run8 API refresh (2026-11-06)
The owner-provided honest-UA api.php helper now succeeds. Historical403/unavailable statements above describe prior attempts and are superseded for captured pages by `.questpc/wiki-r8/` (raw page text, revision metadata, SHA256 captures). Current inventory refreshed788 infobox pages; all373 pinned titles occur. Infobox presence is not a quest/access completion claim. Current Wiki Newhaven/Viola still selects51977→3572+800XP; Corruptor ordinary names resolve stale donor IDs as explicitly documented. Charge item pages establish item defaults, not charge quantities of specific chest grants; explicit donor reward count remains selected. SoulWar page/spoiler refreshed, implementation belongs questpc3 R..S in run8.
