# PROD-ENTITLEMENTS-01 — Platform Entitlement / Game Enforcement Security Dependency

- Status: Registered mandatory security dependency; producer prerequisite satisfied; final Oteryn-v2 consumer contract still pending
- Date: 2026-08-11
- Gate: `PROD-ENTITLEMENTS-01`
- Coordination ID: `OTV2-PROD-ENTITLEMENTS`
- Repository: `blakinio/Oteryn-v2`
- Oteryn-v2 issue: #115
- Platform authority-split evidence: `blakinio/Oteryn-Platform` PR #925, merge `b1e5957614b29e88825ba74425e979be9b6bd070`
- Historical Platform security finding: `blakinio/Oteryn-Platform#944`, `OPA-SEC-0007`, P1/high risk, closed `completed`
- Platform remediation evidence: PR #968, exact final head `27414684ceb77700c7bbf7c6a047c6f3c0c79ad9`, merge `afaa6d1d8340e44b1152b62d6d27e5fd1649804a`
- Producer contract: `blakinio/Oteryn-Platform/docs/contracts/OTERYN_V2_ENTITLEMENT_GAME_DELIVERY_CONTRACT.md` at merge `afaa6d1d8340e44b1152b62d6d27e5fd1649804a`
- Does not authorize: payment/entitlement runtime, Premium/VIP activation, game-server implementation, persistence schema, protocol implementation, FND-04 changes, Platform writes, deployment or production traffic

## 1. Purpose

`PROD-ENTITLEMENTS-01` is the deferred Entitlements, Premium and Commerce Boundary for Oteryn-v2.

Oteryn Platform owns commercial entitlement lifecycle while Oteryn-v2 will own authoritative gameplay application/enforcement of game-affecting entitlement value. Platform security audit finding `OPA-SEC-0007` proved that the earlier Profile-B producer contract did not provide an implementable finite stale-authority bound. Platform PR #968 repaired that producer contract.

This document now records two separate facts:

1. the **producer-side finite-authority prerequisite is satisfied** by exact merged Platform evidence; and
2. the **Oteryn-v2 consumer/enforcement contract is still not accepted or implemented**, so no game-consumed commercial entitlement may be activated yet.

This document does not freeze the final Oteryn-v2 entitlement wire schema, storage, product catalogue, Premium/VIP behavior or session-disconnect policy.

## 2. Accepted authority separation

The native target preserves three independent truth domains:

```text
payment/order truth
    -> Platform Payments / provider reconciliation

Platform entitlement truth
    -> Platform ProductsEntitlements lifecycle and revision authority

Oteryn-v2 gameplay delivery/enforcement truth
    -> authoritative game-domain application/enforcement of game-affecting value
```

Consequences:

- payment success is not proof that gameplay delivery committed;
- an active Platform entitlement is not proof that the corresponding game effect has already been applied;
- a game receipt does not redefine payment settlement truth;
- refund, chargeback, expiry or revocation does not authorize arbitrary direct mutation of game state;
- Platform does not become authoritative gameplay state merely because it owns commercial entitlement lifecycle;
- Oteryn-v2 does not become payment/order authority merely because it enforces a game-consumed entitlement.

This separation remains consistent with ADR-0003, ADR-0004, ADR-0012 and the existing `PROD-ENTITLEMENTS-01` horizon.

## 3. Historical security gap and proven producer remediation

### 3.1 Historical defect

Issue `blakinio/Oteryn-Platform#944` proved the unsafe sequence below for the pre-repair Profile-B producer contract:

```text
Platform entitlement revision R10 = active
-> Oteryn-v2 accepts/caches R10
-> Platform entitlement authority becomes unavailable
-> a later expiry/revoke R11 cannot be observed
-> R10 has no mandatory finite authority-validity cutoff
-> game consumer cannot determine when the old allow must stop
```

That defect made an unbounded stale allow state architecturally possible during a partition.

### 3.2 Remediation

Platform PR #968 is merged as `afaa6d1d8340e44b1152b62d6d27e5fd1649804a` and closes #944 as completed. Its accepted Profile-B contract requires, among other security properties:

- producer-issued `effective_from` / `effective_until` and finite `authority_valid_until` semantics;
- finite per-product/version authority/offline policy rather than implicit infinite grace;
- distinguishable current, stale-within-bound, unavailable, expired, revoked and not-yet-effective states/equivalents;
- lifecycle and authority-revision high-water fencing against delayed/out-of-order delivery, cache replay, reconnect, restart and rollback;
- bounded clock/skew treatment and fail-closed behavior when trusted time is unsafe unless an equivalently bounded accepted authority anchor exists;
- explicit separation of admission/reconnect decisions from running-session mechanics while preventing entitlement benefit outside the provable authority interval;
- rollout/rollback and negative-path validation obligations.

Therefore the original **producer-side** blocker is satisfied. Oteryn-v2 must consume the exact merged producer semantics rather than copying or redefining Platform commercial authority.

## 4. Remaining activation blocker

The following remains forbidden until the bounded Oteryn-v2 consumer/enforcement contract is accepted and its implementation is separately proven:

- enabling Premium/VIP gameplay effects sourced from Platform entitlement state;
- enabling any Profile-B or equivalent game-consumed account entitlement;
- treating cached `active` entitlement state as valid beyond the producer-grounded finite authority interval;
- implementing product-specific grace with no explicit finite upper bound;
- allowing client or GameNode local clocks to extend producer-owned commercial authority;
- allowing delayed/out-of-order old `active` evidence to resurrect authority after newer expiry/revoke evidence;
- treating transport success/failure itself as entitlement truth;
- retrying an ambiguous durable game delivery with a fresh operation identity in a way that may duplicate a committed mutation.

This blocker is scoped to entitlement implementation/activation. It does not block unrelated foundation/runtime architecture work.

## 5. Producer-side prerequisite — SATISFIED

Canonical producer evidence is pinned to:

```yaml
repository: blakinio/Oteryn-Platform
finding: OPA-SEC-0007
issue: 944
issue_state: CLOSED_COMPLETED
repair_pr: 968
repair_final_head: 27414684ceb77700c7bbf7c6a047c6f3c0c79ad9
repair_merge: afaa6d1d8340e44b1152b62d6d27e5fd1649804a
producer_contract: docs/contracts/OTERYN_V2_ENTITLEMENT_GAME_DELIVERY_CONTRACT.md
```

A future Oteryn-v2 consumer contract must pin this merge or an explicitly reviewed later superseding Platform revision. Mutable PR heads are not canonical cross-repository authority.

The producer remediation does **not** itself authorize Oteryn-v2 entitlement runtime, Premium/VIP activation or game-side storage/protocol choices.

## 6. Required Oteryn-v2 consumer contract outcomes

### 6.1 Evidence state

Game-side enforcement must represent semantic equivalents of at least:

```text
CURRENT
STALE_WITHIN_BOUND
AUTHORITY_UNAVAILABLE
EXPIRED
REVOKED
INVALID_OR_CONFLICTING
```

If the producer contract exposes a distinct `NOT_YET_EFFECTIVE` state/condition, the consumer must preserve its safety semantics rather than collapsing it into current authority.

Names may differ. `stale` or `unavailable` may never become an unbounded allow state.

### 6.2 Finite authority lifetime

Every activated game-consumed product/version must have a finite producer-grounded authority/offline policy.

The consumer contract must state:

- producer authority for effective interval and expiry;
- how `authority_valid_until` or an explicitly superseding bounded producer mechanism is evaluated;
- any stricter product-specific maximum stale/offline interval;
- clock/skew or accepted monotonic/authority-anchor semantics;
- deterministic behavior when fresh producer authority cannot be obtained before the accepted bound ends.

There is no implicit or implementation-defined infinite grace.

### 6.3 Ordering and anti-rollback

A newer restrictive lifecycle/authority decision must dominate older allow evidence. Older `active` evidence may not regain authority through:

- delayed delivery;
- out-of-order refresh;
- cache replay;
- GameNode restart;
- reconnect;
- projection/storage rollback;
- replica failover;
- clock skew.

Equal revision with conflicting authenticated content must fail closed or use another explicitly reviewed, non-ambiguous rule compatible with the producer contract.

### 6.4 New admission, reconnect and existing sessions

The game contract must distinguish:

- whether new gameplay admission may consume entitlement-derived capability;
- whether reconnect/recovery may continue entitlement-derived benefit;
- whether an already-running session may temporarily retain a benefit during an outage;
- how expiry/revocation becomes effective operationally, including any bounded delay, safe-point transition or reauthentication policy.

FND-04 owns gameplay admission/session/reconnect authority. `PROD-ENTITLEMENTS-01` consumes those semantics and must not redefine GameSession, CharacterLease or reconnect ownership.

No operational session policy may extend or recreate Platform-owned commercial authority beyond the accepted producer-grounded validity bound.

### 6.5 Game-affecting delivery

Durable entitlement grants/services must use stable operation identity, idempotent outcome and reconciliation semantics.

An ambiguous delivery result must not be retried with a fresh operation identity merely because transport failed. Payment/entitlement retries must not duplicate an already committed game mutation.

Character-service products route authoritative character mutation through Character Authority rather than direct Platform SQL.

### 6.6 Rollout and rollback

Before activation, cross-repository evidence must name:

- exact Platform producer revision;
- exact Oteryn-v2 consumer revision;
- entitlement contract/profile revision;
- mixed-version compatibility rules;
- producer-first, consumer-first or atomic rollout classification;
- rollback order;
- treatment of evidence issued before rollback;
- deterministic failure behavior when one side does not understand the required validity semantics.

Fail-open compatibility that restores unbounded stale authority is forbidden.

## 7. Mandatory validation scenarios

Future consumer contract/implementation evidence must cover at least:

1. current active entitlement within the producer validity bound;
2. Platform outage before the accepted stale/authority bound expires;
3. Platform outage after the accepted bound expires;
4. entitlement effective interval ends while Platform is unreachable;
5. newer revoke after cached active state;
6. delayed old active evidence after newer revoke;
7. reconnect with stale-but-within-bound evidence;
8. reconnect after authority-bound expiry;
9. GameNode restart with cached active evidence;
10. projection/cache rollback to older active evidence;
11. out-of-order lifecycle/authority revisions;
12. equal revision with contradictory authenticated state;
13. unsafe/local clock behavior attempting to extend validity;
14. not-yet-effective evidence under skew/uncertain time;
15. ambiguous durable game delivery followed by retry;
16. producer/consumer version mismatch during rollout and rollback.

Tests must prove safety and bounded availability behavior with deterministic player-visible and operator-visible outcomes.

## 8. Relationship to FND-04A/B/C

### FND-04A

The Platform producer repair does not require changing the accepted FND-04A fresh-admission contract. FND-04A owns gameplay admission trust/session authority, not commercial entitlement enforcement.

Entitlement state must not be smuggled into PreAdmissionGrant authority unless a later accepted cross-repository contract proves a specific need.

### FND-04B/C

FND-04B/C may expose reconnect/session lifecycle hooks consumed by `PROD-ENTITLEMENTS-01`. They must not silently decide Premium/VIP grace, commercial expiry or entitlement cache policy on behalf of the product gate.

If future evidence requires entitlement state at a session boundary, the dependency must be explicit and preserve the Platform-commercial/gameplay-enforcement authority split.

## 9. Current disposition

```yaml
gate: PROD-ENTITLEMENTS-01
canonical_status: DEFERRED
consumer_security_dependency: REGISTERED
platform_authority_split_source: blakinio/Oteryn-Platform@b1e5957614b29e88825ba74425e979be9b6bd070
platform_security_finding: blakinio/Oteryn-Platform#944
platform_security_finding_state: CLOSED_COMPLETED
platform_remediation_pr: 968
platform_remediation_final_head: 27414684ceb77700c7bbf7c6a047c6f3c0c79ad9
platform_remediation_commit: afaa6d1d8340e44b1152b62d6d27e5fd1649804a
producer_prerequisite: SATISFIED
oteryn_v2_consumer_contract: NOT_ACCEPTED
runtime_implementation: NOT_AUTHORIZED
premium_vip_activation: NOT_AUTHORIZED
blocks_fnd04a: false
blocks_game_consumed_entitlement_activation: true
```

The gate remains deferred for unrelated foundation work. When entitlement implementation becomes relevant, exact producer evidence is already available, but the bounded Oteryn-v2 consumer/enforcement contract, cross-repository rollout proof and runtime implementation evidence remain mandatory.

Oteryn-v2 issue #115 should remain open until its remaining consumer-side acceptance criteria are satisfied; the producer-remediation criterion can now be marked with exact evidence.

## 10. Acceptance invariant

Future work complies with this dependency when:

> No game-consumed commercial entitlement can remain authoritative beyond its finite producer-grounded validity. Newer restrictive lifecycle/authority decisions fence older allows; Oteryn-v2 enforces the accepted producer contract without becoming payment authority; game-affecting delivery is idempotent and reconcilable; and exact producer/consumer revisions plus failure/rollback tests prove the boundary before activation.
