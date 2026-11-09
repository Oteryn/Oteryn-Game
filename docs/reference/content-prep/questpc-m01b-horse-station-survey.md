# Horse Station: first pasture survey

Reference donor: [Crystal 201189f8 startup](https://raw.githubusercontent.com/zimbadev/crystalserver/201189f871d23597b82caf88fd611454a2ddefb8/data-global/scripts/globalevents/spawn/horse_station_world_change.lua) and [stable handler](https://raw.githubusercontent.com/zimbadev/crystalserver/201189f871d23597b82caf88fd611454a2ddefb8/data-global/scripts/globalevents/others/horse_station.lua), checked 2026-11-09.

Both scripts identify the Palomino paddock as `(32438..32442,32231..32238,z7)`. Startup horse spawning is suppressed while raid storage 120226 is active. The paddock location stays fixed; the stable movement handler acts on monsters.

The chosen first objective is exactly “Visit the horse station east of Thais and inspect its pasture.” It does not require escaped horses or a running raid. The first-survey resolver therefore uses a permanent landmark, guarded by the authenticated native World/STEP occurrence, independently of those later event mechanics. Only this quest's exact stage-1 objective is eligible for that rule. The full World-change owner stays HELD.

The selected survey point `(32442,32229,-7)` retains ground 4521, two tiles north of the paddock's northeast corner. Its northern starting tile `(32442,32228,-7)` retains ground 4515. Both were verified in `content/world/placements/region-z07-x126-y125.b3`; fences, stable triggers and source objects remain intact.

This resolver is inactive pending native dispatch/PostgreSQL proof. It does not activate horse escapes, raids, taming or rewards.
