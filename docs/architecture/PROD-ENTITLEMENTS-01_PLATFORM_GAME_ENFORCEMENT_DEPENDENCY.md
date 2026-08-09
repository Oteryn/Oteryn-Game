# PROD-ENTITLEMENTS-01 — Platform Entitlement / Game Enforcement Security Dependency

- Status: Registered mandatory security dependency; not the final `PROD-ENTITLEMENTS-01` contract
- Date: 2026-08-09
- Gate: `PROD-ENTITLEMENTS-01`
- Coordination ID: `OTV2-PROD-ENTITLEMENTS`
- Repository: `blakinio/Oteryn-v2`
- Oteryn-v2 issue: #115
- External producer evidence: `blakinio/Oteryn-Platform` PR #925, merge `b1e5957614b29e88825ba74425e979be9b6bd070`
- External security finding: `blakinio/Oteryn-Platform#944`, `OPA-SEC-0007`, P1/high risk
- Does not authorize: payment/entitlement runtime, Premium/VIP activation, game-server implementation, persistence schema, protocol implementation, FND-04 changes, Platform writes, deployment or production traffic

## 1. Purpose

`PROD-ENTITLEMENTS-01` already exists in the Oteryn-v2 global architecture horizon as the deferred Entitlements, Premium and Commerce Boundary.

Oteryn Platform has since accepted a native entitlement/game-delivery authority split and then independently found a material security gap in its Profile-B game-consumed entitlement contract. Oteryn-v2 must retain that finding as a mandatory consumer-side prerequisite so a future game implementation cannot accidentally turn stale Platform entitlement state into unbounded gameplay authority.

This document records the dependency only. It does not freeze the final entitlement wire schema, storage, product catalogue, Premium/VIP behavior or session-disconnect policy.

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
- a refund, chargeback, expiry or revocation does not authorize arbitrary direct mutation of game state;
- Platform does not become authoritative gameplay state merely because it owns commercial entitlement lifecycle;
- Oteryn-v2 does not become payment/order authority merely because it enforces a game-consumed entitlement.

This separation is consistent with ADR-0003, ADR-0004, ADR-0012 and the existing `PROD-ENTITLEMENTS-01` horizon.

## 3. Proven external security gap

Oteryn Platform Issue #944 proves that its accepted Profile-B game-consumed entitlement contract currently lacks an implementable finite stale-authority bound.

Representative unsafe sequence:

```text
Platform entitlement revision R10 = active
-> Oteryn-v2 accepts/caches R10
-> Platform entitlement authority becomes unavailable
-> a later Platform expiry/revoke R11 cannot be observed
-> R10 has no mandatory valid_until / lease expiry / finite max_stale / refresh deadline
-> game consumer has no contract-defined cutoff for the old allow decision
```

The defect is architectural even if Premium/VIP is not currently activated: without a finite authority bound, `stale` can become an indefinite allow state during a partition.

`OPA-SEC-0007` is therefore a mandatory prerequisite for any future Profile-B or equivalent game-consumed entitlement implementation.

## 4. Activation blocker

The following is forbidden until the Platform-side contract is repaired and the Oteryn-v2 consumer contract is accepted:

- enabling Premium/VIP gameplay effects sourced from Platform entitlement state;
- enabling any Profile-B game-consumed account entitlement;
- treating cached `active` entitlement state as indefinitely valid while Platform authority is unavailable;
- implementing a product-specific grace policy with no explicit finite upper bound;
- allowing client or GameNode local clocks to extend producer-owned commercial authority;
- allowing delayed/out-of-order old `active` evidence to resurrect authority after a newer expiry/revoke decision;
- calling transport success/failure itself proof of entitlement truth.

This blocker is scoped to entitlement implementation/activation. It does not block unrelated foundation/runtime architecture work.

## 5. Required producer-side prerequisite

Before Oteryn-v2 freezes the final consumer contract, `blakinio/Oteryn-Platform#944` must have an exact merged remediation revision or an explicit superseding accepted Platform decision with equivalent or stronger guarantees.

The Platform-side evidence must provide an implementable finite authority-validity mechanism, for example one of:

- producer-authoritative `valid_until`;
- entitlement authority lease expiry;
- finite product/version-specific `max_stale` plus authenticated observation time/revision semantics;
- mandatory refresh deadline;
- another reviewed mechanism that proves the same finite upper bound.

Oteryn-v2 must pin the exact merged Platform producer revision. A mutable PR head is not canonical cross-repository authority.

## 6. Required Oteryn-v2 consumer contract outcomes

The later `PROD-ENTITLEMENTS-01` contract must define, at minimum:

### 6.1 Evidence state

Game-side enforcement must distinguish enough typed state to represent the semantic equivalents of:

```text
CURRENT
STALE_WITHIN_BOUND
AUTHORITY_UNAVAILABLE
EXPIRED
REVOKED
INVALID_OR_CONFLICTING
```

Names may differ. The semantics may not collapse `stale` or `unavailable` into an unbounded allow state.

### 6.2 Finite authority lifetime

Every game-consumed product/version must declare a finite authority-validity/offline policy before activation.

There is no implicit or implementation-defined infinite grace.

The contract must state:

- producer authority for validity/expiry;
- maximum accepted stale interval or equivalent finite cutoff;
- clock/skew semantics;
- whether the validity bound is absolute, revision-relative, lease-based or another reviewed form;
- what happens when fresh producer authority cannot be obtained before the bound expires.

### 6.3 Ordering and anti-rollback

A newer restrictive decision must dominate older allow evidence.

At minimum:

```text
newer expiry/revoke/disable revision
> older active revision
```

and the older representation cannot regain authority through:

- delayed delivery;
- out-of-order refresh;
- cache replay;
- GameNode restart;
- reconnect;
- projection rollback;
- replica failover;
- clock skew.

Equal revision with conflicting authenticated content must fail closed or follow another explicitly reviewed non-ambiguous rule.

### 6.4 New admission, reconnect and existing sessions

The entitlement gate must keep these decisions distinct:

- whether a new gameplay admission may consume entitlement-derived capability;
- whether reconnect/recovery may continue using entitlement-derived benefit;
- whether an already-running session may temporarily retain a benefit during an outage;
- whether expiry/revocation requires immediate effect, bounded delayed effect, safe-point transition, reauthentication or another product-specific policy.

FND-04 owns gameplay admission/session/reconnect authority. `PROD-ENTITLEMENTS-01` may consume those semantics but must not redefine GameSession, lease or reconnect ownership.

Deferring forced disconnect policy does not permit entitlement benefit to survive beyond the declared finite authority bound. A session/reconnect contract may define how authority loss is applied operationally, but it cannot extend or recreate Platform-owned commercial authority beyond that producer-grounded bound.

### 6.5 Game-affecting delivery

Durable grants/services must use stable operation identity, idempotent outcome and reconciliation semantics.

An ambiguous delivery result must never be retried with a fresh operation identity merely because transport failed. A payment/entitlement retry must not duplicate an already committed game mutation.

Character-service products continue to route the actual character mutation through Character Authority rather than direct Platform SQL.

### 6.6 Rollout and rollback

Before activation, cross-repository evidence must name:

- exact Platform producer revision;
- exact Oteryn-v2 consumer revision;
- entitlement contract/profile revision;
- mixed-version compatibility rules;
- producer-first/client-first/atomic rollout classification;
- rollback order;
- treatment of evidence issued before rollback;
- deterministic failure behavior when one side does not understand the required validity semantics.

Fail-open compatibility that restores unbounded stale authority is forbidden.

## 7. Mandatory validation scenarios

The future implementation/contract test catalogue must cover at least:

1. Platform available, current active entitlement within validity bound;
2. Platform outage before the accepted stale bound expires;
3. Platform outage after the accepted stale bound expires;
4. entitlement expires while Platform is unreachable;
5. newer revoke arrives after cached active state;
6. delayed old active revision arrives after newer revoke;
7. reconnect with stale-but-within-bound evidence;
8. reconnect after authority bound expiry;
9. GameNode restart with cached active evidence;
10. projection/cache rollback to an older active snapshot;
11. out-of-order lifecycle revisions;
12. equal revision with contradictory authenticated state;
13. local clock skew attempting to extend validity;
14. ambiguous durable game delivery followed by retry;
15. producer/consumer version mismatch during rollout and rollback.

Tests must prove both safety and bounded availability behavior; a fail-closed design still needs deterministic user-visible and operational outcomes.

## 8. Relationship to FND-04A/B/C

### FND-04A

No change is required to the current FND-04A fresh-admission contract merely because Platform #944 exists.

FND-04A covers fresh gameplay admission authority and Platform security/trust evidence, not commercial entitlement enforcement. Entitlement state must not be smuggled into PreAdmissionGrant authority unless a later accepted cross-repository contract proves a specific need.

### FND-04B/C

FND-04B/C may expose reconnect/session lifecycle hooks that `PROD-ENTITLEMENTS-01` later consumes. They must not silently decide Premium/VIP grace, commercial expiry or entitlement cache policy on behalf of the product gate.

If future evidence shows entitlement state is required at a session boundary, the dependency must be explicit and preserve the authority split.

## 9. Current disposition

```yaml
gate: PROD-ENTITLEMENTS-01
canonical_status: DEFERRED
consumer_security_dependency: REGISTERED
platform_authority_split_source: blakinio/Oteryn-Platform@b1e5957614b29e88825ba74425e979be9b6bd070
platform_security_finding: blakinio/Oteryn-Platform#944
platform_security_finding_state_at_registration: OPEN
platform_remediation_commit: UNKNOWN
runtime_implementation: NOT_AUTHORIZED
premium_vip_activation: NOT_AUTHORIZED
blocks_fnd04a: false
blocks_game_consumed_entitlement_activation: true
```

The gate remains deferred for unrelated foundation work. Once entitlement implementation becomes relevant, this dependency is mandatory and cannot be waived by treating the current Platform prose as sufficient evidence.

## 10. Acceptance invariant

Future work complies with this dependency when:

> No game-consumed commercial entitlement can remain authoritative indefinitely from stale cached Platform allow state. Every activated product has a finite, producer-grounded authority-validity rule; newer restrictive lifecycle decisions fence older allows; Oteryn-v2 enforces those rules without becoming payment authority; and exact producer/consumer revisions plus failure/rollback tests prove the boundary before activation.
