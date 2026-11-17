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

UID2244's ADDITEM participant is selected from the actual composed destination
of the existing item-move command. It admits native inactive ring6299, snapshots
the three tree states, and joins the ordinary fenced drop transaction. Wrong
items and an already-open room retain ordinary drop behavior. Crystal's
`src/game/game.cpp:2410-2420` calls post-add notification after placing the item;
its return value does not veto the physical move.

Migration2702 adds an immutable offering receipt tied to that exact fresh drop,
its live item and Ground preimages, active content/frame binding and reset epoch.
The one-ring debit, Ground removal and existing type-2 item audit commit together.
Every prior item guard arm is retained. The Channel owner publishes only the
present-tree removals and corresponding blue effects after fresh durable success.
Replay neither consumes another ring nor repeats the projection. No item output,
personal progress, or invented tree timer is introduced. UID9234 remains the
shared reset owner. The native6299 admission is reused; this does not publish the
separate active-ring6300 lifecycle.

The offering case uses a literal UID2244 on synthetic source geometry. It does
not infer a production UID position from the nearby Fire Bug destination. This
qualifies M07's shared offering/trial/reset owners, not either complete quest
journey or M01's generic route registration.

Reset validation: the focused PostgreSQL18.6 case passes through real STEP,
including two missing trees, one surviving tree, conditional blue effects,
teleport and replay. Six focused Dreamer Quest tests pass. Quest/dispatch check
and all-target/all-feature Clippy pass. The filtered route-less run has8 passes.
The preceding Carrot head also passed all five existing M07 filtered regressions,
including real PostgreSQL Fire Bug and all three Lion trials. Local node boot is
skipped as instructed; no full-quest qualification is claimed.

Offering validation: the mounted PostgreSQL18.6 case passes through actual item
move ingress, covering wrong input, exactly one ring debit while trees remain,
an ordinary surviving ring after all trees are absent, replay without another
projection, same-transaction move/debit/audit proof, rejection of a historical
drop forged into a fresh offering, and the real UID9234 STEP reset. Durability
and dispatch checks and all-target/all-feature Clippy pass. The filtered
route-less run has9 passes. The regression was first observed failing through
the real ring drop with the trees still present, before the participant existed.
