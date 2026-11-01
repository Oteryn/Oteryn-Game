# Boss difficulty selection v1

W21 selection slice, capability 225, command 431. Bundle-world only; requires
boss-encounter capability 117. PR #2769 ANSWER 6542714759 accepts the sourced
window and durable draft choice. It explicitly excludes scaling and activation.

## Request

The command payload is a strict protobuf-style message, at most 1,024 bytes.
Unknown, repeated, truncated or action-incompatible fields are malformed.

| Field | Wire type | Meaning |
|---|---|---|
| 1 | varint | Action: 0 Start, 1 Cancel, 2 Preview, 3 Open |
| 2 | bytes | Boss production key, only for Open, at most 512 bytes |
| 3 | bytes | Server-issued context, exactly 16 nonzero bytes, except Open |
| 4 | varint | Unsigned 16-bit difficulty, only for Start/Preview; clamped to 25 |

Open requires an enabled entry in the existing server-owned boss room catalogue.
Its immutable revision is copied into a context bound to the admitted character,
world, session, connection generation, lease and runtime scope. Client-supplied
context bytes cannot create or transfer that ownership. A context is a draft for
one encounter attempt, not an instance identity or admission credential.

## Reply

Maximum 2,048 bytes. Field 1 is status 0 (window), 1 (closed) or 2 (typed refusal).
Field 2 contains the context when known. A window also contains field 3 boss key,
field 4 content revision, field 5 selected difficulty and field 6 maximum choice
(25). Field 7 is the refusal code: 1 no accepted profile, 2 context closed,
3 missing/foreign context, 4 content changed, 5 boss unavailable, 6 replay conflict.

Start never returns success while no accepted difficulty profile exists, including
difficulty zero. Its typed refusal leaves the context open and its choice intact.
Preview persists the clamped draft choice and keeps the window open. Cancel
closes the context. A later command against a closed context is refused; an exact
replay returns its retained reply without reapplying the transition. A repeated
command identity with different payload is a replay conflict.

The wire contains known window facts only. No invented race identifier, group or
personal highest, bad-luck figure, banner or stat modifier is transmitted. Native
client presentation remains unsupported. A typed gameplay refusal is an accepted
command envelope containing refusal status 2; invalid authority/payload is a
rejected envelope. Transport/durable uncertainty never claims a commit.

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

Left out: HP/damage/loot scaling, tier unlocks, special drops, bad-luck behavior
and Phosphorus activation. No pinned donor implements them; RT0 decision §17.3
keeps BOSS-DIFFICULTY-0 and Phosphorus PARITY_PENDING. The donor's Phosphorus
70,000/90,000 HP and god-command window figures are placeholders and are not used.
