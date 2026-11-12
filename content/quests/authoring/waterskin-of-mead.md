# Waterskin of Mead authoring rule

The public walkthrough identifies the Formorgar Mines reward room:
https://www.tibiawiki.com.br/wiki/Waterskin_of_Mead_Quest and its Waterskin_Quest02.png map.
Matching that map to native automap colours locates floor 10, room near (32215,31065).
The immutable placement source `content/world/placements/region-z10-x125-y121.b3`
(SHA256 3eee23ab13970525b08cca863894db89460f451f79ed3a6dc004a49a5529628e)
has barrels at (32214,31060,10) and (32215,31060,10), each appearance2523,
without UID/action attributes. Preserve both source tiles.

Conservative authored assumption: bind only the eastern barrel, once per character.
The walkthrough does not distinguish these two adjacent barrels. This is an explicit
adaptation, not a claim that donor bytes identify the eastern barrel.

Mission acceptance source: Crystal201189f871d23597b82caf88fd611454a2ddefb8
`data-global/npc/lurik.lua:118-120`, SHA256
4c26f87476f1b358e82c06814ad0c6d6f2b1413066dcd198769ca5c6fbd4d274:
Questline35, Mission09=1 and FormorgarMinesDoor=1. Gates preserve later progress;
claiming this side reward must not complete the parent Ice Islands quest.
Modern fluid subtype16 is FLUID_MEAD in the same source revision's
`src/utils/utils_definitions.hpp:456`, SHA256
03ebfc0da66b01cdef46dca4195c4f1a0132a0fce058d6c670cb401bcc35ae7d.
Reward: fur bag7343 containing waterskin2901 filled with mead16,
berserk potion7439 and 82 gold coins3031.

The selected reward/counts and once rule are the existing authored recipe in
`tools/content-schema/quest-authoring/samples/authored68/recipes.json`, canonical
`oteryn:quest.authored.waterskin_of_mead_quest`; they are not inferred from the map.
Premium is a fresh grant requirement checked against the live account owner.
The parent quest's preserved mission gates are external prerequisites, not a
claim that this callback implements the rest of Ice Islands.
Walkthrough map: https://www.tibiawiki.com.br/images/d/d8/Waterskin_Quest02.png
SHA256 8fa81400d2a6ae6c5fc2d89b944d2fec59076421c6db794f1257021b4c46c6d8.
At two pixels per native tile its upper left corresponds to (32118,31017,10);
the room match is unique among the inspected native Formorgar floors.
