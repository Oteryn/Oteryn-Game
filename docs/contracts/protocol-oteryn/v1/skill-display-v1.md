# SKILL_DISPLAY_V1 (NX-7 C14)

Capability 370, command 640. Read-only own-character projection on bundle Worlds with
qualified training and progression content. Existing 279/495 remains unchanged.

## Request

Empty. Unselected capability, nonempty request, stale session/lease/scope/content
or missing progression refuses without changing character state.

## Reply

Exactly 224 bytes, little endian: existing SKILL_PROGRESS_V1 trained snapshot
(160), level u32, total experience u64, level progress hundredths-percent u16,
loyalty skill-point bonus u8, eight loyalty-adjusted absolute levels u16,
current-level-present u8, eight current absolute levels u32. Family order is
magic, fist, club, sword, axe, distance, shielding, fishing. Absent current values
must be all zero. Current values are optional when the live actor owner cannot
qualify its build/conditions; the client displays unavailable, never trained values.
Percentages are 0..10000, bonus is 0..50 in steps of five. Level must be positive.

The durable read extends the existing session/content-fenced Character build
transaction with progression level/XP. It never trains, awards XP or changes loyalty.
Level progress uses admitted native progression thresholds; no extrapolation beyond
the finite content table (last level reports zero). Rounding matches existing
Crystal nearest hundredth, returning zero for unreachable or overfull progress.

Evidence: Crystal 201189f871d23597b82caf88fd611454a2ddefb8,
`src/server/network/protocol/protocolgame.cpp:4386-4388,4420-4433,9380-9406`
and `src/creatures/players/player.cpp:3799-3809` (OTS_HYPOTHESIS_ONLY).
Native differences: u32 level/current values, explicit unavailable current owner,
and finite content bound. Existing loyalty projection and native current skill
owner are reused. Wiki checks for current client skill displays are wiki-pending.

The existing native skills panel prefers 370/640 when negotiated and refreshes
once per second while open. Older sessions continue to use 279/495.
