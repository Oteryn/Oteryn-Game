# M34 reward placement and allocation sources

Scope: the four selected Oteryn authored recipes. Historical/source reset rules
remain evidence; the accepted recipes deliberately select personal once-only
claims. No current Global playability is inferred from an OTS implementation.

## Captured public sources

- `OTHER_STRUCTURED_TIBIA_DATA`: public retained wiki walkthroughs at
  <https://github.com/Lavovskii/grimorio/tree/dbeee0138a54b30a74dd690c4a7e6fab514c8db0/_questski/tools/briefs>:
  `kingdom-of-kormarak-quest.md`, `mastermind-potion-quest.md`, and
  `mystery-of-the-valley-quest.md`. They retain original TibiaWiki page URLs;
  their coordinates/reward descriptions are cross-checked against the selected
  recipes and native map entries before admission.
- `OTS_HYPOTHESIS_ONLY`: treasure markers in
  <https://github.com/Brunowots/RealMap-Global-8.0-With-Mount-Market-TFS-1.2/blob/e4113537e25870336811c4131e379c2710348ae6/data/lib/QuestPoints/Marks.lua>:
  Kormarak treasure `(32552,32200,11)`; Mastermind treasure
  `(32216,31168,11)`. This old map is a location lead, not a 15.30 rules owner.
- `OTHER_STRUCTURED_TIBIA_DATA`: archival Magic Sword restoration evidence at
  <https://github.com/Ethaan/mintwall/blob/263b5b0b01b689861ebe3c13a605e6badd1ed3ee/docs/reference-74/quests.md#excluded--post-74>:
  far southern Mintwallin `(32411,32247,15)`, northern allocation Yellow Spell
  Wand + Demon Armor + Demon Legs, southern allocation Magic Sword + Demon Armor
  + Demon Legs. The selected Oteryn recipe already explicitly restores this
  deprecated quest and replaces shared reset loot with personal claims.
- `OTS_HYPOTHESIS_ONLY`: old item name witness
  <https://github.com/hampusborgos/rme/blob/da7152ec94031c76732e997de4624c8f1c010225/data/740/items.xml#L3653>:
  historical server item2189 is named `yellow spell wand`. Historical server
  numbers are not canonical Tibia Item IDs; identity needs a separate join.
- `OTHER_STRUCTURED_TIBIA_DATA`: item-name/ID cross-check
  <https://github.com/Brenndy/Tibia-market-mobile/blob/e7805868a3f4b6c8ae35b34e3e4dd1aa4636e788/src/data/itemList.ts#L2627>:
  fragment1/2/3 are Tibia IDs50336/50337/50338; Guardian Statue (Quest) is50335.
  The repository's admitted 15.30 appearance manifest and canonical definitions
  independently contain those four exact IDs.

## Existing accepted source facts

`imports/tibiawiki/facts/items-stats.json` has target-compatible pinned weights:

| Item | page / revision | weight | source |
|---|---|---|---|
| Guardian Statue (Quest),50335 | 107865 / 1143035 | 9.00 oz | <https://tibia.fandom.com/wiki/Guardian_Statue_(Quest)> |
| Fragment1,50336 | 107860 / 1143038 | 3.00 oz | <https://tibia.fandom.com/wiki/Fragment_of_a_Statue_(1st)> |
| Fragment2,50337 | 107862 / 1143037 | 3.00 oz | <https://tibia.fandom.com/wiki/Fragment_of_a_Statue_(2nd)> |
| Fragment3,50338 | 107864 / 1143036 | 3.00 oz | <https://tibia.fandom.com/wiki/Fragment_of_a_Statue_(3rd)> |

Owner decision D165 in
`docs/architecture/reviews/OTERYN_GAME_OWNER_DECISION_BATCH_D165_D173_2026-09-30.md`
classifies50335 as the quest variant. Ordinary guardian statues8008/8009 are
separate identities. The selected Valley recipe requires final consumption of
all three distinct fragments; three copies of one fragment cannot substitute.

## Native map observations

Read through `world_region_codec.py` over the committed placement shards:
Kormarak's exact treasure tile already contains top-level ordinal1, appearance
4240 (`dead human`), beside wooden coffin appearances2474/2475. The Mastermind
marker tile contains ordinal1, appearance2473 (`box`). No source UID is needed
for either binding. The archival Magic Sword coordinates contain nonwalkable mountain filler
(appearance1128); the selected restoration replaces unowned filler with authored
floors, chests and a paired entrance.
The Valley shrine map markers identify approaches, so each final shrine's floor
and exact object must be selected and checked independently.

## Access observations

Normal HTTP requests to the four TibiaWiki BR quest pages returned403, and the
historical Fandom MediaWiki revision API returned402. No protection bypass was
attempted. The public retained sources above and existing pinned wiki records
provide independent working routes. Full response bodies are scratch evidence,
not redistributed in this repository.


Chosen placement restoration: Magic Sword restores a 4 by 3 dirt-floor room
at x32410–32413/y32245–32247/z15, two chests at32411/32412,32247,
and a paired trapdoor/ladder at32411,32246,z14/z15. These replace unowned
mountain filler; the existing walkable z14 approach is preserved.
The three Valley shrines use existing empty walkable platforms at
32463,31092,5;33187,31191,6;33836,31677,4 (northwest of Tefrit).
The final shrine is33082,32670,3. This is an explicit Oteryn placement choice,
not a claim that archived wiki prose contains these exact modern coordinates.
Yellow Spell Wand uses the explicitly restored successor appearance3073:
https://github.com/hampusborgos/rme/blob/da7152ec94031c76732e997de4624c8f1c010225/data/740/items.xml#L3653
and the same server2189 wand in860 items.xml, with860 OTB client3073.
The three fragments retain50336/50337/50338, and quest statue50335;
weights9/3/3/3oz come from pinned Wiki revisions1143035/1143038/1143037/1143036
in imports/tibiawiki/facts/items-stats.json, independently checked against
admitted15.30 appearance flags. Ordinary Guardian Statue8009 is not substituted.
Valley access gates use Ice Islands mission06=8 (Cure the Dogs) and
New Frontier mission08=2 (Offer You Can't Refuse), source:
https://github.com/zimbadev/crystalserver/blob/201189f871d23597b82caf88fd611454a2ddefb8/data-global/npc/ztiss.lua#L90
https://github.com/zimbadev/crystalserver/blob/201189f871d23597b82caf88fd611454a2ddefb8/data-global/scripts/quests/the_ice_islands_quest/movements_nibelor_dog_led.lua#L2
