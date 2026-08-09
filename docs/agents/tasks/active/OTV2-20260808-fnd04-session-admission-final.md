# OTV2-20260808-fnd04-session-admission-final

```yaml
task_id: OTV2-20260808-fnd04-session-admission-final
title: Finalize FND-04 identity Game Session admission and character lease contract
mode: CONTRACT
status: validating
repository: blakinio/Oteryn-v2
base_branch: main
branch: docs/OTV2-20260808-fnd04-session-admission-final
pr: 109
base_sha: 27f7f647f04e3b1a4151f9b124401986910f03d8
head_sha: null
final_head_sha: null
final_head_frozen_at: null
owner: GPT-5.6 Sol architecture continuation session
created_at: 2026-08-08T21:22:00+02:00
updated_at: 2026-08-09T11:29:00+02:00
execution_budget_minutes: 60
owned_paths:
  - docs/agents/tasks/active/OTV2-20260808-fnd04-session-admission-final.md
  - docs/architecture/FND-04_IDENTITY_GAME_SESSION_ADMISSION_CHARACTER_LEASE_CONTRACT.md
  - docs/architecture/FND-04_HEALTHY_BINDING_REBIND_SECURITY_REFINEMENT.md
  - docs/contracts/FND-04_PRE_ADMISSION_GRANT_PROFILE_V1.md
  - docs/contracts/FND-04_REAUTHENTICATED_RECOVERY_GRANT_PROFILE_V1.md
  - docs/contracts/FOUNDATION_FAILURE_SCENARIOS.md
  - docs/architecture/FOUNDATION_PROGRAMME_CURRENT_STATUS.md
public_contracts:
  - docs/architecture/FND-04_IDENTITY_GAME_SESSION_ADMISSION_CHARACTER_LEASE_CONTRACT.md
  - docs/architecture/FND-04_HEALTHY_BINDING_REBIND_SECURITY_REFINEMENT.md
  - docs/contracts/FND-04_PRE_ADMISSION_GRANT_PROFILE_V1.md
  - docs/contracts/FND-04_REAUTHENTICATED_RECOVERY_GRANT_PROFILE_V1.md
depends_on:
  - docs/architecture/FND-04_SESSION_ADMISSION_LEASE_ANALYSIS_BASELINE.md
  - docs/architecture/FND-04_PLATFORM_PRE_ADMISSION_RECONCILIATION_REFINEMENT.md
  - docs/architecture/ADR-0003-platform-identity-game-gateway-and-admission-boundary.md
  - docs/architecture/ADR-0012-character-authority-and-platform-lifecycle-boundary.md
  - docs/architecture/FND-ID-01_FOUNDATION_IDENTIFIER_CONTRACT.md
  - docs/architecture/FND-02_PROTOCOL_OTERYN_V1_CONTRACT.md
  - docs/architecture/FND-03_RUNTIME_EXECUTION_CONTRACT.md
  - docs/architecture/DISCONNECT_REENTRY_PVE_PROTECTION_OWNER_DECISION.md
  - docs/contracts/FOUNDATION_ERROR_VOCABULARY.md
  - docs/contracts/FOUNDATION_FAILURE_SCENARIOS.md
  - blakinio/Oteryn-Platform@216f5b2817e9d102337608609e344518512c2a0d:docs/contracts/OTERYN_V2_PRE_ADMISSION_HANDOFF_CONTRACT.md
  - blakinio/Oteryn-Platform@216f5b2817e9d102337608609e344518512c2a0d:docs/contracts/OTERYN_V2_RUNTIME_STATUS_PROJECTION_CONTRACT.md
blocks:
  - production Game Session admission and reconnect implementation
  - character lease/account presence implementation
  - Platform native admission/recovery producer rollout
  - production protocol-oteryn admission/reconnect/recovery traffic
cross_repository_coordination_id: OTV2-NATIVE-FOUNDATION
external_repositories:
  - blakinio/Oteryn-Platform (read-only reconciliation evidence)
```

## Outcome

Deliver the complete architecture-only FND-04 contract needed before native identity/admission/reconnect/lease implementation can be designed without guessing authority, replay, reconnect, recovery, trust freshness, timing, compatibility or lease security semantics.

Acceptance completes the FND-04 architecture gate only. It does not authorize runtime, Platform, persistence, protocol-codec, key-management, deployment or production implementation.

## Architecture and source of truth

- FND-04 analysis plus the current-Platform reconciliation refinement are canonical on `main` after PRs #104/#107 and lifecycle closeout #108.
- Duplicate/superseded PRs #106 and #110 are closed unmerged and contribute no authority.
- External Platform evidence remains pinned read-only at `blakinio/Oteryn-Platform@216f5b2817e9d102337608609e344518512c2a0d`.
- Platform Identity/Gateway authorizes bounded attempts; Oteryn-v2 remains final GameSession, AccountPresenceClaim, CharacterLease and runtime-control authority.
- FND-02 owns bootstrap, GameSession issuance boundary, connection generation and command/reconciliation semantics; FND-03 owns runtime ordering/fencing/time execution.
- Fresh entry and reauthenticated existing-actor recovery use mutually exclusive signed profiles with fully specified JOSE `alg=Ed25519`; deprecated polymorphic `EdDSA` fallback is rejected.
- Oteryn-owned grant identities follow FND-ID UUIDv7/RFC-variant semantics; Platform-owned AccountId is not silently redefined.
- AdmissionAttemptRef is producer idempotency/correlation, distinct from GrantNonce. Ambiguous fresh-entry issuance returns `ADMISSION_ATTEMPT_RECONCILIATION_REQUIRED` and permits same-AdmissionAttemptRef reconciliation only until deterministic retirement/proof makes a new attempt safe.
- Recovery issuance has the symmetric stable `RECOVERY_ATTEMPT_RECONCILIATION_REQUIRED`: same recovery `attempt_ref` reconciliation only; no blind second recovery grant/independent recovery attempt; deterministic retirement plus proof any possibly issued recovery capability is no longer acceptable before a new recovery attempt; no gameplay/fresh-entry authority.
- Both grant profiles require authenticated signing-key/profile trust/revocation evidence with accepted age `<=5 seconds` at authority-changing validation boundaries. Stale, unavailable, unauthenticated, contradictory or unprovable trust evidence maps to the purpose-specific `*_GRANT_SECURITY_EVIDENCE_STALE`; fresh current evidence explicitly marking the exact key/profile unknown/revoked/not trusted maps to the purpose-specific `*_GRANT_AUTHENTICATION_FAILED`. Neither consumes its nonce or creates authority.
- Reconnect PREPARE is a candidate reservation only. COMMIT atomically revalidates current incumbent/session/presence/lease/runtime/reconciliation eligibility and, for recovery grants, token/nonce/recovery-key-profile trust **and trust-evidence freshness**/Platform-security/compatibility before changing authority.
- Recovery trust is never escrowed by PREPARE, routing or earlier validation. Same-session COMMIT and post-grace new-GameSession attachment both revalidate trust and the `<=5s` trust/revocation-evidence ceiling.
- A reconnect secret, recovery JWT or prepared successor secret alone cannot preempt a healthy current binding. A healthy current playable controller returns `RECOVERY_HEALTHY_CONTROLLER_PRESENT` / `CHARACTER_ALREADY_ACTIVE` before any generic no-target fallback.
- Failed stale COMMIT is candidate-local and non-mutating; it preserves whatever authority state is actually current and never revives a superseded predecessor.
- `RECONNECT_PREPARED_EXPIRED` is distinct from `RECONNECT_GRACE_EXPIRED`; only a fresh PREPARE after current-state/proof evaluation can retry while grace remains valid.
- After healthy-controller conflict is excluded, authoritative state matching neither legal recovery transition maps to `RECOVERY_TARGET_NOT_ELIGIBLE`; no nonce/authority mutation and no recovery-to-fresh-entry reinterpretation.
- Both signed v1 profiles use trusted-server `nbf`/expiry equations including `now + 5s >= nbf` and `now - 5s < exp`.
- Recovery `compatibility_revision` is a signed mandatory current compatibility requirement and is revalidated at authority-changing boundaries.
- Production liveness cadence/hysteresis, CharacterLease timing and hard resource limits remain deliberately deferred to measured implementation evidence gates.

## Accepted candidate semantics

### Authority and lifecycle

- [x] Separate AccountPresenceClaim, CharacterLease, GameSession, TransportBinding and RuntimeScopeAuthority.
- [x] Atomic fresh admission with no externally visible partial authority.
- [x] Account-global one-character exclusion and healthy-incumbent protection.
- [x] GameSession terminality does not release mandatory actor presence.
- [x] Post-grace same-character recovery may attach a fresh GameSession to the same `PRESENT_UNCONTROLLED` actor without reset, respawn, teleport or heal.
- [x] Channel/Instance continuity classes preserve explicit ownership/fencing boundaries.

### Grant and reconnect security

- [x] Strict, mutually exclusive fresh-entry and reauthenticated-recovery JWS profiles.
- [x] Dedicated issuer/audience/type/purpose/key policy, bounded parser/input sizes and no token-directed key discovery.
- [x] AdmissionAttemptRef and recovery `attempt_ref` are producer idempotency/correlation, distinct from game consume nonces.
- [x] `ADMISSION_ATTEMPT_RECONCILIATION_REQUIRED` has bounded same-ref-only reconciliation, deterministic retirement and no blind second capability.
- [x] `RECOVERY_ATTEMPT_RECONCILIATION_REQUIRED` has bounded same-recovery-ref-only reconciliation, deterministic retirement, no blind second recovery capability and no fresh-entry reinterpretation.
- [x] Platform account-security generation/revocation freshness remains bounded before new admission/recovery without becoming post-admission gameplay authority.
- [x] Signing-key/profile trust/revocation evidence is authenticated and no older than five seconds for both grant profiles at authority-changing validation boundaries.
- [x] Stale/unavailable/unprovable trust evidence uses purpose-specific `*_GRANT_SECURITY_EVIDENCE_STALE`; fresh explicit unknown/revoked/not-trusted trust state uses purpose-specific `*_GRANT_AUTHENTICATION_FAILED`.
- [x] Fresh-entry final admission revalidates signing-key/profile trust and trust-evidence freshness atomically before GrantNonce/session authority commit.
- [x] Recovery same-session COMMIT and post-grace new-session commit revalidate key/profile trust plus `<=5s` trust/revocation-evidence freshness.
- [x] Fresh-entry route/runtime owner-generation binding fails closed on stale owner generation.
- [x] Recovery `compatibility_revision` is validated/revalidated against current protocol/runtime/content/ruleset/session compatibility on both recovery paths.
- [x] 32-byte game-domain reconnect proof material and one-winner PREPARE/COMMIT generation transition.
- [x] COMMIT-time authority/security/key-profile-trust-freshness/compatibility revalidation closes PREPARE→COMMIT TOCTOU.
- [x] Failed COMMIT preserves actual current authority and never rolls back/revives a predecessor.
- [x] Healthy binding cannot be evicted by bearer reconnect/recovery proof.
- [x] Accepted 2s loss / 5s concrete transport cleanup / 15s same-session grace / one 4s protection activation per eligible ControlLossEpoch remain binding.

### Failure and decision discipline

- [x] Stable `FS-RECOVERY-GRANT-ISSUANCE-AMBIGUITY`, `FS-ADMISSION-GRANT-REPLAY`, `FS-RECONNECT-CREDENTIAL-REPLAY` and `FS-RECONNECT-PREPARE-COMMIT-ELIGIBILITY-CHANGE` scenarios exist.
- [x] `FS-KEY-ROTATION` now freezes the grant-profile trust/revocation freshness ceiling and distinguishes stale/unprovable evidence from fresh explicit revocation.
- [x] Every FND-04 cross-component error has stable internal code/category, disposition, exact retry authority, mutation/idempotency outcome and bounded public class.
- [x] `RECONNECT_PREPARED_EXPIRED`, `RECOVERY_TARGET_NOT_ELIGIBLE` and `RECOVERY_HEALTHY_CONTROLLER_PRESENT` remain distinct and ordered correctly.
- [x] Recovery validator failures never inherit fresh-entry Gateway actions.
- [x] Fresh-entry/recovery fixtures use exact accepted `nbf` boundary and exact signing-key/profile trust evidence `age=5s` vs `>5s`/unavailable/revoked cases.
- [x] Canonical Decision Timing records every mandatory dimension and defers evidence-sensitive implementation values explicitly.

## Review repair history

Historical heads are evidence of review provenance only and cannot satisfy terminal exact-head gates after later edits.

1. Early reviews: PREPARE→COMMIT stale-authority races, missing Decision Timing, UUIDv7 validation and incomplete failure progression repaired.
2. `a9634cce0599fb21c16e1ace1ea83c20d3cdb75a`: reciprocal refinement linkage and recovery-grant progression repaired.
3. `66d4738131ddd7f1ebb9a0ac1b5a25d70edfd0cb`: complete six-dimension canonical Decision Timing matrix added.
4. `9907e4be8c165c4a4ff571aa8d9e180bcd09ae50`: `ADMISSION_GRANT_NOT_YET_VALID` / `RECOVERY_GRANT_NOT_YET_VALID` progression added.
5. `cf6b13df7a160e186f6171455cfa20ea77b5f91d`: trusted-time `nbf` fixtures aligned with verifier skew equations.
6. `10e2ba70f21401327f83814112b721959713c7d6`: recovery `compatibility_revision` made a mandatory current compatibility constraint.
7. `445302861ff07670c1b3ccf7ba617d37587279bd`: canonical/main COMMIT checklists synchronized with compatibility requirement.
8. `ad8eb45b899bf326483603d94beea0d505ccc8c9`: failed COMMIT no-predecessor-revival semantics and shared compatibility-race evidence repaired.
9. `9a7ace36d1716f10c5edb362c76c5e461c1fdb0c`: recovery key/profile revocation revalidation added to same-session COMMIT.
10. `77d619b509cd8775ad7d21fc49b4879a2aa17422`: post-grace trust revalidation, exact revocation result, `RECOVERY_TARGET_NOT_ELIGIBLE` and `RECONNECT_PREPARED_EXPIRED` repaired.
11. Repair cycle 12 consolidated those four edge cases across recovery profile, canonical refinement, main contract and shared catalogue.
12. `c366bf1e9fda6b3f9525fc9dd3bf8ce86541b0df`: terminal review found missing admission-attempt reconciliation progression and healthy-controller/no-target dispatch conflict.
13. `49d7bf88b0272ba9708a6a9a8f7c687d8cc70fab`: repair cycle 13 added `ADMISSION_ATTEMPT_RECONCILIATION_REQUIRED` and ordered healthy-controller recovery dispatch. Exact-head Agent governance `31305568013`, Dependency review `31305568026`, CodeQL `31305568015` and self-audit passed; all prior threads were resolved. Replacement terminal Codex review nevertheless found one new P1 and one P2, invalidating merge readiness.
14. Exceptional safety repair cycle 14 closes the `49d7bf...` findings: P1 unbounded signing-key/profile revocation-state freshness is replaced by authenticated `<=5s` trust/revocation evidence for **both** grant profiles with purpose-specific stale-evidence vs fresh-explicit-revocation outcomes and commit-time revalidation; P2 ambiguous recovery-grant issuance now has complete `RECOVERY_ATTEMPT_RECONCILIATION_REQUIRED` progression, deterministic retirement/proof and a dedicated shared failure scenario. Both profiles, canonical refinement, main contract, shared failure catalogue and programme status are synchronized.

## Review budget / Codex usage policy

The independent Codex reviewer is a terminal assurance gate, not an iterative repair loop.

- Iterative repairs use exact-diff review, full assistant self-audit, repository governance and exact-head CI.
- The original terminal-review budget allowed one replacement review after a material terminal finding.
- That replacement review on `49d7bf...` found a new **P1 security issue**. The budget cap is therefore narrowly superseded for safety: it cannot be used as justification to merge a known P1.
- Repair cycle 14 permits **one exceptional final independent exact-head safety review** after the repaired head is frozen and local/CI/thread gates are green.
- Do not invoke Codex again for unchanged status, polling or cosmetic confirmation. A further material finding blocks merge and must not be ignored.

## Governance acceptance

- [x] PR title is within repository governance limit.
- [x] Scope remains exactly seven declared documentation paths.
- [x] No runtime/protocol codec/persistence schema/Platform write/key deployment/production activation is introduced.
- [ ] Freeze one final exact head after this repair-cycle-14 task synchronization commit.
- [ ] Full exact-head seven-path architecture/security review reports zero material conflicts.
- [ ] Exact-head Agent governance, Dependency review and CodeQL all pass.
- [ ] One exceptional final independent exact-head Codex architecture/security review reports zero material findings.
- [ ] Zero unresolved review threads.
- [ ] Squash merge only with expected-head protection.
- [ ] Archive/release ownership in a separate closeout PR after merge.

## Validation

### Component / integration / E2E

`NOT_APPLICABLE` for this architecture-only delivery. Future implementation evidence is specified by the contracts and must be provided by separately authorized implementation packages.

### Historical exact-head evidence

- `4bb02e5b...`: historical; former PR-title governance issue.
- `6ea04ac8...`: historical; later material findings invalidated readiness.
- `a9634cce...`, `66d47381...`, `9907e4be...`, `cf6b13df...`, `10e2ba70...`, `44530286...`, `ad8eb45b...`, `9a7ace36...`, `77d619b5...`: historical reviewed repair generations.
- `c366bf1e9fda6b3f9525fc9dd3bf8ce86541b0df`: historical; CI/self-audit green but terminal review found two P2 findings.
- `49d7bf88b0272ba9708a6a9a8f7c687d8cc70fab`: historical; exact-head Agent governance/Dependency review/CodeQL/self-audit green and repair-cycle-13 threads resolved, but replacement terminal review found P1 unbounded signing-key/profile revocation freshness and P2 missing ambiguous recovery-issuance progression.

### Current generation

- final head: pending after this task-synchronization commit;
- exact-head CI: pending;
- exact-head self-audit: pending;
- exceptional final independent Codex safety audit: pending after local gates pass;
- unresolved material findings: repair-cycle-14 P1/P2 are repaired in candidate docs; pending exact-head validation.

## PR and closeout

- delivery PR: #109;
- merge policy: squash only after all exact-head gates are green and the head is unchanged;
- lifecycle ownership release: separate closeout PR following the FND-03 precedent;
- runtime/Platform/persistence implementation remains outside this task.

## Context checkpoint

```yaml
last_progress: Replacement terminal Codex review on 49d7bf88b0272ba9708a6a9a8f7c687d8cc70fab found P1 unbounded signing-key/profile revocation-state freshness and P2 missing ambiguous recovery-grant issuance progression. Exceptional safety repair cycle 14 now freezes authenticated signing-key/profile trust/revocation evidence age <=5s for both grant profiles, stale/unavailable/unprovable evidence -> purpose-specific *_GRANT_SECURITY_EVIDENCE_STALE, fresh explicit unknown/revoked/not-trusted -> purpose-specific *_GRANT_AUTHENTICATION_FAILED, with atomic fresh-admission/same-session/post-grace revalidation and no nonce/authority mutation on failure. It also defines RECOVERY_ATTEMPT_RECONCILIATION_REQUIRED with same-recovery-attempt-ref reconciliation only, deterministic retirement/proof before any new recovery attempt, TEMPORARILY_UNAVAILABLE public mapping and no gameplay/fresh-entry authority. Profiles, canonical refinement, main contract, shared failure catalogue and programme status are synchronized.
status: validating
branch: docs/OTV2-20260808-fnd04-session-admission-final
pr: 109
head_sha: null
final_head_sha: null
final_head_frozen_at: null
ci_check_generation: exceptional-safety-repair-14
repair_cycles_for_current_gate: 14
owner_action_required: null
blocker: null
next_action: treat this task-synchronization commit as the new frozen exact-head candidate; verify exact seven-file scope, live main and repair-cycle-14 delta; run fresh exact-head Agent governance, Dependency review and CodeQL; perform full seven-path self-audit; reply to and resolve the two 49d7 Codex threads only with exact-head evidence; then invoke one exceptional final independent Codex safety review. Squash merge only if it reports zero material findings and head/main/CI/thread state remains unchanged. After delivery merge, perform separate two-path lifecycle closeout and release ownership.
```