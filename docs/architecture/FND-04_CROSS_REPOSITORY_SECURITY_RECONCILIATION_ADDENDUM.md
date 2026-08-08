# FND-04 — Cross-Repository Pre-Admission Security Reconciliation Addendum

- Status: Architecture reconciliation input; mandatory input to the later final FND-04 contract
- Date: 2026-08-08
- Gate: `FND-04`
- Coordination ID: `OTV2-NATIVE-FOUNDATION`
- Repository: `blakinio/Oteryn-v2`
- Applies to: final FND-04 admission grant validation, pre-admission revocation/freshness and ambiguous issuance semantics
- Consumes:
  - `docs/architecture/FND-04_SESSION_ADMISSION_LEASE_ANALYSIS_BASELINE.md`
  - `docs/architecture/ADR-0003-platform-identity-game-gateway-and-admission-boundary.md`
  - `docs/architecture/FND-02_PROTOCOL_OTERYN_V1_CONTRACT.md`
  - `docs/architecture/FND-03_RUNTIME_EXECUTION_CONTRACT.md`
  - current read-only `blakinio/Oteryn-Platform` ADR 0031
  - current read-only Platform `OTERYN_V2_PRE_ADMISSION_HANDOFF_CONTRACT.md`
  - current read-only Platform `OTERYN_V2_RUNTIME_STATUS_PROJECTION_CONTRACT.md`
- Does not authorize: runtime/protocol/persistence implementation, Platform writes, production keys, production traffic, deployment or live account/session mutation

## 1. Purpose

Merged FND-04 analysis PR #104 established a strong local model for account presence, CharacterLease fencing, GameSession lifecycle and TransportBinding. Independent cross-repository review found three Platform-side native pre-admission semantics that the later **final FND-04 contract must explicitly resolve** rather than infer from token expiry, route validation or one-time grant consumption:

1. Platform account-security authority may change after a grant is issued but before game admission;
2. a grant may become stale because the runtime/admission owner or route evidence that justified issuance has been superseded;
3. Platform grant-issuance retry/idempotency identity and game-domain grant-consume replay identity solve different ambiguity boundaries and must not be silently collapsed.

This addendum does not rewrite the merged analysis baseline. It is a mandatory reconciliation input to the final FND-04 contract.

## 2. Decision timing

### Must decide now? — YES, before final FND-04 acceptance

These semantics directly block a secure producer/consumer contract for native admission. A final FND-04 contract that omits them would leave implementations to invent incompatible security behavior independently.

### Concrete downstream work blocked

Until the final FND-04 contract resolves this addendum, do not claim implementation-ready semantics for:

- Platform `PreAdmissionGrant` production;
- Oteryn-v2 grant verification/consumption;
- native admission key/revocation integration;
- route/runtime-generation admission validation;
- ambiguous grant issuance retry/reconciliation;
- exact cross-repository admission fixtures/E2E;
- production native route activation.

### What becomes harder if wrong

A wrong or implicit choice could permit:

- a grant issued before account compromise/disablement to remain usable contrary to the accepted Platform security policy;
- a delayed grant issued for a superseded runtime owner to reach a replacement owner and be accepted only because its ordinary route fields still look valid;
- duplicate independently usable grants after an ambiguous Platform issuance response;
- accidental coupling of a Platform producer-operation identifier to a game consume nonce, preventing safe retries or creating cross-domain replay confusion;
- cross-version producer/consumer drift that passes signature verification but violates current admission policy.

### Evidence that may justify supersession

A later accepted contract may change the selected mechanism with named evidence from security review, fault injection, measured availability/latency, operational key/revocation testing or a changed Platform security policy. The semantic requirements below remain unless explicitly superseded.

### Deliberately not decided here

This addendum does not select:

- JWT/JWS/PASETO or another credential container;
- signature algorithm or exact key format;
- online introspection versus revocation feed versus security-generation binding as the final account-revocation primitive;
- runtime-status transport/API/event technology;
- PostgreSQL/Redis/other storage for replay or idempotency state;
- exact TTL, clock-skew, cache-staleness or retry-window values;
- exact field names or wire encoding.

## 3. Current authority boundary

The durable ownership split remains unchanged:

```text
Platform
  owns reusable account security policy
  + Game Login Ticket
  + route/policy selection
  + authorization to issue one bounded admission attempt

Oteryn-v2
  owns final authoritative ownership/lifecycle validation
  + current runtime/admission-owner validation
  + one-time grant consumption
  + account presence / CharacterLease
  + canonical GameSessionId
  + admitted-session lifecycle
```

A valid Platform signature proves only that the Platform issuer produced the capability under its contract. It does not prove that every fact that justified issuance is still current at final admission.

## 4. Requirement A — post-issuance Platform security changes

### 4.1 Problem

Between grant issuance and game admission, Platform security authority may change because of:

- Identity disablement;
- credential-compromise response;
- account-security generation advance;
- administrative emergency revocation;
- another accepted Platform security transition that invalidates outstanding game authorization.

The final FND-04 contract must define a deterministic, testable disposition for this race.

### 4.2 Required invariant

The system must have a bounded rule answering:

> Can a grant issued under Platform account-security state `S` still be consumed when authoritative Platform account-security state has advanced to `S+1` or has been explicitly revoked before game admission?

The answer may not be left to each implementation.

### 4.3 Acceptable mechanism classes

The final contract may select one or a justified composition of:

- sufficiently short grant lifetime with an explicitly accepted residual-risk bound;
- a Platform-issued monotonic/security-generation binding whose currentness can be proven at admission;
- bounded revocation/introspection for security-sensitive transitions;
- authenticated revocation/generation projection from Platform to the game admission authority;
- another reviewed mechanism that gives equivalent fail-closed semantics.

The mechanism must not turn the Rust game server into a second password/OAuth/MFA authority.

### 4.4 Mandatory final-contract evidence

Before implementation readiness, the final contract must define tests for at least:

1. grant issued -> Identity disabled -> admission attempted;
2. grant issued -> security generation/revocation advances -> admission attempted;
3. revocation state unavailable/stale when the selected mechanism requires it;
4. emergency issuer-key compromise versus per-account security revocation, proving these are not confused;
5. mixed producer/consumer rollout where one side does not understand the required revocation binding.

The public failure response may be deliberately coarse, but internal typed evidence must distinguish policy invalidation from malformed credentials without exposing account-security details.

## 5. Requirement B — runtime observation and ownership-generation binding

### 5.1 Problem

Platform may issue a grant only after combining configured World Registry policy with fresh applicable Oteryn-v2 runtime evidence. The game runtime can recover or move before the client presents the grant.

Signature validity and nominal expiry alone cannot make an old runtime observation current again.

### 5.2 Required invariant

The final contract must explicitly define which issuance-time runtime facts are bound into, referenced by or otherwise made verifiable from the PreAdmissionGrant, including when applicable:

- `WorldId` and `ChannelId`;
- route/offer/topology revision;
- admission target identity;
- runtime observation/source revision;
- current scope/admission ownership generation or equivalent stale-owner fence;
- protocol/content/ruleset/runtime compatibility revisions required for admission.

It must also define which changes make an otherwise unexpired grant unusable.

### 5.3 Minimum stale-owner rule

A delayed grant may never acquire authority merely because it reaches a process serving the same `WorldId + ChannelId` after recovery.

If issuance relied on owner/generation evidence that has since been superseded, final admission must either:

- reject the stale grant; or
- prove through an explicitly accepted generation-independent route/revalidation rule that the grant remains valid without weakening stale-owner fencing.

Silently ignoring the issuance-time owner-generation relationship is not acceptable.

### 5.4 Relationship to current placement

For fresh entry, Platform may authorize a selected Channel/route, but the target game side must still prove current accepted runtime scope/admission authority before committing admission.

For recovery of an existing actor/session, current game-domain placement remains authority. A stale Platform/client route must not move an actor or turn a fresh-entry grant into a recovery/handoff credential.

### 5.5 Mandatory final-contract evidence

At minimum test:

1. grant issued to generation `G` -> owner recovers/restarts to `G+1` before presentation;
2. delayed grant reaches stale owner `G` after `G+1` is current;
3. same WorldId/ChannelId but route/topology revision changes;
4. grant reaches wrong Channel/Instance/target;
5. runtime-status evidence becomes stale/unavailable between issuance and final admission;
6. mixed-version consumer cannot interpret a mandatory generation/revision binding.

No test may accept a fallback to Canary or an alternate native route with the same credential.

## 6. Requirement C — issuance-attempt identity versus consume nonce

### 6.1 Two ambiguity boundaries

The Platform producer and game consumer solve different replay/idempotency problems:

```text
Platform admission-attempt identity
  scope: one logical grant-issuance attempt
  purpose: issuer idempotency, ambiguous issuance reconciliation, tracing

PreAdmissionGrant one-time nonce
  scope: one concrete issued capability
  purpose: game-domain replay detection / one-successful-admission consumption
```

They may be carried together, but the final contract must not assume they are the same semantic value merely because both are unique.

### 6.2 Producer-side ambiguity rule

If grant issuance may have committed but the Gateway/client did not receive the result, the issuer must use one safe bounded model:

- idempotently recover the exact prior issuance outcome by the same admission-attempt identity; or
- deterministically invalidate/retire that attempt and require a new authorized attempt under an explicit reconciliation rule.

It must not mint multiple independently usable capabilities for one logical issuance attempt because a response was lost.

### 6.3 Consumer-side ambiguity rule

Once a concrete grant exists, game-domain one-time consumption ensures at most one successful authoritative admission from that capability. A consumed nonce never becomes reusable because the admission response was lost.

Post-commit client recovery follows the admitted-session/reconnect/recovery contract, not producer-side grant issuance retry semantics.

### 6.4 No unnecessary foundation entity ID

This requirement does not prove that `AdmissionId` is needed as a new FND-ID entity. A bounded producer operation/correlation identifier can remain operation-scoped unless later durability/recovery design proves that it is a separately addressable long-lived domain entity.

### 6.5 Mandatory final-contract evidence

Test at least:

1. issuer commits grant but response is lost, exact logical retry occurs;
2. two concurrent issuer retries use one admission-attempt identity;
3. two distinct concrete grants cannot accidentally share one consume nonce;
4. consumed grant plus lost game-admission response does not trigger grant reuse;
5. logs/traces correlate issuer attempt and consume result without recording bearer material.

## 7. Cross-repository compatibility contract

The final FND-04 package must name exact compatible producer/consumer revisions and provide independent fixtures for all admission-security semantics it freezes.

At minimum the compatibility matrix must account for:

- credential/security-profile revision;
- Platform account-security revocation binding revision;
- route/runtime ownership-generation binding revision;
- game-domain admission/session-state-machine revision;
- protocol major/transport profile and applicable revision compatibility.

Unsupported mandatory semantics fail closed. A lower-version consumer may not ignore a critical field and accept the grant as though it were optional.

## 8. Failure vocabulary implications

The final contract should preserve stable internal distinctions for:

- credential malformed/authentication failure;
- expired/consumed/replayed credential;
- Platform account-security authorization superseded/revoked;
- stale route/runtime/admission-owner generation;
- incompatible security/profile/revision;
- ambiguous producer issuance requiring reconciliation;
- admission dependency unavailable.

Public errors may collapse sensitive distinctions. Internal audit/correlation must not include raw grants, reconnect secrets, OAuth tokens or signing material.

## 9. Final FND-04 acceptance additions

The later final FND-04 contract is not complete until it explicitly:

1. selects and documents the post-issuance Platform account-security change disposition;
2. selects the runtime observation/ownership-generation binding and stale-grant invalidation rule;
3. defines producer admission-attempt idempotency separately from concrete grant consume replay semantics, or proves a deliberate equivalence;
4. maps these semantics to stable failure classes and redaction requirements;
5. pins producer/consumer compatibility behavior and downgrade failure;
6. requires exact-revision fixtures/fault tests for all three race classes;
7. preserves Platform as reusable-credential/security-policy authority and Oteryn-v2 as final admission/GameSession/lease authority.

## 10. Relationship to merged analysis PR #104

The following #104 directions remain compatible with this addendum:

- separate AccountPresenceClaim / CharacterLease / GameSession / TransportBinding / RuntimeScopeAuthority semantics;
- hybrid signed PreAdmissionGrant plus authoritative game-domain one-time consumption as the recommended credential class;
- game-domain rotating reconnect proof;
- no new AdmissionId/CharacterLeaseId without evidence;
- fresh admission as one externally unambiguous authority transition;
- current game-domain placement as recovery authority;
- stale generations fail closed;
- lease uncertainty never self-grants replacement authority;
- no runtime implementation is authorized by architecture analysis alone.

This addendum narrows the remaining final-contract obligations; it does not supersede those compatible directions.

## 11. Gate result

If this addendum passes exact-head review/CI/audit and merges, the safe next package is the **final architecture-only FND-04 Identity, Game Session, Admission and Character Lease Contract** consuming both:

- `FND-04_SESSION_ADMISSION_LEASE_ANALYSIS_BASELINE.md`;
- `FND-04_CROSS_REPOSITORY_SECURITY_RECONCILIATION_ADDENDUM.md`.

No runtime or production implementation is authorized merely because these architecture inputs exist.
