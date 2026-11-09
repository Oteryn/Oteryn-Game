# M07 Dreamer's Challenge: Death Ring room

Reference: Crystal `201189f871d23597b82caf88fd611454a2ddefb8`,
`data-global/scripts/quests/dreamers_challenge_quest/movement_death_ring_teleport.lua:1-28`.
The registered UID9234 STEP teleports to `(32819,32347,9)`, restores missing
trees3636 at `(32857,32231..32233,11)`, and emits blue magic13 only at those
restored cells. Teleport uses source effect11. The shared world owner restores
its removed base entries without creating portable tree items or personal flags.
The source creates trees even beneath actors; this exact reset bypasses the
generic door occupancy refusal. All other callbacks retain that check.

The callback's UID, destination and three tree bindings are closed together.
The mounted case uses UID9234 on synthetic source geometry, with two missing
trees and one surviving tree as its precondition. It qualifies the actual STEP
and world projection owners; it does not claim a recovered production UID position.
Generic route authoring remains M01's boundary.

M07 remains PARTIAL until the UID2244 ADDITEM offering joins the actual item-drop
caller. A surrogate USE or a craft output would not implement the donor's action.

Reset validation: the focused PostgreSQL18.6 case passes through real STEP,
including two missing trees, one surviving tree, conditional blue effects,
teleport and replay. Six focused Dreamer Quest tests pass. Quest/dispatch check
and all-target/all-feature Clippy pass. The filtered route-less run has8 passes.
The preceding Carrot head also passed all five existing M07 filtered regressions,
including real PostgreSQL Fire Bug and all three Lion trials. Local node boot is
skipped as instructed; no full-quest qualification is claimed.
