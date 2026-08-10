# OTV2-20260810-fnd04a-r1-header-binding-repair

```yaml
task_id: OTV2-20260810-fnd04a-r1-header-binding-repair
title: FND-04A-R1 deterministic protected-header/binding validation repair
mode: CONTRACT
status: validating
repository: blakinio/Oteryn-v2
base_branch: main
branch: docs/fnd04a-r1-header-binding-repair
issue: 120
programme_issue: 112
pr: 125
supersedes_delivery_pr: 114
superseded_delivery_head: 79678485d009c22ece2736c822d6b75b6d235ad2
trusted_base_sha: 43ca28f1f0f259c08a275c92946aa35f05d4d112
reconstruction_squash_sha: 6c34e48e868b604b824f079920694d007f7eb493
owner: GPT-5.6 Sol architecture continuation session
created_at: 2026-08-10T11:05:00+02:00
updated_at: 2026-08-10T11:22:00+02:00
repair_cycles_for_current_gate: 3
max_repair_cycles_for_current_gate: 3
final_head_sha: null
final_head_frozen_at: null
owner_action_required: null
blocker: null
owned_paths:
  - docs/agents/tasks/active/OTV2-20260810-fnd04a-r1-header-binding-repair.md
  - docs/architecture/FND-04A_AUTHORITY_FRESH_ADMISSION_CONTRACT.md
  - docs/contracts/FND-04_PRE_ADMISSION_GRANT_PROFILE_V1.md
public_contracts:
  - docs/architecture/FND-04A_AUTHORITY_FRESH_ADMISSION_CONTRACT.md
  - docs/contracts/FND-04_PRE_ADMISSION_GRANT_PROFILE_V1.md
```

## Goal

Deliver the bounded successor repair authorized by the product owner after PR #114 exhausted its `3/3` repair budget and an exact-head terminal review found a material interoperability contradiction. This task is a new gate with its own repair budget; it is not a fourth repair cycle on #114.

## Source / provenance

- current trusted reconstruction base: `main@43ca28f1f0f259c08a275c92946aa35f05d4d112`;
- reviewed but unmerged exhausted candidate: PR #114 exact head `79678485d009c22ece2736c822d6b75b6d235ad2`;
- staging reconstruction was squashed once onto the fresh current-main branch as `6c34e48e868b604b824f079920694d007f7eb493`, preserving content without importing #114's 11-commit history;
- all previously repaired #114 semantics remain candidate evidence unless explicitly changed by the bounded R1 repairs below;
- Issue #120 records the owner disposition and bounded successor acceptance criteria;
- delivery PR: #125.

## Repair cycle 1 — protected `typ` classification

The v1 profile simultaneously required an early exact protected-header check before signature verification and required a correctly signed wrong `typ` to map to `ADMISSION_GRANT_BINDING_MISMATCH` after authentication. Because `typ` is a protected JOSE member, fail-fast consumers could classify the same token differently.

Accepted repair:

```text
malformed typ/header structure
-> ADMISSION_GRANT_MALFORMED

well-formed header + key/signature failure
-> ADMISSION_GRANT_AUTHENTICATION_FAILED

authenticated wrong typ/iss/aud/purpose
-> ADMISSION_GRANT_BINDING_MISMATCH
```

Semantic exact `typ=oteryn-admission+jwt` comparison is therefore deferred until successful Ed25519 verification.

## Repair cycle 2 — verifier-anchored trust scope

The first successor freeze left `key/profile` trust potentially interpretable as token-selected before `profile`/`iss`/`purpose` authentication.

Accepted repair:

```text
pre-signature trust scope
= verifier-configured expected fresh-entry v1 issuer/profile/key-purpose context

untrusted iss/aud/profile/purpose/typ
!= trust selector

kid
= selector only inside the fixed trusted set
```

Wrong/unsupported profile plus failed signature remains authentication failure; only a successfully verified unsupported profile can produce `ADMISSION_GRANT_REVISION_UNSUPPORTED`.

## Repair cycle 3 — complete cryptographic/schema precedence

The second successor freeze left a final material ambiguity for a syntactically valid non-exact `alg`: it was rejected before signature, but its exact error classification was not frozen. The same review also required adjacent `kid` and authenticated payload-schema precedence to be explicit so no equivalent oracle/divergence remains.

Accepted final precedence:

```text
unparsable / malformed JWS or protected-header shape
-> ADMISSION_GRANT_MALFORMED

well-formed non-exact alg
OR fixed-scope kid/key/trust/signature failure
-> ADMISSION_GRANT_AUTHENTICATION_FAILED

authenticated exact payload-schema violation
-> ADMISSION_GRANT_MALFORMED

authenticated wrong typ/iss/aud/purpose
-> ADMISSION_GRANT_BINDING_MISMATCH

authenticated unsupported profile
-> ADMISSION_GRANT_REVISION_UNSUPPORTED
```

Details:

1. missing/null/non-string/out-of-bound `alg` or `kid`, duplicate/extra/forbidden protected members and structurally invalid `typ` are malformed input;
2. syntactically valid `alg` other than exact `Ed25519` is authentication failure with no negotiation, downgrade or fallback;
3. well-formed `kid` absent/untrusted in the fixed verifier-configured trust set is authentication failure;
4. pre-signature payload processing is limited to bounded safe parsing needed to authenticate; semantic exact claim membership/unknown-claim rejection is not used as a pre-authentication oracle;
5. invalid signature plus otherwise well-formed missing/unknown semantic payload claim remains authentication failure;
6. successfully signed missing/unknown/wrong-type/noncanonical payload claim is malformed;
7. fixtures now cover the complete algorithm, key, trust, signature, exact-schema, binding and profile precedence matrix.

Repair budget is now exhausted at `3/3`. Any new material finding after the next freeze is a **hard blocker**. No task-local exception or fourth repair cycle is authorized.

## Scope preserved from FND-04A

The successor keeps the #114 accepted candidate semantics for:

- Platform attempt versus game-domain final authority;
- AccountPresenceClaim, CharacterLease, GameSession, initial TransportBinding and RuntimeScopeAuthority separation;
- atomic fresh-admission revalidation/commit;
- current AccountId->CharacterId before CharacterId->WorldId/world-eligibility ordering;
- no stale-world retarget;
- independent protocol/transport/ruleset/content/map/world-policy/offer revisions;
- GrantNonce versus AdmissionAttemptRef;
- account-global duplicate-login non-preemption;
- authenticated source-age <=5s security/trust evidence and monotonic anti-rollback fences;
- complete FND-04A error-vocabulary rows;
- no reconnect/recovery/PREPARE-COMMIT implementation or finalization.

## Explicit exclusions

No Rust runtime, protocol codec/schema implementation, persistence schema, Platform write, KMS/HSM/vendor selection, deployment, production traffic, FND-04B reconnect/recovery semantics, FND-04C integration, entitlement implementation or Premium/VIP activation.

## Validation plan

- verify branch ancestry starts from exact current trusted main and reconstruction added only reviewed #114 content;
- verify final changed paths versus main are exactly this active task plus the two FND-04A public contracts;
- verify no stale #114 active task remains in the successor diff;
- verify architecture/profile agree on complete cryptographic/header/schema/binding/profile precedence and verifier-configured trust scope;
- verify all algorithm, malformed/unknown `kid`, protected-member, invalid-signature + semantic defect, authenticated schema defect, binding and profile fixtures are explicit;
- verify all prior #114 P1/P2 repairs remain intact;
- verify current-main entitlement dependency remains separate and does not alter FND-04A semantics;
- perform full exact-head architecture/security self-review;
- freeze exact final head only after all repair metadata is complete;
- require exact-head Agent Governance, Dependency Review and CodeQL PASS;
- require zero unresolved material review threads;
- require one terminal exact-head architecture/security review with zero material findings;
- squash merge unchanged accepted head only.

Runtime/component/browser E2E: `NOT_APPLICABLE` because this gate changes architecture/documentation only. Future implementation must execute the named interoperability/security fixtures.

## Current checkpoint

```yaml
status: validating
last_progress: Final allowed repair cycle 3/3 completed. Architecture and profile now define exhaustive pre-auth cryptographic/header classification, fixed trust selection, post-auth schema/binding/profile precedence, and explicit adversarial fixtures.
repair_cycles_for_current_gate: 3
next_action: freeze the resulting exact head on the immutable PR surface, repeat full exact-head architecture/security review and all required CI. Any new material finding is a hard blocker.
```
