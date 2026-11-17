# CLHUD C18a — native directional spell regression

Base main: `9668e0a04`. No migration, command, capability or registry numbers allocated.

Donor: Crystal `201189f871d23597b82caf88fd611454a2ddefb8`,
`src/creatures/combat/spells.cpp:1238-1261`: ordinary spells use the selected
opponent or the actor's direction. The existing native contract sends
`AttackTarget/true` when a session target exists and `None/false` otherwise;
the server owns direction, eligibility and execution.

The added application tests invoke the native action bar's Apply and Cast
callbacks, admit a real Session over framed duplex IO, run the production
`run_combat`, decode the ordinary spell command, and reduce the server refusal
through Outbox into the native HUD. Both target states verify bound cast index
71 rather than the displayed slot index 1. This is Energy Wave (`exevo vis hur`),
`candidate:spell/energy_wave` in the SPELL-D1 key-sorted production source book.
The test loads the pinned `content/test-packs/spells/r25/catalog.json`, sorts its
source identities as `SpellBook::new_with_alias_selections` does, and asserts
index 71 maps to Energy Wave with `needs_direction=true` and `needs_target=false`.
Its catalogue provenance maps `needDirection(true)` to Crystal
`ff7ede593c69d4c658b382c97443e8155926924a`
`data/scripts/spells/attack/energy_wave.lua:87` and Canary
`99902524e052f37574194466c2949c576e4ab269`, the same path at `:48`.
Existing dev-client wire tests remain.

Checks: `clhud_native_bar_aims_at_the_admitted_attack_target` and
`clhud_native_bar_without_opponent_keeps_directional_fallback` in
`apps/client/src/features/local-spell-action-bar/tests/directional.rs`.
Both tests passed in the full native app suite (345 library cases, 17.16 s).
All app/UI targets passed, as did changed-crate check, all-target Clippy with
`-D warnings`, formatting and architecture checks. Full command timings are in
`CLHUD-CONTRACT-NOTES.md`.
