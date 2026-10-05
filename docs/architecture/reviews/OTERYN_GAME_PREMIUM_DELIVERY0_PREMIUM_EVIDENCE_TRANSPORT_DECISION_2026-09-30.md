# PREMIUM-DELIVERY-0 Premium evidence from Platform to Game

- Decision: `PREMIUM-DELIVERY0-PULL-SNAPSHOT-V1`
- Status: **CANDIDATE**. Acceptance needs exact-head validation, independent review (security,
  cross-repository integration) and protected integration here, and the matching producer change
  accepted in `Oteryn/Oteryn-Platform` (PREM-P). The Game side is accepted when this document's
  exact head passes that review and merges here; it then binds Game's PREM-1b and PREM-2..5 and
  the amendments below take effect. Activation (§10.3) still needs PREM-P's acceptance.
- Role: Sol Supervising Architect (`OTV2_SOL_SUPERVISING_ARCHITECT` 1.3)
- Amended (2026-09-30): §3.1, the request and response handling, answering PREM-1b's
  `ARCHITECTURE_ESCALATION_REQUIRED` (#162 5916078350).
- Amended (2026-10-03): §4, the `refresh_after` bound; §10, reconciled with `main` after PREM-1a (#1391), and §11, the PREM-1b
  packet, for Game-side acceptance (control plane D350, owner answer 1a).
- Amended (2026-10-03, PREM-DELIVERY-0A, control plane D369): §3.1 and §11 scope items 3 and 7,
  the full envelope before `Unsupported` and failures older than the latest proof.
- Amended (2026-10-04, PREM-E2E-1, owner D540): §12 reconciles §3, §3.1, §4 and §5 with the
  producer Platform accepted (PREM-P), and §10.3 gains the cross-repository end-to-end test.
  Where §12 and an earlier section differ, §12 governs.
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
| PREM-1a (merged, #1391) | Oteryn-Game | the consumer fence and classification (consumer contract §6-§8), the surface policies of PREMIUM-ACTIVATION §4.1, and the `premium_current(account)` and `premium_entitlement_ended(account)` reads (§10.1) | this decision |
| PREM-1b | Oteryn-Game | the §3/§3.1 snapshot client over mutual TLS, nonces, the admission, reconnect and refresh pulls, the failed-pull state, and the in-process test producer (§11) | Game-side acceptance of this decision; PREM-1a |
| PREM-2..5 | Oteryn-Game | as PREMIUM-ACTIVATION §5 | PREM-1 (PREM-1a for code; PREM-1b for a live Premium read) |

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

- **Request** (path amended by §12.1): `POST /v1/premium/snapshot` over the §3 mutual TLS channel, with
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
     version is older than one the account has already accepted (a downgrade). This applies only
     to a complete, well-formed envelope: every §4 field present with its baseline type and
     value form (enumerations, RFC 3339 timestamps, tokens, the closed `NONE` variant, and the
     structural interval checks: a positive lease, `authority_issued_at` before
     `authority_valid_until`, `refresh_after` inside it). A response that fails any of these is
     malformed (a failed pull) whatever its `schema` or profile says, so such a body never records
     the permanent marker. A well-formed lease longer than `max_authority_lease` (§4) is a
     product-policy mismatch, not a form failure: it stays `Unsupported` and is checked together
     with the compatibility pair, after the form checks.

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
refresh_after:         RFC 3339 UTC, authority_issued_at <= refresh_after <= authority_valid_until
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

PREM-P has decided these values. §12.2 records its decision, which amends the refresh point and
stale use below.

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

## 10. Reconciliation with `main` (2026-10-03)

### 10.1 What PREM-1a delivered (#1391, migration 0029)

PREM-1 was split for size (the archived record `OTV2-20260930-prem-1.md`). PREM-1a on `main`
matches §4-§6:

- **Fence (§6, §9 item 2).** `game_premium_evidence` is an immutable log keyed by (account,
  `authority_revision`) with a SHA-256 fingerprint of the §3.1 compared fields;
  `game_premium_account_fence` holds the account high water and the sticky conflict marker
  (`conflict_authority_revision`); `game_premium_entitlement_fence` holds the (account,
  entitlement) `lifecycle_revision` high water and lifecycle fingerprint. Triggers let rows only
  advance, reject clearing the marker, and reject deletes.
- **Message (§4).** `premium::snapshot` validates `oteryn.premium_snapshot.v1` at most 1,024 bytes
  (`PREMDEL0-RL-01`), the nonce and account binding, the closed `NONE` variant and the lease bound
  (with the defects in §10.2).
  The compatibility pair PREM-1 records (§4) is `producer_profile`
  `"oteryn.entitlement.profile_b.v1"`, `product_version` 1, with `product_id`
  `"oteryn.premium_time"`.
- **Policy (§5).** `MAX_AUTHORITY_LEASE_US` 60 minutes, `MAX_CLOCK_SKEW_US` 5 seconds,
  `REQUIRE_CURRENT` on every surface (`premium-surfaces-1`); trusted time is `TrustedNow`, and
  without it Premium is not current.
- **Reads (§6).** `PremiumConsumer::premium_current` and `premium_entitlement_ended`, with the
  durable fence re-proved after a restart before any benefit (§9 item 3).
- **Stricter than this document, kept.** PREM-1a encodes the per-lifecycle-revision state machine
  (NOT_YET_EFFECTIVE -> ACTIVE -> EXPIRED only as producer-issued time crosses each boundary);
  any other change, entering or leaving `REVOKED`, a lower `lifecycle_revision` under a higher
  `authority_revision`, or reopening a withdrawn entitlement without a higher
  `lifecycle_revision`, is `INVALID_OR_CONFLICTING`. These follow from §4 ("`lifecycle_revision`
  rises with each grant, expiry or revocation change") and are part of this decision. A fence
  write failure quarantines the account as `INVALID_OR_CONFLICTING` (consumer contract §6.4).

### 10.2 Corrections

- **Three PREM-1a defects; PREM-1b fixes them (§11 scope items 6 and 7).** Neither is reachable in
  production yet: PREM-1a has no client, so no evidence has been ingested.
  1. **The decoder rejects the canonical `NONE`.** `snapshot::Wire` declares `effective_from` and
     `effective_until` as strings, so the producer's `NONE` form (§4: both null,
     `entitlement_id` null, `lifecycle_revision` 0) fails as `Malformed` before the closed-variant
     check, and every free account would count as a failed pull. PREM-1a's `NONE` test leaves both
     times set and `lifecycle_revision` 3, which hides this. §4 stands; the decoder must accept the
     null interval in `NONE` only.
  2. **The fingerprint omits `refresh_after`.** §3.1 rule 1 compares every field except `nonce` and
     `producer_revision`. `PremiumEvidence::fingerprint` excludes `refresh_after_us` as
     scheduling-only, and a PostgreSQL case asserts that a changed `refresh_after` at the same
     `authority_revision` is a replay. §3.1 stands: two snapshots under one revision that differ in
     their refresh schedule are a contradiction, and the response is `INVALID_OR_CONFLICTING`,
     not a successful pull. The stored fingerprint must cover `refresh_after`.
  3. **An unsupported response is not durable.** §3.1 rule 2 (unknown or downgraded `schema`,
     `producer_profile`, `product_id` or `product_version`) needs the same sticky, durable marker
     as rule 1, but `PremiumConsumer::ingest` sets `AccountView.unsupported` in memory only:
     `release` or a restart drops it, and a later compatible pull re-proves cached `ACTIVE`
     evidence without reconciliation. The account fence cannot hold it either, since
     `game_premium_account_fence` requires an accepted evidence row and a first-ever response can
     be the unsupported one. A new migration adds the durable representation (§11 scope item 7).
- **`refresh_after` bound (§4).** The producer sets it within the snapshot's own authority
  interval; one outside fails as malformed (a failed pull, under the §3 backoff). The client also
  waits at least 60 seconds after a successful pull before the next scheduled one, so an
  authenticated producer cannot drive a request loop.
- **Security audit (§3.1; consumer contract §15).** Every semantic failure, a same-revision
  contradiction against the current or a retained historical revision and an unsupported or
  downgraded response, records a durable security audit row before the denial is visible. PREM-1a
  has none.
- **`PREMDEL0-RL-04`.** The evidence log is never deleted, so every accepted revision's
  fingerprint is kept for the fence's life, which satisfies "at least 30 days". That is the
  recorded horizon: PREM-1a claims equivocation detection for every retained revision, and §6's
  "past the horizon" case does not arise. The withdrawn-entitlement check (§10.1) also reads this
  log, so any later pruning needs its own accepted decision that keeps both properties; no child
  here may prune it.
- **The failed-pull class (§3.1) is PREM-1b's.** PREM-1a has no pull, so it has no failed pull:
  it classifies only the kept evidence. PREM-1b adds the per-account unavailable state (§11).
- **`PROD-ENTITLEMENTS-01`** is `ACCEPTED / LIFECYCLE_CLOSED / NOT_STARTED` (PR #20, closeout
  #27, `docs/architecture/README.md`); its file keeps a historical candidate header. PREM-1a
  starts its Game consumer; it stays `NOT_STARTED` for activation until §10.3.
- **PREMIUM-ACTIVATION-V1** is consumed as recorded: every surface `REQUIRE_CURRENT` (§4.1),
  relocation reads only `premium_entitlement_ended` (§4.5). The §4.1, §4.5 and §5 amendments in
  the header take effect when this document's Game side is accepted, with no further edit to that
  document; the WHEEL-0 amendment of the brief stays pending on WHEEL-0 itself.

### 10.3 What is still open

- **PREM-P** (Platform's coordinator; this repository writes nothing there) accepts or amends §3,
  §3.1, §4 and §5. A change of path, form or value updates this document before activation.
  **Done (2026-10-04):** PREM-P is merged on Platform `main`, and §12 reconciles it. The Game
  half of the cross-repository end-to-end test exists (§12.4). The real-endpoint run and the
  §6.6 evidence are in the candidate activation record (§12.5).
- **Activation** of Premium for players needs PREM-1b merged, PREM-P live, the
  cross-repository end-to-end test (§3.1) passing, and the rollout evidence that
  `PROD-ENTITLEMENTS-01` §6.6 requires: the exact Platform producer and Game consumer revisions,
  the entitlement contract and profile revision (the §4 compatibility pair), the mixed-version
  compatibility rules, the rollout classification (producer first, consumer first or atomic), the
  rollback order, the treatment of evidence issued before a rollback, and the deterministic
  failure when one side does not understand the validity semantics, with no fail-open that
  restores unbounded stale authority. PREM-1's activation record carries all of it and only then
  names the switch-over date (§6). Until then `premium_current` is false for every account.

## 11. PREM-1b packet (for the control plane)

```yaml
task_id: OTV2-2026100x-prem-1b
title: "PREM-1b Premium snapshot client, refresh and test producer"
mode: IMPLEMENT
worker: oteryn-hard-worker   # authority: Premium evidence ingestion, a production network client
repository: Oteryn/Oteryn-Game
issue: 162
lane_id: premium
depends_on: ["PREMIUM-DELIVERY-0 Game-side acceptance", "PREM-1a (#1391, merged)"]
cross_repository_coordination_id: OTV2-PREMIUM-DELIVERY
external_repositories: []
leases: one migration number from the control plane at allocation (the semantic conflict and audit tables, scope item 7)
owned_paths:
  - apps/game-server/src/premium/            # new client.rs, refresh.rs, test_producer.rs; mod.rs; snapshot.rs, tests.rs (§10.2)
  - apps/game-server/migrations/<lease>_premium_semantic_conflict.sql
  - apps/game-server/src/durability/premium_fence.rs            # fingerprint, conflict and audit rows (§10.2)
  - apps/game-server/tests/support/premium_fence_postgres_cases.rs
  - apps/game-server/src/lib.rs              # module wiring only
  - apps/game-server/Cargo.toml              # reqwest (workspace, rustls) only
  - Cargo.lock
  - apps/game-server/tests/premium_snapshot_client.rs
  - the admission and reconnect call sites that load Premium (one pull before the first Premium
    read; the worker names the exact files in its record before writing them)
  - docs/agents/tasks/archive/OTV2-2026100x-prem-1b.md
```

**Scope.**

1. **Client (§3, §3.1).** `POST /v1/premium/snapshot` with the exact request body (at most 256
   bytes, `PREMDEL0-RL-02`); mutual TLS with a configured client identity and Platform CA (both
   from environment configuration; none committed); redirects off; 5-second total timeout
   (`PREMDEL0-RL-03`); body read capped at 1,024 bytes (`PREMDEL0-RL-01`) before parsing; only a
   200 with `application/json` is passed to `PremiumConsumer::ingest`. HTTPS only. No
   configuration means no client and Premium reads Free; login is never affected.
2. **Nonce.** 128 bits from the operating-system CSPRNG, 32 lowercase hexadecimal characters,
   fresh per request, never reused across retries.
3. **Failed-pull state.** A failed pull (§3.1 "anything else is unavailable", including a `Stale`
   ingest) marks the account `AUTHORITY_UNAVAILABLE` at once; it clears only on a successful
   ingest (`Accepted` or `Replayed`) of a pull started after the latest failure (reuse PREM-1a's
   ingest tickets). `Conflict`, `Unsupported`, `REVOKED`, `EXPIRED` and `NOT_YET_EFFECTIVE` still
   win over it. It never changes `premium_entitlement_ended`. In memory only: after a restart the
   re-proof pull (§9 item 3) is required anyway. The account also keeps the ticket of its latest
   proof: a failed pull or fence quarantine from an ingest whose ticket is older than that proof
   changes nothing, whatever order the ingests finish in (the inverse of the rule above; review
   4153900550 on #1391). A semantic failure (§3.1, scope item 7) is durable and is never ignored.
4. **Scheduling.** A pull at fresh admission and reconnect before any Premium read; then one at
   each snapshot's `refresh_after`, and never sooner than 60 seconds after the last successful
   pull, while the account is online. At most one request in flight
   per account. Retry with capped exponential backoff and jitter; a 429 or 503 `Retry-After` is
   honoured within the cap. `release` cancels the account's schedule.
5. **Test producer.** An in-process mutual-TLS server speaking exactly §3.1 and §4, scriptable
   per request (status, body, delay, wrong nonce or account), used by PREM-1b's tests and by the
   Game half of the end-to-end test. Test-only: not reachable from the production binary.
6. **PREM-1a fixes (§10.2).** The decoder accepts the canonical `NONE` (null `effective_from` and
   `effective_until`, null `entitlement_id`, `lifecycle_revision` 0) and still rejects a null
   interval in any other state; the evidence fingerprint covers `refresh_after`. No migration:
   the fingerprint column is unchanged, and no production evidence exists to re-fingerprint.
   A `refresh_after` outside `[authority_issued_at, authority_valid_until]` is `Malformed`.
7. **Durable semantic conflict and audit (§10.2).** Migration `<lease>` adds
   `game_premium_account_conflict` (one row per account, keyed by account alone so it needs no
   evidence row: kind `CONTRADICTION` or `UNSUPPORTED`, the response's `authority_revision`,
   `schema`, `producer_profile`, `product_id` and `product_version`, database time; no update
   and no delete) and `game_premium_security_audit` (append-only, unique per account, kind and
   `authority_revision`, bounded columns without the payload or any credential; no update and no
   delete). Every §3.1 semantic failure writes both in one transaction before the outcome
   returns; a fence write failure quarantines as today. `Unsupported` is classified only after
   the whole envelope passes every baseline type and value-form check (§3.1 item 2): today
   `snapshot::validate` returns it before the strict parse, and PREM-1b moves the compatibility
   comparison to the very end of `validate`, after the typed parse and every enumeration,
   timestamp, token, closed-`NONE` and structural interval check. The `max_authority_lease` check
   stays `Unsupported` (as `snapshot.rs` does today) and runs at that same end point. Any conflict
   row denies Premium, and
   the restart re-proof (§9 item 3) loads it before any benefit, alongside the existing
   `conflict_authority_revision`. No path clears it (§3.1 declared deferral).

**Required tests** (fail closed in each case): wrong nonce or account; oversize, malformed or
wrong content type; redirect; timeout; TLS failure and an untrusted server; a 500 or 429 with
`Retry-After`; a stale response; failure while cached `ACTIVE` is in its interval reads
`AUTHORITY_UNAVAILABLE` and a later success restores `CURRENT_AUTHORITY`; an older pull finishing
after a newer failure does not restore it, and an older failed pull or quarantine finishing after
a newer proof does not deny it; an incompatible `schema` or profile with a missing or mistyped
baseline field, or with a malformed value form (at least an unknown `schema` with a malformed
RFC 3339 timestamp), is a recoverable failed pull that leaves no conflict or audit row, while an
otherwise well-formed response whose lease exceeds `max_authority_lease` stays `Unsupported` with
its conflict and audit rows; a conflict
stays denied after successful pulls; one
request in flight per account; admission does not wait on or fail from a pull; the exact
producer-form `NONE` is accepted as Free and a `NONE` with any non-null interval, entitlement id or
non-zero `lifecycle_revision` fails closed; a changed `refresh_after` at an accepted
`authority_revision` sets the conflict marker (replacing PREM-1a's replay assertion); a
first-ever unsupported response, and one after accepted `ACTIVE` evidence, stays denied after
`release`, a restart and a later compatible pull; a `refresh_after` before issue or after the
lease fails closed, and a valid past one does not schedule a pull within 60 seconds; current and
historical-revision contradictions and an unsupported response each leave exactly one audit row
(a repeat adds none).

**Validation.** `cargo fmt --all --check`; `cargo clippy --locked -p oteryn-game-server
--all-targets -- -D warnings`; `cargo test --locked -p oteryn-game-server` with PostgreSQL; the
governance and repository policy validators. Independent security review on the frozen head.

**Out of scope.** Any Platform change; PREM-2..5 benefits; reconciliation of a conflict marker
(§3.1 declared deferral); pruning the evidence log (§10.2); activation (§10.3).

## 12. Reconciliation with PREM-P (2026-10-04, PREM-E2E-1)

PREM-P is merged on Platform `main`: the contract
`docs/contracts/OTERYN_V2_PREMIUM_TIME_SNAPSHOT_CONTRACT.md` (Platform #1432) and the producer
under `app/ProductsEntitlements/` (Platform #1433). Platform Issue #1431 is still open. This
section reads that contract at Platform commit `71bbe6c5cffc29d430195fa286b3c6941af4abb8`, the
same commit the vendored fixtures are pinned to (`tests/fixtures/premium-snapshot-v1/PROVENANCE.md`).
Platform owns those values (§5, consumer contract §5). Game adopts each one below and records
where it stays stricter. Owner direction: D540 ("unblock Premium end to end").

### 12.1 Transport (amends §3 and §3.1)

| Item | §3 / §3.1 | PREM-P | Game |
|---|---|---|---|
| Path | `POST /v1/premium/snapshot` | `POST /internal/v1/products-entitlements/premium-snapshots/read` on Platform's private internal route group (§4.1) | `client::SNAPSHOT_PATH` is the PREM-P path |
| mutual TLS | Platform-issued service identity for this one read | a trusted terminator requires `TLSv1.3` and a verified client subject equal to a dedicated configured identity (§4.2) | the client requires TLS 1.3 at least. Issuing the certificate, configuring the subject and trust anchor are deployment tasks, out of scope here |
| Request | `schema`, `account_id`, `nonce`, at most 256 bytes | the same three members. `account_id` must be a lowercase UUIDv7 and anything else is a `400` (§4.4) | no change: `AccountId` is a UUIDv7 (`domain::AccountId`, `strong_uuid_v7!`) |
| Unknown account | no separate answer. A never-granted account is a `200` `NONE` | `404` when no Platform Identity has the `account_id`. A known account without an entitlement is a `200` `NONE` (§4.5) | **amended:** a `404` is a non-200, so it is a failed pull (`AUTHORITY_UNAVAILABLE`). It is never Free by `NONE`. The code already did this |
| Other failures | any non-200 is unavailable | `400`, `401`, `429`, `503`, all with empty bodies. `503` also covers the disabled endpoint (`PRODUCTS_ENTITLEMENTS_PREMIUM_SNAPSHOT_ENABLED` not `true`) and a missing build revision (§4.3, §4.5) | no change: each one is a failed pull and denies Premium. Login is unaffected. `Retry-After` is honoured when present |
| Rate | at most one request in flight per account | a per-peer limiter of 1,200 requests a minute by default (§4.5) | one scheduled pull per online account about every 35 minutes (§12.2). That is about 34 requests a minute per 1,000 online accounts, retries excluded |

### 12.2 Policy (amends §3 and §5)

- `max_authority_lease` 3,600 seconds, `max_clock_skew` 5 seconds and no stale use: accepted by
  PREM-P as requested (§3). Game's constants are unchanged.
- **`refresh_after`** is `authority_issued_at + floor(2 × lease / 3)` seconds, not a fixed issue +
  40 minutes. It equals 40 minutes for a full lease and stays before a cutoff that the commercial
  end clips (PREM-P §3, §5.2).
- **Stale use is denied from `refresh_after`** (PREM-P §3, §8.2). Without newer accepted evidence,
  kept evidence is `STALE_WITHIN_BOUND` from `refresh_after` and `EXPIRED` from
  `authority_valid_until`, and both deny benefit. This replaces §3's "it never affects the class"
  and §5's row "producer refresh point: `authority_valid_until`". It is more restrictive than
  either. Game adds `PremiumClass::StaleWithinBound`. Any account reads it once the upper bound
  of trusted time reaches `refresh_after`, and `premium_current` is then false. A failed pull
  still denies at once (§3.1), and `premium_entitlement_ended` does not change.
- **Scheduling** (amends §11 scope item 4 and §10.2). PREM-P §8.5 asks Game to pull early enough
  that newer evidence is accepted before `refresh_after`. The refresher therefore pulls
  `REFRESH_LEAD` (5 minutes) before `refresh_after`, never sooner than 60 seconds after the last
  successful pull. Several capped retries (`RETRY_CAP` 60 seconds) fit before benefit stops.

### 12.3 Message (amends §4)

- The field set, names and types are unchanged. `producer_profile` is the constant
  `oteryn.entitlement.profile_b.v1`. The `NONE` variant has nulls exactly as §4 says.
- **Platform's v1 rules** (PREM-P §5.1, §5.2, §5.3 and the fixture manifest's
  `cross_field_rules`) are now checked by `snapshot::validate`, after the compatibility
  comparison. They are:
  - whole-second `YYYY-MM-DDTHH:MM:SSZ` timestamps;
  - `producer_revision` as 40 lowercase hexadecimal characters;
  - `entitlement_id` as a lowercase RFC 9562 UUIDv7;
  - `authority_revision` from 1 through 2^53 − 1, and `lifecycle_revision` at most 2^53 − 1;
  - `authority_valid_until` equal to `min(issued + 3600 s, effective_until)` for `ACTIVE` and
    `NOT_YET_EFFECTIVE`, and to `issued + 3600 s` otherwise;
  - the exact `refresh_after`, strictly before the cutoff;
  - a state that matches the interval at issue (`ACTIVE`: `effective_from <= issued <
    effective_until`; `NOT_YET_EFFECTIVE`: `issued < effective_from`; `EXPIRED`:
    `issued >= effective_until`);
  - `effective_from < effective_until`.

  A supported envelope that breaks one of these is malformed, a failed pull. It is never the
  durable marker.
- **Order** (keeps §3.1 item 2): form checks, then the compatibility pair and the lease bound
  (`Unsupported`), then the v1 rules (`Malformed`). The v1 rules belong to the product version,
  so they never decide whether an unknown schema or profile is durable.
- **Stricter than PREM-P, kept.** PREM-P §8.1 treats every refused body as "no fresh authority".
  Game also makes some refusals durable. A well-formed envelope outside the compatibility pair,
  or with a lease above 3,600 seconds, is a durable `INVALID_OR_CONFLICTING` (§3.1). Among
  Platform's invalid fixtures, Game classifies these five as `Unsupported`: "wrong schema id",
  "wrong producer_profile", "wrong product", "wrong product version" and "lease longer than
  3600 s". It classifies the other fifteen as `Malformed`. Both outcomes deny.
- **Lifecycle model, compatible with §10.1.** Platform keeps one entitlement per account, with a
  stable `entitlement_id` through expiry, revocation and re-grant. `lifecycle_revision` rises by
  one on each grant or revocation and never on expiry. Within one lifecycle revision, only
  `ACTIVE` to `EXPIRED` happens, by time (PREM-P §2, §6). In Game a higher `lifecycle_revision`
  always advances the entitlement fence (extend, revoke, re-grant), and within one revision
  `ACTIVE` to `EXPIRED` is reached by time, so both orders are accepted. PREM-P §8.3 rejects a
  lower lifecycle revision. Game records it as a conflict, which is stricter, and keeps that.
- **Platform database restore** (PREM-P §6.5). Revisions at or below Game's high water are stale,
  so Premium fails closed until Platform issues higher revisions. No Game change.
- **PREM-P §9.1** names one possible conflict: treating `ACTIVE` alone as enough. Game never did:
  it derives the class from the absolute times and trusted time (§4, `premium::classify`).

### 12.4 Cross-repository end-to-end test, Game half (§3.1, §10.3)

- Platform's fixtures are vendored unmodified in `tests/fixtures/premium-snapshot-v1/`, pinned to
  the commit above, with SHA-256 hashes the test checks. Running the PHP producer in Game CI was
  not cheap: Game CI has no PHP toolchain or Platform database. PREM-P §7 offers the vendored
  copy.
- `apps/game-server/tests/premium_platform_fixtures.rs` checks:
  - the pins;
  - both compatibility records (the manifest policy values and the schema constants against
    Game's);
  - that Game's request is the fixture request, on the PREM-P path;
  - that every valid fixture is accepted as its state;
  - that every invalid fixture is refused with the outcome in §12.3. A new Platform case fails
    the test until it has a Game outcome;
  - that every valid fixture passes through the production client over mutual TLS 1.3 to the
    test producer.
- `premium::tests::platform_fixtures_classify_as_the_contract_requires` proves the manifest's
  expected classification of each valid fixture. `ACTIVE` is current until `refresh_after`, then
  stale until the cutoff, then expired, with the clipped lease included. `EXPIRED`, `REVOKED`
  and `NONE` always deny.
- PREM-P §10 ends with "followed by a real endpoint run". That run needs a deployed producer and
  a Platform-issued client certificate, so it is activation evidence (§12.5), not CI.

### 12.5 Activation

The candidate activation record is
`OTERYN_GAME_PREMIUM_ACTIVATION_RECORD_CANDIDATE_2026-10-04.md`. It carries the §10.3 and
`PROD-ENTITLEMENTS-01` §6.6 fields and lists what is still open. It names no switch-over date.
This task changes no configuration: no snapshot client is configured in production, so every
account reads Free.
