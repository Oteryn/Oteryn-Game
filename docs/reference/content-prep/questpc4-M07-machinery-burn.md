# M07 Machinery: native ballista burn sites

This implements the selected Machinery of War programme's eight-ballista stage,
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
Repair sites, catapults, tower wheels, commission/report and finale wiring remain
outside this ballista slice. No completion or achievement is granted here.
