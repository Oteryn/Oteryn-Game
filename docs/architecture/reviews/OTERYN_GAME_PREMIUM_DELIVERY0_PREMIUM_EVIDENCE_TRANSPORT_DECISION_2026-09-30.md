# PREMIUM-DELIVERY-0 Premium evidence from Platform to Game

- Decision: `PREMIUM-DELIVERY0-PULL-SNAPSHOT-V1`
- Status: **CANDIDATE**. Acceptance needs exact-head validation, independent review (security,
  cross-repository integration) and protected integration here, and the matching producer change
  accepted in `Oteryn/Oteryn-Platform` (PREM-P).
- Role: Sol Supervising Architect (`OTV2_SOL_SUPERVISING_ARCHITECT` 1.3)
- Amended (2026-09-30): §3.1, the request and response handling, answering PREM-1b's
  `ARCHITECTURE_ESCALATION_REQUIRED` (#162 5916078350).
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
  flight per account; a failed pull is retried with backoff until one succeeds (§3.1).
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

### 3.1 Request and responses (both sides; ruling on PREM-1b, #162 5916078350)

PREM-1a stopped because §3 and §4 fix the response, not the request. This fixes the request so
PREM-1b's client and test producer and PREM-P serve the same exchange.

- **Request:** `POST /v1/premium/snapshot` over the §3 mutual TLS channel, with
  `Content-Type: application/json` and this body, at most 256 bytes (`PREMDEL0-RL-02`):

  ```text
  schema:     "oteryn.premium_snapshot_request.v1"
  account_id: AccountId, canonical lowercase hyphenated UUID (8-4-4-4-12)
  nonce:      32 lowercase hexadecimal characters (the 128-bit nonce of §3)
  ```

- **Text forms.** `account_id` in the request and in the §4 response uses the same canonical
  lowercase hyphenated form as the existing Platform contracts; `nonce` is echoed byte for byte.
  Game compares both as exact strings; any other form fails closed.
- **200** carries the §4 body only, with `Content-Type: application/json`. An account without a
  Premium entitlement, including one Platform has never granted, is a 200 with
  `entitlement_state` `NONE`, `entitlement_id` null and Platform's normal `authority_revision`
  series: there is no separate "unknown account" answer.
- **Anything else is unavailable:** any other status, a redirect (never followed), another content
  type, a timeout after 5 seconds (`PREMDEL0-RL-03`), or a TLS failure. A 200 whose body is
  malformed or dropped under §3 or §4 (size, JSON shape, nonce, `account_id`, the closed `NONE`
  variant), or that the fence (§6) rejects as stale without contradiction (consumer contract §8.1
  rules 1 and 3), counts as a failed pull too. The two semantic failures below are not failed
  pulls.
- **Semantic failures are `INVALID_OR_CONFLICTING`, not unavailable** (consumer contract §8.1
  rules 2 and 4, §8.3, §16). A well-formed 200 on the §3 channel, with the echoed nonce and the
  requested `account_id`, is authenticated evidence; it is classified `INVALID_OR_CONFLICTING`
  when either:
  1. **Same-revision contradiction:** it carries an `authority_revision` the account has already
     accepted (the current high water or a retained historical one, §6) and any field other than
     `nonce` and `producer_revision` differs from that accepted evidence. An identical payload is
     an exact replay (§6.2 rule 3) and is a successful pull.
  2. **Unsupported or downgraded semantics:** its `schema`, `producer_profile`, `product_id` or
     `product_version` is outside the compatibility pair PREM-1 records (§4), or its profile or
     version is older than one the account has already accepted (a downgrade).

  Such a response is never accepted as evidence and never moves the high water. PREM-1 records a
  durable conflict marker on the account's fence row (§6) before any later benefit check, raises a
  security audit event (consumer contract §15), and denies Premium (login unaffected). The marker
  survives later successful pulls and restarts: consumer contract §8.3 requires reconciliation,
  not a retry. **Declared V1 deferral:** V1 defines no Premium reconciliation, so no path clears
  the marker; the account's Premium stays denied until a later accepted decision defines
  reconciliation (consumer contract §13 covers only Profile C/D). This decision grants no
  authority to clear it.
- **A failed pull denies Premium at once.** A failed admission, reconnect or refresh pull yields no
  new evidence: the fence keeps its last accepted evidence unchanged, but the account's class is
  `AUTHORITY_UNAVAILABLE` from that failure until a later pull for the account succeeds, even while
  the cached `ACTIVE` evidence is still inside its interval. This is consumer contract §8.3: a
  failed refresh is `STALE_WITHIN_BOUND` only where stale use is permitted, and it is not
  (PREMIUM-ACTIVATION §4.1 is `REQUIRE_CURRENT` on every surface; §5), so it is
  `AUTHORITY_UNAVAILABLE`, which denies benefit (its §16 row "Same outage; surface requires
  current"). A conflict marker (`INVALID_OR_CONFLICTING`, §8.1) and a restrictive class from the
  kept evidence (`REVOKED`, `EXPIRED`, `NOT_YET_EFFECTIVE`, §8.2) still win over it, and reaching
  `authority_valid_until` or `effective_until` is still the separate transition to `EXPIRED`. The
  §3 retry with backoff continues; a 429 or 503 `Retry-After` is honoured within that backoff.
  Login is never affected. A failed pull, like a semantic failure, leaves
  `premium_entitlement_ended` (§6) unchanged: it reads only the kept evidence, so it stays true
  when that evidence already ended the entitlement and never becomes true from the failure.
- **Test producer** (PREM-1b): an in-process server that speaks exactly this exchange and the §4
  body, used by PREM-1's tests and by the cross-repository end-to-end test's Game half.
- PREM-P accepts or amends this in Platform (the cross-repository note in the header); a Platform change
  of path or form updates this section before activation.

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
entitlement_id:        EntitlementId, or null (NONE only)
entitlement_state:     ACTIVE | NOT_YET_EFFECTIVE | EXPIRED | REVOKED | NONE
lifecycle_revision:    u64, monotonic per entitlement; 0 (NONE only)
authority_revision:    u64, monotonic per account snapshot series
effective_from:        RFC 3339 UTC, absolute; null (NONE only)
effective_until:       RFC 3339 UTC, absolute (the paid-up end of Premium time); null (NONE only)
authority_issued_at:   RFC 3339 UTC
authority_valid_until: RFC 3339 UTC, at most issued_at + max_authority_lease
refresh_after:         RFC 3339 UTC
```

- **The `NONE` variant** (an account that has never held a Premium entitlement) is closed:
  `entitlement_id` null, `lifecycle_revision` 0, `effective_from` and `effective_until` null;
  every other field, including the four authority fields, is present and carries the same rules as
  for any snapshot, so a `NONE` advances the account fence (§6) like any other. Any other
  combination fails closed: a null in a non-`NONE` state, a non-null interval, entitlement id or
  non-zero `lifecycle_revision` in `NONE`. Game's class for `NONE` is Free. An account that once
  held Premium never returns to `NONE`: its merged entitlement stays `EXPIRED` or `REVOKED`.
- Several Premium grants are merged by Platform into one account-level entitlement with a stable
  `entitlement_id` and one interval; Game never adds intervals itself.
- Every new snapshot for an account carries a strictly higher `authority_revision`, including a
  pure lease renewal; `lifecycle_revision` rises with each grant, expiry or revocation change.
- `entitlement_state` is the producer's lifecycle state. Game derives its class from the absolute
  times (consumer contract §8.3) and, where the two differ, the more restrictive wins (§8.2): for
  example `NOT_YET_EFFECTIVE` with a start in the past still reads as not effective.
- An unknown or downgraded `producer_profile` is `INVALID_OR_CONFLICTING` (§3.1; consumer
  contract §8.1 rule 4). PREM-P and PREM-1 each record the compatibility pair (`producer_profile`,
  `product_version`) they support, as consumer contract §4 requires, and the end-to-end test checks
  both records.
- An unknown `schema`, `product_id` or `product_version` is `INVALID_OR_CONFLICTING` likewise
  (§3.1; consumer contract §4, §8.1 rule 4).

## 5. Requested Premium product policy (PREM-P decides)

The consumer contract makes these Platform's product/version policy; Game requests:

| Value | Requested | Why |
|---|---|---|
| `max_authority_lease` | 60 minutes | bounds how long a revocation can go unseen |
| producer refresh point | `authority_valid_until` | no stale window: evidence is current until the lease ends or a pull fails (§3.1) |
| `max_clock_skew` | 5 seconds | the FND-04 security-source bound; Game servers run NTP |
| bounded stale use | not permitted | PREMIUM-ACTIVATION §4.1 is `REQUIRE_CURRENT` on every surface |

Game requests `refresh_after` = issue + 40 minutes (set by Platform in the snapshot). A pull that
fails suspends Premium benefits (never login) at once as `AUTHORITY_UNAVAILABLE` (§3.1), not at
`authority_valid_until`; the lease cutoff is only the separate expiry for evidence that is never
refreshed. Benefits return when a pull succeeds (not while a §3.1 conflict marker is set); nothing
is lost, D73/D76 already define benefits checked at use, and neither an unavailable class nor a
lapsed lease relocates a character (§6).

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
- The account row also holds the §3.1 conflict marker and a fingerprint of the fields compared in
  §3.1 for each accepted `authority_revision`, kept for at least 30 days (`PREMDEL0-RL-04`). PREM-1
  records this horizon; a response whose `authority_revision` fingerprint is past it cannot be
  compared and is rejected as stale, and PREM-1 claims no equivocation detection beyond it
  (consumer contract §6.2). A fence whose continuity is unsafe is `INVALID_OR_CONFLICTING` too
  (consumer contract §6.4).
- `premium_current(account, now)` is true only for `CURRENT_AUTHORITY` (§8.3) with no conflict
  marker; every other class is Free. Every Premium check in Game (PREM-2..5, the depot limit,
  charms, and the Market and house gates once the owner's pre-Premium answers end) reads it.
- `premium_entitlement_ended(account, now)` is true only when the latest accepted evidence says the
  entitlement itself ended: producer state `EXPIRED`, `REVOKED` or `NONE`, or `effective_until`
  passed. A lease past `authority_valid_until`, missing evidence, a failed pull or a semantic
  failure (§3.1) never makes it true, and none of them clears it once true. PREMIUM-ACTIVATION
  §4.5's login relocation reads only this predicate.
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
2. **Serialization:** one fence row per (account, entitlement) and one account high-water row
   (with the conflict marker and bounded revision fingerprints, §6), monotonic revisions.
3. **Restart:** the fence and evidence are durable; a restart re-pulls before any benefit.
4. **Typed references:** AccountId, EntitlementId, revisions, absolute UTC times.
5. **Wire:** no client wire; a private service endpoint (§3, §3.1, §4).
