# INSPECT-1 native inspection v1

Status: candidate for INSPECT-1; acceptance is effective after operator review, CI and protected integration.

Capability 132 (`INSPECTION_V1`) requires item views (4) and exact entity references (6).
Command 153 (`INSPECTION`) is read-only. Responses use the command result payload.
Domain 97 is allocated to inspection, with no unsolicited snapshot or delta in v1.
No durable state, audit cause, writer or SQL migration is introduced; migration 0262 remains unused.
Game owns item and equipment truth; World and Channel identities stay distinct.

## Request

Strict protobuf subset: exactly one of the following fields, no duplicate or unknown fields.

| Field | Wire type | Meaning |
| --- | --- | --- |
| 1 | bytes | Existing extended LOOK payload selecting an ItemHandle only |
| 2 | bytes | Existing extended LOOK payload selecting an exact player EntityRef only |
| 3 | uint32 | Trade leaf index shifted left one, with counter-offer flag in bit zero |
| 4 | uint32 | Nonzero active-generation item definition index (catalogue details) |
| 5 | bytes | Permission action submessage: field 1 uint32 action, field 2 bytes extended LOOK player target |

Maximum 128 bytes. A handle must still resolve in the session table and in current owner
facts after the durable metadata read. Ground uses current native map visibility, not
pickup adjacency. Trade inspection uses the live trade owner's current pair, same-floor
range (two cells), sight and fence/custody checks before and after metadata I/O. The current
trade owner admits leaves only, so an index other than zero is unavailable. Counterpart
items require both sides to have offered; no arbitrary player inventory read is exposed.
Player targets must be shown with their exact generation and still visible at the read.
Permission actions are 1 request inspection, 2 grant, 3 revoke, 4 enable allow-all,
5 disable allow-all and 6 poll pending requests. Only 1–3 require a player target;
4–6 reject a target. The target receives pending exact player references by polling,
filtered to its current session visibility. Requests expire after 60 seconds.
Another player's equipment requires an individual grant or that target's allow-all setting.
Both grant endpoints bind to exact actor generations and logical GameSessions. Session release
forgets incoming/outgoing requests and grants; channel transfer or actor replacement does not
inherit permission. Allow-all is session-local; a native client may resend its preference on
admission. The reply contains equipment and the main backpack item, never bag entries.
At most 32 pending requests and 32 individual grants per owner, and 1,024 owners per Channel.
LOOK and inspection share the existing ten-per-sliding-second GameSession budget, retained
across resume/transfer. Malformed commands or unselected capabilities are rejected.

## Reply

Field 1 uint32 is the status: 1 success, 2 unavailable/stale, 3 not visible, 4 out of range,
5 rate limited, 6 permission denied, 7 request queued, 8 permission acknowledgement.
Field 2 is a repeated inspected-item submessage. Field 3 is repeated bytes carrying extended
LOOK player targets, only in status 8 replies to a pending-request poll (at most 32).
Refusals and permission replies carry no items. An empty equipment success is valid. At most ten items and 65,536 encoded bytes.

An inspected item has singular uint32 fields 1 slot (0 single item, 1–9 existing native equipment slots, 10 main backpack),
2 definition index (nonzero), 3 quantity (nonzero), 4 stored tier (0–10); field 5 is its
nonempty UTF-8 display name. Repeated field 6 carries label/value pairs, whose fields
1 and 2 are nonempty UTF-8 strings. At most 64 pairs, 1,024 bytes per string; NUL is refused.
No item-instance identity, account identity or internal production key is returned.
The command does not move items, grant durable authority or checkpoint active timers.
Unknown content facts are omitted rather than fabricated. Stored metadata is never a
custody witness and cannot authorize writes.

The active content pin supplies name, description, weapon/protection attributes, known
requirements, container capacity and physical weight. Instance reads supply current
stored forge tier, imbuements and timed charges/budget. Container Total Weight includes
the whole graph (4,096 nodes, path length 32), rejecting cycles/overflow/unknown weights.
Catalogue reads do not invent applied imbuements or instance tier.

## Reference evidence and remaining parity

[Canary protocol](https://github.com/opentibiabr/canary/blob/47dfd51f45280a59a1d3e50ba7edd573d7234446/src/server/network/protocol/protocolgame.cpp)
and [Canary descriptions](https://github.com/opentibiabr/canary/blob/47dfd51f45280a59a1d3e50ba7edd573d7234446/src/items/item.cpp)
separate opcode 0xCD inspection from LOOK; item details are label/value pairs.
[Crystal protocol](https://github.com/zimbadev/crystalserver/blob/201189f871d23597b82caf88fd611454a2ddefb8/src/server/network/protocol/protocolgame.cpp)
provides the same item and self-equipment projection. Canary's `parseInspectPlayer` consumes
requests without implementing the consent exchange; Crystal has no such handler at this pin.
[Tibia's official announcement](https://www.tibia.com/news/?id=3910&subtopic=newsarchive)
confirms takeable-item details and requires permission before other-player equipment is shown.
[TibiaWiki inspection hints](https://tibia.fandom.com/wiki/Random_Hints) describe weight,
body position, imbuements and remaining charges. Reference-only numeric values and exact
formatting are **not verified against Global**, under owner decision 22.

Tier labels include the pinned default Onslaught/Momentum/Ruse/Transcendence/Amplification
percentages by admitted item category. Those display values do not become combat authority.
An augment can carry optional source-qualified `ProjectV2AugmentBinding.inspection` text;
absent text remains unknown. It is nonempty UTF-8, at most 1,024 bytes, without NUL, and omitted
from canonical serialization when absent, preserving existing content bytes. Inspection joins
known augment descriptions under the `Augments` label. Current production profiles contain no
augment bindings. Generic candidate fields are not interpreted as display or gameplay rules.
General character summary/outfit are outside this equipment-only projection. Duration wording
uses native stored time rather than Canary's `brand-new` placeholder. Unknown native facts,
including absent source flags for some optional description labels, remain omitted. These
qualification limits are retained in INSPECT-1-FINAL.md.

## Compatibility and decision scope

This additive extension is **server-first-safe**: older clients do not select capability 132
and cannot send command 153. The server sends no inspection payload without selection.
The Game server produces replies; the future native client consumes this one Game-owned
schema. Rollback withdraws the offer and rejects the command; it requires no item-state
rollback or Platform deployment. No cross-repository identity/session contract changes.

The native request/reply and consent boundary must be decided now to implement INSPECT-1
without disclosing another player's equipment by visibility alone. The compatibility burden
is this capability's schema and allocated numbers. Published Global behavior, a concrete
privacy defect or consumer interoperability evidence can justify a versioned replacement.
Client UI, persistent client preferences, future trade-container support and combat effect
rules are deliberately not selected here. Independent byte-level golden cases accompany
round-trip/malformed/limit tests; shared client/server codecs are not the sole wire oracle.
