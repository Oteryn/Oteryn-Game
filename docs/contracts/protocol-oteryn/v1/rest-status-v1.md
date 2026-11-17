# Rest status V1 — GAP-SV7 C10

Rebased onto `c97858c3f440fd26676d3bc9be6d6a35b2c711ff`. Existing native
routes and capabilities 286/287 are retained; the five new shortcuts use
Ctrl+Alt+Shift plus C/R/B/K/P respectively to avoid the base bindings.

Capability 277, command 493. Read-only, exact native actor/session projection.

Current-main gap (`c8b3890e6`): `crates/protocol-oteryn/src/actor_spell.rs:128-136`
has no resting or food fields, although the authoritative cadence exists at
`crates/gameplay/dispatch/src/gameplay_transport/composed_admission/vitals.rs:137-211`.

## Request

Empty payload; unknown bytes are rejected. Requires selected capability and the
current item fence, connection generation, runtime actor, session and lease.

## Reply

Exactly 11 bytes, little endian: flags u8 (bit 0 inside resting area, bit 1
stamina bonus, bit 2 soul bonus; other bits forbidden), daily streak u32,
food remaining milliseconds i32 (0 absent, -1 infinite; a live finite
condition with less than 1 ms left is represented as 1), health multiplier u8,
mana multiplier u8 (each 0 inactive, 1 normal, 2 double). Since Winter Update 2026
(thread 565) the bonuses no longer depend on the streak: inside a resting area a Free
character has health and mana multiplier 1 and no stamina or soul bonus; a Premium
character has multipliers 2 plus the stamina and soul bonuses. The streak field stays
informational. These describe
resting-area bonus tiers, not total health/mana regeneration: a zero resting
bonus does not imply that the always-on native regeneration clock has stopped. Failure has rejected
command status with empty payload. Client clears the old snapshot on refusal,
malformed reply, session replacement and capability loss.

## Rules and donor parity

Pinned Crystal `201189f871d23597b82caf88fd611454a2ddefb8`,
`src/server/network/protocol/protocolgame.cpp:7801-7845` sends protection and
streak-based health/mana/stamina/soul bonus state; `:7822-7838` names the
normal and doubled regeneration tiers. Crystal
`src/creatures/players/player.cpp:801-810` determines resting status from PZ.
Oteryn retains its existing PZ-qualified resting-area approximation, already
recorded in `dispatch/src/gameplay_transport/composed_admission/vitals.rs:137`;
this package does not broaden or change it.

The existing Oteryn owner unlocks health at streak 2, mana at 3, Premium stamina
at 4, Premium double health at 5, double mana at 6 and soul at 7. It already
retains normal health/mana bonuses on Free accounts, with Premium-only tiers
4–7 (`spell/src/spell/food_regeneration.rs:62-92` and
`dispatch/src/gameplay_transport/resting_area.rs:139-162`). The projection reads
that policy without advancing a clock or granting regeneration. Existing
always-on food-selected regeneration follows the accepted current Tibia
news 8541/8543 difference from Crystal's fed-only clocks; no formula changes.

Food uses the actual `FoodRegeneration` condition and its owner-computed
remaining duration. Crystal `src/creatures/combat/condition.cpp:193-208`
stores a deadline, subtracts condition ticks and retains `-1` for infinity;
`:1330-1357` consumes the regeneration condition on the server. The projection
reads the owner instead of constructing a client timer. Its observation clock
is sampled after acquiring the current actor state; a clock older than the
condition owner is rejected. Millisecond duration is an Oteryn wire extension requested by
GAP-SV7, not a claim that Crystal sends the same duration payload. Dead actors
show no active regeneration bonuses. This panel never changes food or bonuses.
