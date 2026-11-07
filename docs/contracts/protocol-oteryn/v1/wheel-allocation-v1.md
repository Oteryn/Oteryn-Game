# Wheel allocation v1

Command 161 requires offered and selected capability 148 (`WHEEL_ALLOCATION_V1`). This is a
native addition to protocol-oteryn; no Canary packet compatibility is implied. The client
opens the playable Wheel and Atelier panel with Ctrl+W when capability 148 is selected.

The canonical codec is `crates/protocol-oteryn/src/wheel.rs`. Requests and results are bounded
to 256 bytes. Unknown fields, duplicate scalar fields and malformed shapes are rejected.

| Request field | Wire tag | Value |
| --- | --- | --- |
| action | 1, varint | 0 query, 1 replace/save, 2 reset |
| occurrence | 2, bytes | Exactly 16 bytes; mutations require a UUIDv7 receipt identity |
| expected_revision | 3, varint | Expected Wheel revision, including 0 for the initial allocation |
| points | 4, repeated varint | Exactly 36 unsigned 16-bit values in state-slot order |
| vessels | 5, repeated bytes, optional extension | Exactly four entries in green/red/purple/blue order; empty bytes clear a vessel, otherwise exactly 16 opaque gem-identity bytes |

A query contains only action 0. Both mutations carry occurrence, expected_revision and points. Reset requires
36 zero points. An absent vessel extension preserves the current vessels. A present extension
replaces all four atomically with the points, under the same expected Wheel revision and
occurrence receipt. Duplicate nonempty gem identities, partial vectors and malformed lengths
are rejected by the codec. Gem identities are opaque values issued by Atelier, including the
SHA-derived initial gems; they are not required to have UUID version bits. The durable owner
checks the Character's ownership, matching domain and current gem ruleset before any write.
Socketing, replacing or removing a vessel alone requires no temple; reducing points still does.

The result has `refused` (tag 1, varint boolean), `revision` (tag 2, varint), and 36 current
`points` (tag 3, repeated varint). A request with the vessel extension also receives four current
`vessels` (tag 4, repeated bytes, with the same encoding). Requests without that extension keep
their original reply bytes, including queries. `ATELIER_QUERY` also reports each gem's vessel
state. A well-formed refused allocation returns `ACCEPTED` with
`refused=true` and the unchanged committed allocation. Malformed, unnegotiated and stale
admission requests return `REJECTED`; unavailable or indeterminate durable operations end the
connection, allowing the same occurrence to reconcile through its retained receipt.

The server derives level, vocation, promotion, Premium and temple eligibility from their owners.
Any decrease, including action 1 that redistributes points, requires a protection-zone tile
within X distance 1 and Y distance 10 of the temple. Crystal's two-parameter range overload
does not compare the temple's floor (`player_wheel.cpp:2151`, `position.hpp:41–44`); the
current tile must still supply qualified protection-zone evidence. Action 2 supplies no location authority. A map without
retained temple evidence refuses decreases. W-2 qualifies the native source town; bundle
decreases fail closed without town evidence.
Allocation and vessel changes reload both committed projections before replying and refresh
the existing Wheel stat contribution. Receipt replay reloads current committed rows without
reapplying the change or spending resources. Premium and promotion remain owner-supplied
eligibility facts for both changes and runtime effects.

## Shared mutation interval

Fresh Wheel saves, resets, vessel changes and Atelier locks, reveals, dismantles,
domain changes and grade upgrades share one 490 ms ExAction interval per native
session. The existing Channel owner uses its monotonic clock; the separate
200 ms world-action interval is unchanged. An exhausted attempt returns a normal
rule refusal without changing state, spending resources or extending the interval.
A fresh admitted attempt starts the interval even if a later gameplay rule refuses it.

The durable occurrence lock and immutable receipt lookup precede this admission
check. An exact replay returns its retained result during the interval, and an
internal Character revision retry keeps the first admission decision. Server-owned
Atelier initialization does not start an interval.

Read-only command 161 (action 0) and command 308 are exempt: together they provide
the native panel projection and remain usable immediately after a mutation. This is
an explicit native wire adaptation of Crystal `201189f` `game.cpp:11325-11396`,
whose single OpenWheel response includes the panel data. The 490 ms value is
`config.lua.dist:469` (500 ms) minus the source's 10 ms adjustment. Coordinator
[ANSWER 6544165094](https://github.com/Oteryn/Oteryn-Game/pull/2771#issuecomment-6544165094)
authorizes this read exemption.

MONKQUEST3 (owner decision 2026-10-30) supplies the accepted Monk-quest extra-point
source: the once-only final report to Enpa-Deia Pema contributes 10 points, owned
by `content/quests/monk-pilgrimage.json`. The Wheel allocation transaction reads the
durable final-report flag under its Character lock and adds this reward to the
existing level and gem points with checked arithmetic. Repeated reports cannot
multiply the reward. Ordinary Wheel eligibility and allocation rules still apply.
