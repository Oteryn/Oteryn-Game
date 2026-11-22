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

The corresponding objects and floors are retained in the shipped Crystal
00ce02a5 map export. The native routes use the western forcefield at
(33552, 31628, 13) and eastern forcefield at (33558, 31628, 13), gated by
their respective access tracks. The surface vortex diverts to the shell cave
until the player retrieves the Sea Ear. One retained shell carries the ear
and five retained statues each contribute a different line. A northwest step
enters the shrine and lands beside the chest. These routes use fixed landings
and objectives. The cave landing uses the swimming outfit qualified from
Crystal's movement script and appearance data.

The northern forcefield at (33555, 31624, 13) opens when the Guardian accepts
the Time Compass and records `access_birthing = 1`. Crystal 201189f8 leaves
the destination unset and supplies no movement action for the forcefield.
The pinned map places the Birthing Grounds return forcefield at
(33426, 31741, 13), linked to (33553, 31624, 13). The entrance lands on the
walkable tile directly south of that return forcefield at (33426, 31742, 13).
[Crystal map](https://github.com/zimbadev/crystalserver/blob/201189f871d23597b82caf88fd611454a2ddefb8/data-global/world/world.otbm)
[TibiaWiki](https://tibia.fandom.com/index.php?curid=69583&oldid=1149608)

The pinned world scan verifies each callback's physical selector and floor,
plus the routes' destination cells. The PostgreSQL chain exercises actual
stepping at the access gates and selected-item USE on the retained objects.
It relocates the test character between distant witnesses.
