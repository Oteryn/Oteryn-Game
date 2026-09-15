# FND-04 — Platform Pre-Admission Reconciliation Refinement

- Status: Architecture analysis refinement; binding input to the later final FND-04 contract when merged
- Date: 2026-08-08
- Gate: `FND-04`
- Coordination ID: `OTV2-NATIVE-FOUNDATION`
- Repository: `blakinio/Oteryn-v2`
- Refines: `FND-04_SESSION_ADMISSION_LEASE_ANALYSIS_BASELINE.md`
- Current external reconciliation pin: `blakinio/Oteryn-Platform@216f5b2817e9d102337608609e344518512c2a0d`
- External contracts consumed read-only:
  - `docs/contracts/OTERYN_V2_PRE_ADMISSION_HANDOFF_CONTRACT.md`
  - `docs/contracts/OTERYN_V2_RUNTIME_STATUS_PROJECTION_CONTRACT.md`
- Does not authorize: Platform writes, token/key implementation, runtime/protocol implementation, persistence schema, production key distribution, routing activation or production traffic

## 1. Why this refinement exists

The merged FND-04 analysis correctly separates Platform authorization from game-domain final admission, but its original external reconciliation source was historical.

A delayed exact-head review found that current accepted Oteryn Platform contracts contain three additional producer/consumer semantics that the final FND-04 contract must consume explicitly:

1. Platform account/security authority can change **after** PreAdmissionGrant issuance but **before** game admission;
2. Platform issuance is conditioned on fresh current-owner runtime evidence and preserves runtime observation / ownership-generation applicability;
3. Platform has an issuance-attempt correlation/idempotency lifecycle that is semantically different from game-domain grant consumption/replay state.

This refinement closes that analysis gap without redefining the accepted Oteryn-v2 authority split.

Where this document is more specific about those three topics, it supersedes the corresponding incomplete wording in `FND-04_SESSION_ADMISSION_LEASE_ANALYSIS_BASELINE.md`.

## 2. Authority remains unchanged

The current Platform contracts do **not** make Platform the final game admission authority.

The authority chain remains:

```text
Platform Identity / security policy
    -> authenticates account and owns account-security state

Game Gateway / Platform admission issuer
    -> authorizes one bounded attempt using current Platform policy
    -> uses fresh current Oteryn-v2 runtime observation for route selection

PreAdmissionGrant
    -> proves a bounded Platform-issued attempt capability
    -> not a GameSessionId
    -> not a CharacterLease
    -> not current gameplay owner proof

Oteryn-v2 final admission
    -> revalidates current AccountId -> CharacterId ownership
    -> revalidates current game route/runtime ownership applicability
    -> applies account-presence / character-lease exclusion
    -> consumes grant anti-replay state
    -> commits canonical GameSessionId + initial connection_generation only on success
```

Platform projections may prevent knowingly unsafe issuance. They never override newer authoritative game-domain facts.

## 3. Post-issuance account-security changes are a first-class race

### 3.1 The race

A grant may be validly issued at `T_issue`, then Platform account-security authority may change before game admission, for example:

- Identity/account disabled;
- compromise response begins;
- account security generation/revision advances;
- administrative emergency revocation is issued;
- a policy transition makes the prior authorization unsafe.

The final FND-04 contract must not assume:

```text
valid at issuance
+ signature still valid
+ nominal exp not reached
= necessarily still admissible
```

That implication is not universally safe.

### 3.2 Required final-contract property

FND-04 must freeze one explicit, testable **post-issuance security-change disposition** with a named maximum risk/staleness window.

The chosen design must establish what happens when security authority changes between issuance and final admission.

Acceptable mechanism families include:

1. **bounded short-lifetime risk window** — the grant is intentionally valid for a very short maximum interval and product/security accepts that revocation may take effect only by expiry;
2. **account-security generation binding + current validation** — grant carries the Platform security generation/revision accepted at issuance and the game admission boundary validates it against a sufficiently current trusted Platform security projection/introspection result;
3. **revocation projection / emergency deny channel** — game admission consumes a trusted bounded-staleness security-revocation projection capable of invalidating otherwise unexpired grants;
4. **online introspection** — final admission asks a trusted Platform security authority whether the grant/account security generation remains admissible;
5. another reviewed design that proves an equivalent bound.

A hybrid is allowed and likely desirable.

### 3.3 Analysis recommendation

Prefer a design that preserves the normal-path goal of avoiding an unnecessary synchronous Platform dependency **while still allowing bounded emergency account-security invalidation**.

The strongest current direction for final evaluation is:

```text
short-lived signed PreAdmissionGrant
+ Platform account-security generation/revision bound into the grant
+ trusted bounded-staleness game-side security-revocation/generation projection
+ fail closed for new admission when required security freshness cannot be established
+ optional online introspection only for exceptional/high-risk reconciliation if the final security review requires it
```

This is a semantic direction, not approval of a transport/database/cache implementation.

The final contract must choose the concrete security-freshness mechanism and its maximum staleness/TTL evidence before implementation.

### 3.4 Existing gameplay is separate

A Platform account-security change may affect policy for future admission/recovery, but this refinement does not itself authorize Platform to terminate an already-authoritative gameplay actor/session asynchronously.

Any emergency termination/revocation of an already admitted GameSession requires an explicit game-domain control contract and safe fencing path. Do not smuggle post-admission mutation authority into PreAdmissionGrant validation.

## 4. Account-security generation is not a gameplay identity

If the final contract uses a Platform account-security generation/revision, its semantics should be:

```text
scope: AccountId
owner: Platform Identity/security authority
purpose: prove which Platform security state authorized the bounded attempt
```

It is not:

- AccountId;
- GameSessionId;
- connection_generation;
- character_lease_generation;
- runtime scope ownership generation;
- an authorization by itself without trusted issuer/current-state validation.

It may be an opaque monotonic revision rather than a globally public foundation identifier.

No new Oteryn-v2 foundation entity ID is required merely to carry this producer-owned revision.

## 5. Runtime observation and ownership-generation applicability

### 5.1 Platform issuance-time evidence

Current Platform policy issues native pre-admission material only after it has accepted fresh Oteryn-v2 runtime evidence for the target `WorldId + ChannelId`.

That observation may contain or be correlated with:

- source/runtime observation identity or source revision;
- current runtime owner identity where appropriate;
- current scope ownership/fencing generation;
- runtime lifecycle/readiness;
- protocol/runtime/content compatibility revisions;
- freshness/expiry semantics;
- route/topology revision.

Platform may use these facts for **attempt routing/issuance** only.

### 5.2 Grant must preserve applicability evidence

The final cross-repository PreAdmissionGrant profile must preserve enough immutable issuance-time context to let Oteryn-v2 distinguish:

```text
capability issued for current owner/route/revision
vs
capability issued against an observation that has since been superseded
```

The exact field names/encoding remain later profile work, but semantics should include as applicable:

- `WorldId`;
- `ChannelId`;
- route/offer/topology generation or immutable route target revision;
- runtime observation/source revision;
- observed scope ownership generation when that generation is required to prevent stale-owner admission;
- required runtime/protocol/content compatibility revisions.

### 5.3 Recommended invalidation rule

For a **fresh-entry** grant, if authoritative Oteryn-v2 scope ownership generation for the bound target has advanced since the issuance observation, treat the grant as stale for final admission unless the final contract explicitly proves that the new owner can safely accept the old capability under an equivalent preserved route/security fence.

Default safe rule:

```text
observed_ownership_generation != current_authoritative_ownership_generation
-> reject fresh-entry grant as stale/superseded
-> require fresh Platform routing + fresh grant
```

This may cost a retry during failover, but it prevents an unexpired grant from becoming a bearer route to a replacement owner whose readiness/revisions were not the evidence used at issuance.

The final FND-04 contract may relax this only with explicit proof of equivalent current-owner admission safety.

### 5.4 Current game-domain validation remains mandatory

Even when the grant carries issuance-time runtime evidence:

- Oteryn-v2 validates that the receiving admission boundary is the current accepted owner/route for the target;
- NodeId alone never grants authority;
- Platform runtime observation never self-grants current game authority;
- a delayed/stale Platform observation cannot override a newer Oteryn-v2 ownership generation;
- wrong/superseded route/generation fails closed;
- no silent retarget to another channel/owner using the same fresh-entry capability.

### 5.5 Recovery remains different from fresh entry

The runtime-observation/ChannelId binding above is a **fresh-entry** semantic.

Existing-actor/session recovery still resolves the actor's current authoritative placement from game-domain state. A stale Platform fresh-entry grant cannot drag a present actor to its old ChannelId or out of an InstanceRuntime.

If Platform participates in a reauthenticated recovery flow, that credential must use a distinct recovery purpose and current game-domain route resolution as already required by the main FND-04 analysis.

## 6. Platform issuance-attempt identity is distinct from game consume nonce

### 6.1 Two different lifecycle objects

Current Platform architecture requires a producer-side admission-attempt correlation/idempotency reference.

The FND-04 analysis also requires a cryptographically random game-domain one-time grant nonce/replay key.

These solve different problems:

```text
AdmissionAttemptRef
owner: Platform producer/issuer operation lifecycle
purpose:
  - correlate one logical issuance attempt
  - distinguish retry from independent login attempt
  - reconcile ambiguous issuer response
  - security/audit trace

GrantNonce
owner/use: cross-boundary capability uniqueness + game-domain anti-replay consumption
purpose:
  - prevent one issued capability from producing multiple successful game admissions
  - key authoritative consume/replay state
```

They must not be silently collapsed into one concept.

### 6.2 No new foundation entity ID required

`AdmissionAttemptRef` is a semantic producer operation/correlation reference, not a canonical gameplay entity.

Current recommendation:

- do not add it to FND-ID foundation entity catalogue;
- define a bounded opaque representation in the Platform→Oteryn admission profile;
- make it unique/idempotency-safe within the issuer's declared retention/reconciliation scope;
- allow it to appear in authorized cross-boundary correlation/audit metadata;
- never accept it as admission authority, replay proof, GameSession identity or CharacterLease identity.

A future identifier ADR is required only if implementation/recovery evidence proves that this producer operation must become a durable cross-system entity with lifecycle beyond bounded issuance reconciliation.

### 6.3 Issuance idempotency requirement

If the Platform issuer receives a retry for the same logical `AdmissionAttemptRef` after an ambiguous response, it must not mint multiple independently usable grants.

The final producer contract must choose one bounded behavior such as:

- return/recover the same still-valid grant/capability outcome for that attempt; or
- return a stable committed/failed/unknown reconciliation result and require a new Game Login Ticket/new attempt before minting another independent capability.

A new independent login attempt gets a new `AdmissionAttemptRef`.

### 6.4 Consumer relationship

A PreAdmissionGrant may carry `AdmissionAttemptRef` for correlation, but authoritative game consumption still keys replay safety on `GrantNonce` (plus trusted issuer/profile context as defined by the final profile).

Game admission must not infer:

```text
same AdmissionAttemptRef == same GameSessionId
```

or

```text
new AdmissionAttemptRef == permission to bypass an already consumed/still-active account/character/session state
```

Normal account/character/lease/session checks remain binding.

## 7. Revised grant semantics

The final FND-04 PreAdmissionGrant profile should therefore authenticate at minimum, where applicable:

- credential type;
- security/profile revision;
- issuer;
- audience/purpose;
- `AdmissionAttemptRef`-class producer correlation reference;
- cryptographically random `GrantNonce`;
- AccountId;
- Platform account-security generation/revision or equivalent post-issuance security applicability evidence if the chosen security design uses it;
- CharacterId;
- WorldId;
- ChannelId for fresh entry;
- route/offer/topology generation or immutable target revision;
- issuance-time runtime observation/source revision;
- issuance-time scope ownership generation where required by stale-owner prevention;
- protocol/transport profile;
- required runtime/content/ruleset compatibility revisions;
- server issuance/not-before/expiry semantics;
- signing-key identifier/version.

The exact representation is deferred to the final cross-language profile.

## 8. Revised validation sequence

The final admission sequence should conceptually include:

1. FND-02 bounded material/frame validation;
2. credential type/profile parsing;
3. trusted issuer/key-purpose cryptographic verification;
4. issuer/audience/time/skew validation;
5. post-issuance Platform account-security applicability validation under the final chosen mechanism;
6. route/topology/runtime-observation/ownership-generation applicability validation;
7. current game-domain target owner/readiness/revision validation;
8. grant nonce consume eligibility/replay validation;
9. authoritative AccountId→CharacterId ownership revalidation;
10. AccountPresenceClaim / duplicate-login / actor lifecycle validation;
11. CharacterLease compatibility/acquisition;
12. one atomic final admission/GameSession/connection-generation commit.

`AdmissionAttemptRef` is available for correlation/idempotency evidence but is not an authorization check that can bypass steps 5–12.

## 9. Failure and race cases required by final FND-04

The final contract/test catalogue must cover at least:

### 9.1 Account disabled/security generation advances after issuance

Given a valid signed unexpired grant issued under security state `S`, when Platform security authority moves to a state that should revoke `S`, final admission follows the explicitly chosen bounded revocation/staleness policy and never relies on client claims.

### 9.2 Emergency security revocation while grant is in flight

There is a testable maximum interval/evidence rule after which a revoked grant cannot succeed in fresh admission.

### 9.3 Runtime owner generation changes after issuance

Grant issued from runtime observation generation `G`; current target generation is `G+1` before admission. Default outcome is stale/superseded rejection and fresh routing/grant unless final contract proves safe carry-forward semantics.

### 9.4 Delayed old runtime observation reaches Platform

Platform must not issue based on a superseded observation merely because it arrived later or has a favorable wall-clock timestamp. Current-owner/source-revision comparison remains required.

### 9.5 Ambiguous issuer response

Retry with the same `AdmissionAttemptRef` cannot create two independently usable grants.

### 9.6 Multiple grants / same account-character race

Even if two independently authorized Platform attempts exist, Oteryn-v2 account presence, CharacterLease and GameSession rules still ensure at most one successful current gameplay authority.

### 9.7 Grant replay

Reusing the same `GrantNonce` cannot create a second successful admission, regardless of AdmissionAttemptRef.

## 10. Failure vocabulary direction

No new foundation entity is required by this refinement.

The final FND-04 error/failure mapping should distinguish internally at least:

- `PLATFORM_SECURITY_STATE_STALE_OR_REVOKED` -> `SESSION_REJECTED` or `AUTHENTICATION_FAILED` depending final security semantics;
- `RUNTIME_OBSERVATION_SUPERSEDED` -> `STALE_GENERATION` / `SESSION_REJECTED` according to whether a canonical generation mismatch is exposed internally;
- `ADMISSION_ATTEMPT_RECONCILIATION_REQUIRED` -> bounded internal retry/reconciliation category, public mapping possibly `DEPENDENCY_UNAVAILABLE` / `TIMEOUT`;
- `ADMISSION_GRANT_REPLAYED` -> `SESSION_REJECTED`;
- wrong route/owner/revision -> `STALE_GENERATION` or `UNSUPPORTED_REVISION` according to exact cause.

Raw account-security revision values, grant nonce, bearer material and private runtime fencing details must not be leaked in public errors.

## 11. Consequences for final FND-04 contract

The final contract must now explicitly decide all of the following before implementation authority can be considered:

1. the post-issuance Platform account-security revocation/freshness mechanism and maximum accepted staleness/risk window;
2. whether an account-security generation/revision is carried in the grant and how current validation is obtained;
3. exact fresh-entry runtime observation/route/ownership-generation bindings;
4. whether ownership-generation change always invalidates an unexpired fresh-entry grant or which proven exceptions exist;
5. `AdmissionAttemptRef` semantics, retention and idempotent issuer retry behavior;
6. distinct `GrantNonce` consume/replay semantics;
7. cross-language profile fields/fixtures for both concepts;
8. E2E cases for the races in Section 9.

These decisions are in addition to the remaining final FND-04 items listed in the primary analysis baseline.

## 12. Non-authorization

This refinement does not choose or implement:

- JWT, PASETO, COSE or another concrete envelope;
- signature algorithm/library;
- KMS/HSM/vendor;
- security-generation projection transport/cache/database;
- synchronous introspection API;
- grant issuance service implementation;
- Oteryn-v2 consume store schema;
- runtime status transport;
- protocol message encoding;
- production TTL/staleness values;
- Platform or game code;
- production activation.

It defines only the semantic/race constraints that the final FND-04 contract must resolve.

## 13. Concise reconciliation rule

```text
Platform issue decision at T0
-> current account-security state S0
-> fresh current-owner runtime observation R0 / ownership generation G0
-> one producer AdmissionAttemptRef A
-> one independently consumable grant with GrantNonce N

before final admission
-> Platform security state may advance S0 -> S1
-> runtime owner may advance G0 -> G1

Oteryn-v2 final admission
-> validate cryptographic grant profile
-> validate chosen bounded Platform security freshness/revocation rule
-> validate route/runtime observation applicability
-> validate current game owner/revisions
-> consume N at most once
-> revalidate AccountId -> CharacterId
-> enforce AccountPresenceClaim + CharacterLease
-> create canonical GameSessionId only on atomic success

A != N
A != GameSessionId
N != GameSessionId
Platform observation != current gameplay authority
nominal grant expiry != automatic proof of unchanged account-security state
```
