# M07 Machinery: native ballista burn sites

This implements the selected Machinery of War programme's eight-ballista and
eight-catapult burn occurrences,
not certification of the whole quest or its original dynamic war schedule.
`tools/content-schema/quest-authoring/samples/completion242/recipes.json`, record109,
selects level50, free-account eligibility and deterministic Fire Bug interactions.

The existing native map contains eight three-part ballistae at the declared
callback coordinates. Each has a front5697, rear5695 one tile north and side5696
one tile west. Evidence is
`content/world/placements/region-z07-x128-y123.b3`, SHA256
`865ee3aa9e0c4d592571b40b72c137a8b05a19f850e0b570974276ebbeb9a22d`.
The authoring test decodes that B3 and checks all24 parts and eight ordinals.
There are no recovered donor action/unique IDs; these are explicit Oteryn native
bindings to the captured objects.

The exact carried tool is5467. Its physical facts come from
`imports/canary/items-xml/items.xml:14936`, revision
`47dfd51f45280a59a1d3e50ba7edd573d7234446`, file SHA256
`1cf2992cdd7cc5b97bcf930b8c89676ec1627170008e995fd2576110e26022f2`.
[Pinned Fire Bug source](https://github.com/zimbadev/crystalserver/blob/201189f871d23597b82caf88fd611454a2ddefb8/data-crystal/scripts/actions/tools/fire_bug.lua)
uses2113 for a lit coal basin; it does not implement these Machinery sites.

Chosen behavior: retain the Fire Bug, require all three original machine parts,
show fire2113 on the front for60seconds, and give one personal credit per distinct
ballista. The other two parts remain intact. The shared visual reset allows
another character's later interaction; it does not clear personal credit.
The60second visual lifecycle and guaranteed success are authored adaptations,
not claims about an original server's timer or random failure behavior.

The native callback model now permits its already transactional level check to
compose with these world predicates only for exact native, irreversible player
USE trials with personal writes. Legacy unbound combinations remain rejected.
Level enforcement remains in
`crates/gameplay/durability/src/durability/quest_world_callback.rs:214`.
Catapults have four parts:5608 front,5606 north,5607 west and5605 northwest.
The same pinned B3 contains exactly the eight declared sites; the separate
catapult authoring test verifies all32 parts and native ordinals. Some fronts
are ordinal2 because a grass decoration precedes the machine in the source
stack. These sites have separate personal flags and a separate eight-site
counter, using the same chosen retained-tool/fire/reset behavior.

The PG driver resolves appearance facts through the bundle palette and uses
the actual callback selector and declared rear part. Both machine families run
the same complete assertions with their own captured native crops.
Six two-part wheels use7775 on the left and7776 on the right, matching the
named wheel entries in `imports/canary/items-xml/items.xml:21008`.
Their captured left positions are32964,31814;32961,31817;32964,31817;
32964,31820;32961,31823;32964,31823, all on source floor7. The two groups
of three match the selected two-tower/six-wheel objective. Evidence is
`content/world/placements/region-z07-x128-y124.b3`, SHA256
`bdcf1412c9d0ea16670cd16ed415740da23293bc12f30cfc16cbe1bd04194f66`.
The geometry test verifies both parts and exact left ordinals, including the
grass-decorated ordinal2 cases. A separate six-site personal counter prevents
reusing one restored wheel to satisfy the other five.

Repair sites, commission/report and finale wiring remain outside these burn
slices. No completion or achievement is granted here.
