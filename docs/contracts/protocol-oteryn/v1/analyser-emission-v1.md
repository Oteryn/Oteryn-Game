# Capability 10: live analyser emission

S-ANALYSER amends the unoffered analyser v1 schema before production enablement.
State domain 15 keeps its existing empty snapshot, monotonic session revision,
bounded fact queue and delta framing.

`AnalyserFactV1.kill` (oneof field 7, `AnalyserKillV1`) reports exactly one committed
creature death credited to the live loot-right holder. Its race is the existing
Bestiary wire index, or zero when absent. A zero-loot kill still emits `kill`.
`kill_loot` reports committed items and gold only: clients MUST NOT count kills
from these messages. Any number of loot fragments can follow a kill, including
across delta batches. Each fragment carries its race for independent aggregation.
No kill identifier or fragment adjacency is needed to count kills or aggregate loot.
This supersedes the earlier interpretation of `kill_loot` as both a kill and loot.

Facts come from final applied combat receipts, committed item consumption and
committed reward receipts. Loot output includes entries committed before a later
entry failure; replayed settlements share transient per-corpse emission receipts.
XP uses the committed base before party sharing and the final awarded amount,
including zero awarded XP.
No analyser fact is persisted or consulted by a gameplay rule.

Only selected sessions receive queues or domain 15. Queues are bounded by the
existing 1,024-fact cap and batches by 64 facts/4,096 bytes. Recovery discards pending
facts and increments the retained revision once when it does so. Reconnect and
scope transfer never reset the revision. Exact actor/session and connection
bindings prevent a stale socket from draining or clearing a successor's queue.


## Outgoing charm damage

Negotiated capability 625 `ANALYSER_CHARM_SOURCE_V1` requires capabilities 6 and 10
and extends domain 15, delta type 1. In `AnalyserImpactV1`, field 4 is the canonical
`charm_key` and field 5 is the target `EntityRefV1` with a nonzero generation.
Both fields appear together only on a committed outgoing charm damage impact.
Without 625, impacts use the existing encoding. Capability 486 independently
governs the attacker on incoming damage. Batch and fact bounds are unchanged.

The player receives the damage line through existing Game chat feedback for the
Server Log, with the lowercased charm display name as the suffix. The damage
line is produced from applied health loss, including lethal hits.
