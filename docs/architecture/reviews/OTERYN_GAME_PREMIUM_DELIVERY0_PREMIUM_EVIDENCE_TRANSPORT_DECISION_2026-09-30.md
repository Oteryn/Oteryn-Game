# PREMIUM-DELIVERY-0 Premium evidence from Platform to Game

- Decision: `PREMIUM-DELIVERY0-PULL-SNAPSHOT-V1`
- Status: **CANDIDATE**. Acceptance needs exact-head validation, independent review (security,
  cross-repository integration) and protected integration here, and the matching producer change
  accepted in `Oteryn/Oteryn-Platform` (PREM-P).
- Role: Sol Supervising Architect (`OTV2_SOL_SUPERVISING_ARCHITECT` 1.3)
- Answers: the owner's direction to start Premium now (2026-09-30, verbatim: "no to wydaj takie
  decyzje i przygotuj zeby to ruszylo", answering the recommendation to start the Platform lane and
  the Game lane in parallel)
- Builds on: `PROD-ENTITLEMENTS-01` (Game consumer contract, `ACCEPTED`, `NOT_STARTED`) §4-§12 and
  §18; `PREMIUM-ACTIVATION-V1` (D69-D76) §4.1 and §5; Platform's
  `OTERYN_V2_ENTITLEMENT_GAME_DELIVERY_CONTRACT.md` (accepted, Profile B); FND-04 (admission)
- Decides what the consumer contract §5, §18 and §21 leave to an integration decision: the
  transport, the evidence message, and the Game's requested product policy values for Premium.
- Cross-repository coordination id: `OTV2-PREMIUM-DELIVERY`. Platform owns its side: this document
  never binds Platform; PREM-P accepts or amends it there.
- Amends, pending on acceptance of PREMIUM-DELIVERY-0: `PREMIUM-ACTIVATION-V1` §4.1 (policy
  revision, product binding, degraded-behaviour owners), §4.5 (relocation only on `EXPIRED`,
  `REVOKED` or no entitlement) and §5 (PREM-1 and PREM-P rows), answering its review (#162
  5913685128).
- Runtime, migration, Platform and production authority: NONE. Each child needs its own
  allocation (Game: #162; Platform: its own coordinator).
- `MERGE_AUTHORITY: WORK_COORDINATOR_ONLY`

## Implementation brief

| Child | Repository | Builds | Depends on |
|---|---|---|---|
| PREM-P | Oteryn-Platform | a Premium time product (`oteryn.premium_time` v1) in ProductsEntitlements (Platform #322, smallest slice): operator and test grants with RBAC, MFA and audit, no payment (D69); the private snapshot endpoint of §3 | Platform's own acceptance of §3-§5 |
| PREM-1 | Oteryn-Game | the consumer fence and classification (consumer contract §6-§8), the snapshot client of §3 with a test producer, the surface policies of PREMIUM-ACTIVATION §4.1, and one `premium_current(account)` read for gameplay | this decision; PREMIUM-ACTIVATION-V1 accepted |
| PREM-2..5 | Oteryn-Game | as PREMIUM-ACTIVATION §5 | PREM-1 |

**Amendment (pending on acceptance of WHEEL-0; `OTERYN_GAME_WHEEL0_WHEEL_OF_DESTINY_DELIVERY_DECISION_2026-09-30.md` §6.2; owner answer W1 a,
#162 5917665342).** The PREM-2..5 row reads: PREM-3, PREM-4 and the Premium blessing service of
PREM-5 depend on PREM-1; PREM-2 and the promotion service of PREM-5 do not, as amended in
PREMIUM-ACTIVATION §5.

PREM-1 and PREM-P run in parallel: PREM-1 tests against a test producer that serves §4 exactly;
the two meet in one cross-repository end-to-end test before activation (consumer contract §22).

## 1. Question

How does Game learn, safely and in time, that an account has Premium?

## 2. Facts

**PROVEN**

- Consumer contract (accepted): Platform is commercial authority, Game enforces (§2); Option C, a
  durable monotonic consumer fence plus a producer-issued finite authority interval (§3, §6); the
  evidence fields of §5; trusted time with `max_clock_skew` (§7); classifications
  `CURRENT_AUTHORITY`, `STALE_WITHIN_BOUND`, `EXPIRED`, `REVOKED`, and others (§8); losing Premium
  never blocks login (§10); reconnect cannot restore expired benefit (§11); no forced logout (§12).
  It leaves the transport, the encoding and the numeric lease values open (§5, §18, §21), and
  requires numeric values before activation (§22).
- Platform's delivery contract (accepted): Profile B covers "an account-level entitlement ... such
  as a future premium/VIP capability"; Platform owns grant, expiry and revocation; operator grants
  are allowed under RBAC, MFA and audit; transport is deferred.
- Platform state (`docs/agents/PROJECT_STATE.md`, `c914564`): Premium and entitlement delivery
  remain Platform issue #322; there is no Premium product or transport yet.
- PREMIUM-ACTIVATION-V1 (candidate): every surface is `REQUIRE_CURRENT`; first grants are
  operator or test grants (D69).
- No Game-to-Platform transport exists; the native pre-admission handoff is not implemented.

## 3. Transport: Game pulls a snapshot (PREM-1 and PREM-P)

- **Pull only.** Game asks Platform for the Premium snapshot of one AccountId over a private
  service endpoint. No push, broker or handoff field in V1: one mechanism, which works before the
  native handoff exists, and whose failure only withholds evidence (consumer contract §8.2: transport
  is not truth).
- **When:** at fresh admission and reconnect (before any Premium benefit), and for each online
  account from `refresh_after` on. Platform sets `refresh_after` in each snapshot (Game requests
  issue + 40 minutes, §5); Game only schedules its next pull from it, and it never affects the
  class. The producer's refresh point is `authority_valid_until` (§5). At most one request in
  flight per account; a failed refresh is retried with backoff until `authority_valid_until`.
- **Authentication:** mutual TLS on the private network between the Game server and Platform, with
  a Platform-issued service identity for the Game server, scoped to this one read purpose. The
  credential is an environment secret; this decision stores none. Platform rotates it with two
  credentials valid at once during a rotation window, and revokes one at once on compromise; a
  Game server with a revoked credential gets no evidence, so Premium reads as Free (login is never
  affected).
- **Replay and binding:** each request carries a fresh 128-bit nonce; the response echoes it, and
  its `account_id` equals the requested AccountId; a response failing either is dropped and fails
  closed. The durable fence (§6) rejects any older revision.
- **Size:** a response is at most 1,024 bytes (`PREMDEL0-RL-01`); anything larger or malformed fails
  closed for benefit.

## 4. Evidence message (both sides)

A versioned JSON object, `oteryn.premium_snapshot.v1`:

```text
schema:                "oteryn.premium_snapshot.v1"
producer_revision:     Platform build SHA (provenance only, consumer contract §4)
producer_profile:      "oteryn.entitlement.profile_b.v1" (the semantic profile revision)
nonce:                 echo of the request nonce
account_id:            AccountId
product_id:            "oteryn.premium_time"
product_version:       1
entitlement_id:        EntitlementId, or null when the account has none
entitlement_state:     ACTIVE | NOT_YET_EFFECTIVE | EXPIRED | REVOKED | NONE
lifecycle_revision:    u64, monotonic per entitlement
authority_revision:    u64, monotonic per account snapshot series
effective_from:        RFC 3339 UTC, absolute
effective_until:       RFC 3339 UTC, absolute (the paid-up end of Premium time)
authority_issued_at:   RFC 3339 UTC
authority_valid_until: RFC 3339 UTC, at most issued_at + max_authority_lease
refresh_after:         RFC 3339 UTC
```

- Several Premium grants are merged by Platform into one account-level entitlement with a stable
  `entitlement_id` and one interval; Game never adds intervals itself.
- Every new snapshot for an account carries a strictly higher `authority_revision`, including a
  pure lease renewal; `lifecycle_revision` rises with each grant, expiry or revocation change.
- `entitlement_state` is the producer's lifecycle state. Game derives its class from the absolute
  times (consumer contract §8.3) and, where the two differ, the more restrictive wins (§8.2): for
  example `NOT_YET_EFFECTIVE` with a start in the past still reads as not effective.
- An unknown `producer_profile` fails closed for benefit. PREM-P and PREM-1 each record the
  compatibility pair (`producer_profile`, `product_version`) they support, as consumer contract §4
  requires, and the end-to-end test checks both records.
- Unknown `schema`, `product_id` or `product_version` fail closed for benefit (consumer contract
  §4).

## 5. Requested Premium product policy (PREM-P decides)

The consumer contract makes these Platform's product/version policy; Game requests:

| Value | Requested | Why |
|---|---|---|
| `max_authority_lease` | 60 minutes | bounds how long a revocation can go unseen |
| producer refresh point | `authority_valid_until` | no stale window: evidence is current until the lease ends |
| `max_clock_skew` | 5 seconds | the FND-04 security-source bound; Game servers run NTP |
| bounded stale use | not permitted | PREMIUM-ACTIVATION §4.1 is `REQUIRE_CURRENT` on every surface |

Game requests `refresh_after` = issue + 40 minutes (set by Platform in the snapshot), so a Platform
outage longer than about 20 minutes suspends Premium benefits (never login) at `authority_valid_until`
until a refresh succeeds; nothing is lost, D73/D76 already define benefits checked at use, and a
lapsed lease never relocates a character (§6).

**Time.** Game evaluates the absolute times against the node's clock, synchronized by NTP, with
uncertainty at most `max_clock_skew`; a node whose clock is not synchronized treats Premium as not
current (consumer contract §7, ENT-CDF-04).

## 6. Game side (PREM-1)

- The fence of consumer contract §6.1: one durable row per (AccountId, EntitlementId) with its
  `lifecycle_revision` high water and latest accepted evidence, plus one row per AccountId with the
  `authority_revision` high water, both written crash-consistently before any benefit uses them.
  A snapshot naming another `entitlement_id` than before opens a new row and never lowers the
  account high water; a snapshot with no entitlement (`NONE`) advances the account high water and
  reads as Free, so an older `ACTIVE` snapshot can never come back.
- `premium_current(account, now)` is true only for `CURRENT_AUTHORITY` (§8.3); every other class is
  Free. Every Premium check in Game (PREM-2..5, the depot limit, charms, and the Market and house
  gates once the owner's pre-Premium answers end) reads it.
- `premium_entitlement_ended(account, now)` is true only when the latest accepted evidence says the
  entitlement itself ended: producer state `EXPIRED`, `REVOKED` or `NONE`, or `effective_until`
  passed. A lease past `authority_valid_until`, missing evidence or a failed pull never makes it
  true. PREMIUM-ACTIVATION §4.5's login relocation reads only this predicate.
- **Switch-over.** The owner's answers 4 and 5 (#162 5913348961) end when this consumer is
  delivered: PREM-1's activation record names the date, and the Market and house gates then read
  `premium_current`.

## 7. Rejected options

- **Premium in the login handoff only.** The native handoff is not built, and a running session
  still needs refresh.
- **Push events from Platform.** A broker adds a component and ordering rules; a pull with a fence
  is enough.
- **A Game-side "everyone is Premium" switch.** PREMIUM-ACTIVATION rejects it (D69).
- **Bounded stale use.** It would contradict `REQUIRE_CURRENT`.

## 8. Decision test

- **Must decide now:** YES. The owner asked for Premium now, and both lanes need the same message.
- **Minimum sufficient:** one pull endpoint, one message, one fence row per (account, entitlement)
  plus one account high-water row.
- **Superseding evidence:** Platform's PREM-P choosing other values (they are Platform's), or a
  measured outage profile.
- **Deliberately not decided:** payment, prices, the Store, VIP, push delivery.

## 9. Before-freeze checklist

1. **Contract amendments:** none in this repository; PREM-P records the producer side in Platform.
2. **Serialization:** one fence row per (account, entitlement) and one account high-water row,
   monotonic revisions.
3. **Restart:** the fence and evidence are durable; a restart re-pulls before any benefit.
4. **Typed references:** AccountId, EntitlementId, revisions, absolute UTC times.
5. **Wire:** no client wire; a private service endpoint (§3, §4).
