# Daily native gate step bindings

Source classification: `OTS_HYPOTHESIS_ONLY`, compatibility following the donor
as the owner requested. Exact scripts/hashes are retained in
`fury_gates-source.txt` and `nightmare_isles-source.txt` in this folder; no wiki
fact is added by this binding.

| Native binding | Source |
| --- | --- |
| Fury entrance AID 9710 requires current Premium, permanent promotion and level at least60; rejection returns to the prior position. Accepted entrance targets33290,31786,z13. | `crystal:201189f871d23597b82caf88fd611454a2ddefb8:data-global/scripts/world_changes/fury_gates.lua:40–60` |
| Fury exit AID 9715 targets the currently selected city's exit, without entrance eligibility checks. Ten exit coordinates are retained verbatim in the source evidence. | Same source:5–15,62–65 |
| Nightmare entrances UIDs 64001, 64002 and 64003 target33497,32616,z8. | `crystal:201189f871d23597b82caf88fd611454a2ddefb8:data-global/scripts/world_changes/nightmare_isles.lua:58–73` |
| Nightmare exit UID 35020 uses the selected daily variant's exit. | Same source:30–56 |
| Nightmare's 16 ladder action IDs target the exact supplied coordinates/floors. | Same source:75–119 |

Implementation decision: dispatch follows the ordinary production STEP using
actual placed UID/AID facts. Entrances and Nightmare actions require their
corresponding selected native geometry. Fury exit AID 9715 instead reads the
independently chosen, pinned-layer-validated city metadata (`exit_layers`), so a
player already inside the dungeon can return on a W10 off-day. That metadata
never activates entrance AID 9710; the native landing proof still refuses a city
target absent from the loaded geometry. This retains the donor's city draw
(`fury_gates.lua:20–29`) alongside the accepted W10 entry chance. A regional
bundle cannot invent an uncompiled choice. Ambiguous placements or multiple
selected variants fail closed. No automatic unconditional teleport attribute is
created for the Fury entrance: doing so would bypass its eligibility rules.

The existing sealed callback teleport proof preserves current actor/session,
position revision, pin/context, walkability, occupancy, closed-door and PZ landing
checks. Durable Premium/promotion reads happen before Channel locks; state and
snapshot qualification is repeated under the movement mutation owner. A failed
landing leaves the player on the entered tile. No migration or protocol ID is
needed.

## Focused verification

`cargo test --locked -p oteryn-gameplay-dispatch --lib
w05b_daily_gate_real_step_uses_selected_native_ids_and_refuses_unqualified_fury
-- --nocapture` first failed on the ordinary production STEP: the admitted
Nightmare entrance left the actor at (101,100,z7) instead of (33497,32616,z8).
The off-day regression separately failed on ordinary STEP at (101,100,z7) instead
of the selected Thais exit (32265,32164,z7), despite valid exit metadata.

After implementation, `cargo test --locked -p oteryn-gameplay-dispatch --lib
 daily_world_gates -- --nocapture` passed 3 tests, 0 failures, 0 ignored in 69.45s.
The configured PostgreSQL route executed 9 fresh authority/session scenes,
each emitting `W05B_DAILY_GATE_POSTGRES_CASE_COMPLETED` after its asserted
native position: active/inactive Nightmare entry, Nightmare exit and ladder,
Fury active exit, refused entry, qualified level 60 entry, off-day exit and
fully qualified off-day entry refusal. The boundary unit also checks level 59,
level 60 and level 61 and the two independent missing entitlement facts.

The shared test artifact cache contains only immutable native content; every
scene creates its own SQL Harness and current Character/Node/session owners.
A WorldId assertion checks the reused artifact against each fresh dependency.
No current authority, Premium, promotion or recovery evidence is cached.
