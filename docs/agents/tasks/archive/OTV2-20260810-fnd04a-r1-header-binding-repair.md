# OTV2-20260810-fnd04a-r1-header-binding-repair

```yaml
task_id: OTV2-20260810-fnd04a-r1-header-binding-repair
title: FND-04A-R1 deterministic protected-header/binding validation repair
mode: CONTRACT
status: completed
repository: blakinio/Oteryn-v2
base_branch: main
delivery_issue: 120
programme_issue: 112
delivery_pr: 125
superseded_pr: 114
trusted_reconstruction_base_sha: 43ca28f1f0f259c08a275c92946aa35f05d4d112
reconstruction_squash_sha: 6c34e48e868b604b824f079920694d007f7eb493
final_delivery_head_sha: 2cd6d28af32326a1d7fdab93e9890ff739f21923
merge_sha: cae318b8891844891c734012eb2020e669ebaff4
repair_cycles_for_gate: 3
max_repair_cycles_for_gate: 3
owner: GPT-5.6 Sol architecture continuation session
completed_at: 2026-08-10T11:30:00+02:00
ownership_released: true
owned_paths: []
canonical_public_contracts:
  - docs/architecture/FND-04A_AUTHORITY_FRESH_ADMISSION_CONTRACT.md
  - docs/contracts/FND-04_PRE_ADMISSION_GRANT_PROFILE_V1.md
```

## Result

`FND-04A` authority + fresh-admission architecture is accepted on `main` through bounded successor PR #125. No runtime implementation was authorized.

The successor was necessary because original PR #114 exhausted its `3/3` repair budget and a later terminal review found a material protected-header / error-classification contradiction. The owner explicitly authorized a fresh bounded successor instead of an illegal fourth repair cycle. PR #114 was subsequently closed unmerged and remains historical reviewed evidence only.

## Reconstruction provenance

The successor branch started from exact current trusted main:

```text
main@43ca28f1f0f259c08a275c92946aa35f05d4d112
```

Reviewed #114 content at exact head:

```text
79678485d009c22ece2736c822d6b75b6d235ad2
```

was reconstructed onto that branch as one staging squash:

```text
6c34e48e868b604b824f079920694d007f7eb493
```

rather than importing the exhausted PR's 11-commit history.

## Bounded repair history

### Cycle 1 — protected `typ` classification

Froze deterministic distinction between malformed protected-header structure, authentication failure and authenticated binding mismatch. Semantic exact `typ` comparison occurs only after successful Ed25519 authentication.

### Cycle 2 — verifier-anchored trust scope

Froze pre-signature trust selection to the verifier-configured expected fresh-entry v1 issuer/profile/key-purpose context. Unauthenticated token semantics cannot select or broaden trust; `kid` only indexes the fixed trusted set.

### Cycle 3 — complete cryptographic / payload-schema precedence

Froze the complete v1 credential-validation precedence:

```text
malformed JWS/protected-header shape
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

The gate exhausted its successor repair budget at `3/3`; final terminal review found no further material issue.

## Preserved FND-04A invariants

The accepted contracts retain:

- Platform bounded attempt authority versus Oteryn-v2 final game-domain admission authority;
- separate AccountPresenceClaim, CharacterLease, GameSession, TransportBinding and RuntimeScopeAuthority concepts;
- one atomic final fresh-admission revalidation/authority commit;
- AccountId -> CharacterId ownership validation before CharacterId -> WorldId/world-eligibility classification;
- `ADMISSION_GRANT_WORLD_STALE` with no GrantNonce/presence/lease/session/transport mutation and no stale-grant retarget;
- independent protocol/transport/ruleset/content/map/world-policy/offer revision dimensions;
- AdmissionAttemptRef versus one-time GrantNonce separation;
- account-global duplicate-login non-preemption;
- authenticated source-age `<=5s` security/trust evidence with monotonic anti-rollback ordering and fail-closed restart floor reconstruction;
- complete FND-04A Foundation Error Vocabulary rows and credential-free/redacted diagnostics;
- explicit exclusion of FND-04B reconnect/recovery/PREPARE-COMMIT/liveness/ControlLossEpoch semantics.

## Final validation evidence

Delivery exact head:

```text
2cd6d28af32326a1d7fdab93e9890ff739f21923
```

Required exact-head CI:

- Agent Governance run `31374454008`: **PASS**;
- Dependency Review run `31374454334`: **PASS**;
- CodeQL run `31374453636`: **PASS**.

Terminal architecture/security review:

- review `4895362558`: **PASS**;
- material findings: `0`;
- unresolved review threads: `0`.

Runtime/component/browser E2E:

```text
NOT_APPLICABLE
```

The delivery is architecture/documentation only; future implementation must execute the contract-defined cryptographic/interoperability, replay, freshness/rollback, independent-revision and world-transfer race fixtures.

Squash merge:

```text
cae318b8891844891c734012eb2020e669ebaff4
```

## Supersession / hygiene

- PR #114: closed unmerged as superseded; historical evidence only;
- Issue #113: closed after completion through the governed successor;
- internal reconstruction staging Issue #122: closed; no architecture authority;
- accidental placeholders #121 and #123 were closed `not_planned` and carry no project decision.

## Next architecture gate

`FND-04` remains incomplete. Per replacement programme #112, the ordered successor is `FND-04B — Reconnect + Recovery + Continuity`, followed by `FND-04C` integration.

This archive releases FND-04A-R1 task ownership. The two public FND-04A contracts remain canonical architecture on `main` until explicitly superseded by a later accepted decision.
