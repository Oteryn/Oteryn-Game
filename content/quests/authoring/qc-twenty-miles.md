# Twenty Miles Beneath the Sea

The NPC and object callbacks restore the pre-withdrawal quest from
`211d1edc4^`. The retained Crystal 201189f8 tree contains Brandon's and the
Guardian's NPC definitions and the separate Oramond dive event. Its global and
custom NPC spawn files omit all three contacts, and its global map omits these
quest objects. The placement converter uses the captured TibiaWiki positions
for Brandon and the Guardian and the captured TibiaWiki BR position for Yeo.
Yeo uses the existing solid-contact policy because his tile contains broken
glass. The bundle selector test checks all three coordinates.

The west and east entrances, the Sea Ear diversion and the song follow the
walkthrough. [TibiaWiki](https://tibia.fandom.com/index.php?curid=69583&oldid=1149608)

The retained Crystal 00ce02a5 map export supplies the forcefields, shrines,
and whirlpools. The western and eastern portals check their personal access
tracks. The northern forcefield at (33555, 31624, 13) checks `access_birthing`
and lands at (33426, 31740, 13), one step north of the retained return portal.
That return portal at (33426, 31741, 13) targets the northern side of the
Seacrest hub. [Seacrest Grounds](https://tibia.fandom.com/wiki/Seacrest_Grounds#Birthing_Grounds)

The surface vortex uses the character's quest stage and, during stage 19–21,
current Sea Ear custody. The cave stays reachable after dropping the ear.
The route uses the swimming outfit qualified from the pinned Crystal movement
script and appearance data. The walkthrough documents public entry to the
main grounds without quest progress or diving headgear. Underwater movement
retains its normal drowning rules.

The Secret Shrine entrance lands at the south side of the retained whirlpool
maze. Three retained whirlpools lead to the chest, and the exit returns to
the approach. The map export does not carry their original destinations.
Each statue asks the player to return after two minutes before awarding its
line. The three repaired
altars show their restored appearance for one minute.

A bare USE on any paid hoard returns the character to the ring chamber
during stages 4–5. The Guardian's lost-item dialogue recovers an outstanding
quest item after its previous issuance is gone. The writer checks carried
items and surviving issued instances, including those in Ground or another
character's custody.

The pinned world scan verifies physical selectors, destination cells, and the
three permanent contacts. The PostgreSQL fixtures drive STEP, USE, selected
tools, and NPC dialogue through framed commands. Distant witnesses are
reached with fixture relocation.
