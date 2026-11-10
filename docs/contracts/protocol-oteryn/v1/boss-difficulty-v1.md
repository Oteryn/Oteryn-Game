# Boss difficulty selection v1

Boss difficulty, capability 225, command 431. Bundle-world only; requires
boss-encounter capability 117. PR #2769 ANSWER 6542714759 accepts the sourced
window and durable draft choice. The owner decision of 2026-11-09 adds individual unlocks and native room admission; scaling remains outside this change.

## Request

The command payload is a strict protobuf-style message, at most 1,024 bytes.
Unknown, repeated, truncated or action-incompatible fields are malformed.

| Field | Wire type | Meaning |
|---|---|---|
| 1 | varint | Action: 0 Start, 1 Cancel, 2 Preview, 3 legacy Open, 4 Open with progress |
| 2 | bytes | Boss production key, only for either Open action, at most 512 bytes |
| 3 | bytes | Server-issued context, exactly 16 nonzero bytes, except either Open action |
| 4 | varint | Unsigned 16-bit difficulty, only for Start/Preview; clamped to 28 |

Either Open requires an enabled entry in the existing server-owned boss room catalogue.
Its immutable revision is copied into a context bound to the admitted character,
world, session, connection generation, lease and runtime scope. Client-supplied
context bytes cannot create or transfer that ownership. A context is a draft for
one encounter attempt, not an instance identity or admission credential.

## Reply

Maximum 2,048 bytes. Field 1 is status 0 (window), 1 (closed) or 2 (typed refusal).
Field 2 contains the context when known. A window also contains field 3 boss key,
field 4 content revision, field 5 selected difficulty and field 6 maximum choice
(boss-specific: Mimar Haffar 25, Maior Domus 15, Phosphorus 28). Fields 8 and 9 carry personal highest defeated difficulty and unlocked maximum. Field 10 carries the
canonical boss creature key so the Bosstiary/Archfoe detail uses the same state.
Action 4 opts into this progress-bearing window and native Start. Legacy action 3
retains the original window bytes, 25 choice bound and no-accepted-profile Start
refusal. The extended client also decodes legacy windows; no invented progress
is substituted for a legacy response. Field 7 is the refusal code: 1 no accepted profile, 2 context closed,
3 missing/foreign context, 4 content changed, 5 boss unavailable, 6 replay conflict, 7 difficulty locked, 8 room admission refused.

For a difficulty-enabled boss, the lever opens this window without entering a room.
Start admits difficulty 1 or a difficulty no higher than the character's personal
highest + 1, capped by the boss maximum. Every member of the lever roster must
have unlocked the selected difficulty. Zero cannot start a difficulty encounter.
Preview retains a draft choice; Cancel closes it. A successful Start returns Closed
only after the existing native activity admission commits and publishes its scope
barrier. An admission refusal leaves the draft available for another command.
Bosses without a sourced difficulty maximum retain the no-accepted-profile refusal.

The native timely boss-death owner records max(previous, encounter difficulty)
for every admitted participant in the same terminal transaction, including offline
participants. Timeout and conflicting or late deaths do not award a victory.
The Phosphorus Record (highscores category 16) includes only recorded Phosphorus
victors, ranked by highest victorious difficulty through the existing snapshots
and highscores dispatch. No pity/bad-luck rule has an official source; none is
transmitted or inferred. The difficulty and Archfoe views share confirmed state.

## Authority and replay

Every request, including replay, proves retained admission publications, recovery,
session use, current character lease, node and runtime scope ownership before it
reads or mutates the private context. All context changes and their immutable
request/reply receipt commit in one transaction under the character root lock.
Old session or connection generations cannot reuse a context. Catalogue revision
changes refuse further preview/start; cancellation may still close the old draft.

## Parity and explicit exclusions

Primary: Crystal `201189f871d23597b82caf88fd611454a2ddefb8`,
`src/server/network/protocol/protocolgame.cpp:11792-11862`: window,
Start/Cancel/Preview, 0..25 clamp. Secondary: Canary
`47dfd51f45280a59a1d3e50ba7edd573d7234446`, same path:3317-3323,
consumes the selection only. Both are `OTS_HYPOTHESIS_ONLY`.

Oteryn extensions accepted in PR #2769: explicit Open action, strict field/context
validation, durable preview choice, immutable replay receipts and a typed
no-accepted-profile refusal. Crystal's Preview is client-local; its Start only
logs and closes, without a gameplay callback. Oteryn keeps a refused Start open
so the client can explain the refusal or cancel the draft.

Owner-supplied source quotations (2026-11-09), TibiaWiki Boss Difficulty System
and Updates/15.30.a30dad: "Higher difficulties must first be unlocked by defeating
bosses in lower difficulties. Every character unlocks progress individually."
"The difficulty selection UI appears once the lever is pulled, the battle will
actually be started through the new UI window." The Phosphorus Record lists
victors by the highest difficulty of a single victorious fight. Sourced maxima
are Mimar Haffar 25, Maior Domus 15 and Phosphorus 28.

No HP/damage/loot multipliers, special-drop probabilities or pity parameters are
invented. Room content, capacity, quest, cooldown and physical-position checks
remain with the existing native activity owner.

Content limitation: the current captured lever catalogue has no room for these
three Summer bosses; entry and normal/final phase sources remain unavailable.
This change does not bypass native template qualification or invent their layouts.
