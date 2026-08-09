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
updated_at: 2026-08-09T10:19:00+02:00
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

Deliver the complete architecture-only FND-04 contract needed before native identity/admission/reconnect/lease implementation can be designed without guessing authority, replay, reconnect, recovery, timing, compatibility or lease security semantics.

Acceptance completes the FND-04 architecture gate only. It does not authorize runtime, Platform, persistence, protocol-codec, key-management, deployment or production implementation.

## Architecture and source of truth

- FND-04 analysis plus the current-Platform reconciliation refinement are canonical on `main` after PRs #104/#107 and lifecycle closeout #108.
- Duplicate/superseded PRs #106 and #110 are closed unmerged and contribute no authority.
- External Platform evidence remains pinned read-only at `blakinio/Oteryn-Platform@216f5b2817e9d102337608609e344518512c2a0d`.
- Platform Identity/Gateway authorizes bounded attempts; Oteryn-v2 remains final GameSession, AccountPresenceClaim, CharacterLease and runtime-control authority.
- FND-02 owns bootstrap, GameSession issuance boundary, connection generation and command/reconciliation semantics; FND-03 owns runtime ordering/fencing/time execution.
- Fresh entry and reauthenticated existing-actor recovery use mutually exclusive signed profiles. Both use fully specified JOSE `alg=Ed25519`; deprecated polymorphic `EdDSA` fallback is rejected.
- Oteryn-owned grant identities follow FND-ID UUIDv7/RFC-variant semantics; Platform-owned AccountId is not silently redefined.
- Reconnect PREPARE is a candidate reservation only. COMMIT atomically revalidates current incumbent/session/presence/lease/runtime/reconciliation eligibility and, for recovery grants, current token/nonce/recovery-key-profile trust/Platform-security/compatibility validity before changing authority.
- Recovery signing-key/profile trust is never escrowed by PREPARE, routing or earlier recovery validation. It is revalidated at the actual authority-changing boundary for both same-session COMMIT and post-grace new-GameSession attachment. Emergency revocation fails as `RECOVERY_GRANT_AUTHENTICATION_FAILED`, without RecoveryGrantNonce consumption or authority mutation.
- A reconnect secret, recovery JWT or prepared successor secret alone cannot preempt a healthy current binding.
- A failed stale COMMIT candidate never rolls authority back to its PREPARE predecessor. It mutates no gameplay authority and preserves whatever GameSession/TransportBinding/lease/runtime ownership state is actually current at revalidation; a predecessor already fenced, superseded, handed off or terminalized is never revived.
- Prepared-candidate expiry is distinct from same-session grace expiry. `RECONNECT_PREPARED_EXPIRED` terminalizes only the prepared candidate; a new PREPARE is allowed only after fresh current-state/proof evaluation while same-session grace and authority eligibility still permit it. `RECONNECT_GRACE_EXPIRED` ends same-session retry eligibility.
- A valid recovery grant with no legal recovery target, including an actor already legally `ABSENT`, fails as `RECOVERY_TARGET_NOT_ELIGIBLE`: terminal for that recovery transition, no RecoveryGrantNonce consumption, no authority mutation and no recovery-to-fresh-entry reinterpretation.
- `FND-04_HEALTHY_BINDING_REBIND_SECURITY_REFINEMENT.md` is reciprocally linked and canonical for healthy-binding/rebind semantics, the complete mandatory Decision Timing matrix, FND-04 failure progression and the PREPARE→COMMIT eligibility-change scenario.
- Both signed v1 profiles use the same trusted-server time equations for the accepted clock-skew window, including `now + 5s >= nbf` and `now - 5s < exp`; FND-04 shorthand such as `after nbf` means entry into that accepted skew window, never literal `now >= nbf`.
- Recovery `compatibility_revision` is a signed required compatibility constraint, not descriptive metadata: current Oteryn-v2 protocol/runtime/content/ruleset/session compatibility must support it before either same-session or post-grace recovery, and it is revalidated at the authority-changing boundary. Unsupported or superseded revision maps to `RECOVERY_GRANT_REVISION_UNSUPPORTED` without nonce consumption or authority mutation.
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
- [x] AdmissionAttemptRef is distinct from GrantNonce/RecoveryGrantNonce.
- [x] Platform account-security generation/revocation freshness is revalidated for new admission/recovery without becoming post-admission gameplay authority.
- [x] Fresh-entry route/runtime owner-generation binding fails closed on stale owner generation.
- [x] Recovery `compatibility_revision` is validated against current protocol/runtime/content/ruleset/session compatibility on both recovery paths and revalidated at the authority-changing commit boundary.
- [x] Recovery signing-key/profile trust/revocation is revalidated at same-session COMMIT and post-grace new-session commit; neither PREPARE nor earlier validation escrows continued trust.
- [x] Post-PREPARE or post-validation emergency key/profile revocation maps to `RECOVERY_GRANT_AUTHENTICATION_FAILED`, consumes no RecoveryGrantNonce, commits no candidate/new-session authority and preserves actual current authority.
- [x] Canonical refinement, main contract and recovery profile carry matching current recovery-key/profile trust and compatibility checks.
- [x] 32-byte game-domain reconnect proof material and one-winner PREPARE/COMMIT generation transition.
- [x] COMMIT-time authority/security/key-profile-trust/compatibility revalidation closes PREPARE→COMMIT TOCTOU.
- [x] Failed COMMIT is candidate-local and non-mutating: actual current authority remains current, including a newer fence/handoff/takeover/terminal state; PREPARE predecessor is never revived as rollback.
- [x] Healthy binding cannot be evicted by bearer reconnect/recovery proof; any future healthy migration needs separately current-generation-authorized semantics.
- [x] Accepted 2s loss / 5s concrete transport cleanup / 15s same-session grace / one 4s protection activation per eligible ControlLossEpoch remain binding.

### Failure and decision discipline

- [x] Stable `FS-ADMISSION-GRANT-REPLAY`, `FS-RECONNECT-CREDENTIAL-REPLAY` and `FS-RECONNECT-PREPARE-COMMIT-ELIGIBILITY-CHANGE` scenarios exist.
- [x] Shared `FS-RECONNECT-PREPARE-COMMIT-ELIGIBILITY-CHANGE` explicitly requires post-PREPARE key/profile revocation to return `RECOVERY_GRANT_AUTHENTICATION_FAILED`, with no RecoveryGrantNonce consumption/no authority mutation and preservation of actual current authority; compatibility drift remains `RECOVERY_GRANT_REVISION_UNSUPPORTED`.
- [x] Every FND-04 cross-component error has stable internal code/category, RETRYABLE/TERMINAL/SECURITY_TERMINAL disposition, exact retry authority, mutation/idempotency outcome and bounded public class.
- [x] `RECONNECT_PREPARED_EXPIRED` is `TIMEOUT` + `TERMINAL` for the expired candidate, allows only a newly evaluated PREPARE while same-session grace/current eligibility still permit, and is distinct from `RECONNECT_GRACE_EXPIRED`.
- [x] `RECOVERY_TARGET_NOT_ELIGIBLE` is `SESSION_REJECTED` + `TERMINAL`, consumes no RecoveryGrantNonce, mutates no authority and never turns the recovery grant into fresh-entry authority.
- [x] Recovery validator failures have recovery-specific malformed/authentication/not-yet-valid/expiry/replay/security-revocation/stale-security/unsupported-revision/no-target progression and never inherit fresh-entry Gateway actions.
- [x] Recovery key/profile trust revocation maps to `RECOVERY_GRANT_AUTHENTICATION_FAILED`; unsupported/superseded recovery `compatibility_revision` maps to `RECOVERY_GRANT_REVISION_UNSUPPORTED`; both fail before nonce consumption/authority mutation when discovered at an authority-changing boundary.
- [x] `ADMISSION_GRANT_NOT_YET_VALID` and `RECOVERY_GRANT_NOT_YET_VALID` explicitly allow only bounded same-unconsumed-grant retry once trusted server time enters the accepted `nbf` skew window while all other purpose-specific bindings remain valid; neither consumes its nonce or mutates authority.
- [x] Fresh-entry and recovery fixtures use the exact accepted boundary `now + 5s >= nbf`; `now + 5s < nbf` is the not-yet-valid case.
- [x] The canonical Decision Timing matrix answers for every material row: decide now/defer, exact blocked downstream work, what becomes harder or impossible later, evidence required to supersede, and what is deliberately not decided here.
- [x] Deferred liveness, lease, resource, persistence and implementation/vendor choices are explicitly blocked on their named evidence rather than library defaults.

## Review repair history

Historical heads are evidence of the review process only and cannot satisfy terminal exact-head gates after later edits.

1. Early Codex reviews found PREPARE→COMMIT stale-authority races, missing Decision Timing, missing UUIDv7 validation and incomplete Foundation Error Vocabulary progression. These were repaired.
2. Exact-head Codex review on `a9634cce0599fb21c16e1ace1ea83c20d3cdb75a` found P1 missing reciprocal discovery/normative linkage and P2 incomplete recovery-grant progression. Both were repaired and the main contract/current-status/refinement were harmonized.
3. Exact-head Codex review on `66d4738131ddd7f1ebb9a0ac1b5a25d70edfd0cb` found one P1: canonical Decision Timing did not include harder/impossible-later and deliberately-undecided dimensions for every row. The six-column canonical matrix repaired this.
4. Exact-head Codex review on `9907e4be8c165c4a4ff571aa8d9e180bcd09ae50` found one P2: grant profiles validate `nbf`, but canonical failure progression did not define an otherwise-valid-yet-not-active outcome. The refinement added `ADMISSION_GRANT_NOT_YET_VALID` and `RECOVERY_GRANT_NOT_YET_VALID` with bounded accepted-window same-grant retry, no nonce consumption, no authority mutation and `TEMPORARILY_UNAVAILABLE` public mapping.
5. Exact-head Codex review on `cf6b13df7a160e186f6171455cfa20ea77b5f91d` found one P2: canonical fixtures still said retry only after literal `nbf`, while fresh-entry already accepted the five-second verifier-skew window and recovery lacked explicit trusted-`now` equations. Recovery v1 now uses the same explicit equations as fresh-entry, canonical FND-04 defines `after/post-nbf` as entry into the accepted skew window, and fixtures test `now + 5s < nbf` versus `now + 5s >= nbf`.
6. Exact-head Codex review on `10e2ba70f21401327f83814112b721959713c7d6` found one P2: recovery `compatibility_revision` was syntactically validated but not required to match current runtime/content/session compatibility. Recovery v1 now treats the signed revision as a mandatory current compatibility constraint on both same-session and post-grace recovery, revalidates it at COMMIT for prepared same-session recovery, maps mismatch to `RECOVERY_GRANT_REVISION_UNSUPPORTED` and requires negative/change-after-PREPARE fixtures.
7. Exact-head Codex review on `445302861ff07670c1b3ccf7ba617d37587279bd` found one P2: recovery profile had the COMMIT-time compatibility revalidation, but canonical refinement §3.2 and main §14.3 omitted it. Both architecture-level checklists now explicitly require the current signed compatibility revision at COMMIT, map unsupported/superseded/change-after-PREPARE to `RECOVERY_GRANT_REVISION_UNSUPPORTED`, preserve nonce/current authority on failure and carry matching fault evidence.
8. Exact-head Codex review on `ad8eb45b899bf326483603d94beea0d505ccc8c9` found P1 that failed COMMIT wording could incorrectly restore PREPARE predecessor after another valid fence/handoff/terminal transition, and P2 that the shared failure catalogue did not explicitly include signed recovery compatibility drift. Main contract, canonical refinement and recovery profile now define failed stale candidate COMMIT as non-mutating with no predecessor rollback/revival; crash/lost-response paths reconcile actual current authority. The shared failure scenario now covers compatibility drift and superseding authority transitions.
9. Exact-head Codex review on `9a7ace36d1716f10c5edb362c76c5e461c1fdb0c` found one P1: the canonical refinement required current recovery signing-key/profile revocation trust at COMMIT, but the recovery profile §9.1 and main §14.3 did not mirror it. Recovery profile, main contract and shared failure catalogue now require post-PREPARE emergency key/profile revocation to reject COMMIT as `RECOVERY_GRANT_AUTHENTICATION_FAILED`, preserve current authority, consume no RecoveryGrantNonce and carry a dedicated change-after-PREPARE fixture.
10. Exact-head Codex review on `77d619b509cd8775ad7d21fc49b4879a2aa17422` found four final material edge cases: P1 missing post-grace new-GameSession key/profile trust revalidation; P2 shared failure catalogue not naming exact `RECOVERY_GRANT_AUTHENTICATION_FAILED`; P2 no canonical no-legal-recovery-target progression; and P2 no prepared-transition-expiry progression distinct from grace expiry. Repair cycle 12 closes all four across recovery profile, canonical refinement, main contract and shared failure catalogue. The repair introduces `RECOVERY_TARGET_NOT_ELIGIBLE` and `RECONNECT_PREPARED_EXPIRED` with complete Foundation Error Vocabulary progression and fixtures.

## Review budget / Codex usage policy

The independent Codex reviewer is a **terminal assurance gate, not an iterative repair loop**.

- Iterative architecture/security repair generations use local exact-diff review, full assistant self-audit, repository governance and exact-head CI.
- Do not invoke `@codex review` merely after every small repair, for status polling, or to repeat an unchanged result.
- Invoke one independent Codex architecture/security review only after the delivery head is frozen and the local audit/CI/thread checks indicate it is ready to merge.
- If that final Codex review finds a material issue, repair the issue, restart exact-head CI/self-audit, and permit at most one further terminal Codex review on the newly frozen repaired head.
- This execution-budget rule reduces external reviewer consumption without weakening the requirement that the final material delivery receive independent review before merge.

## Governance acceptance

- [x] PR title is within repository governance limit.
- [x] Scope remains exactly seven declared documentation paths.
- [x] No runtime/protocol codec/persistence schema/Platform write/key deployment/production activation is introduced.
- [ ] Freeze one final exact head after this task-synchronization commit.
- [ ] Full exact-head seven-path architecture/security review reports zero material conflicts.
- [ ] Exact-head Agent governance, Dependency review and CodeQL all pass.
- [ ] One final independent exact-head Codex architecture/security review reports zero material findings.
- [ ] Zero unresolved review threads.
- [ ] Squash merge only with expected-head protection.
- [ ] Archive/release ownership in a separate closeout PR after merge.

## Validation

### Component / integration / E2E

`NOT_APPLICABLE` for this architecture-only delivery. Future implementation evidence is specified by the contracts and must be provided by separately authorized implementation packages.

### Historical exact-head evidence

- `4bb02e5b...`: historical; governance failed only on former overlong PR title.
- `6ea04ac8...`: historical; later material Codex findings invalidated readiness.
- `a9634cce...`: historical; CI green but P1/P2 findings required repair.
- `66d4738131ddd7f1ebb9a0ac1b5a25d70edfd0cb`: historical; CI green but Codex found incomplete canonical Decision Timing dimensions.
- `9907e4be8c165c4a4ff571aa8d9e180bcd09ae50`: historical; CI green/self-audit green but Codex found missing `nbf`/not-yet-valid progression.
- `cf6b13df7a160e186f6171455cfa20ea77b5f91d`: historical; CI green/self-audit green but Codex found literal-`nbf` fixture wording inconsistent with the accepted five-second skew window and missing recovery trusted-time equations.
- `10e2ba70f21401327f83814112b721959713c7d6`: historical; exact-head CI green and all older threads resolved, but Codex found missing current compatibility validation for recovery `compatibility_revision`.
- `445302861ff07670c1b3ccf7ba617d37587279bd`: historical; exact-head CI green but Codex found architecture-level COMMIT checklists did not yet mirror the recovery-profile compatibility requirement.
- `ad8eb45b899bf326483603d94beea0d505ccc8c9`: historical; exact-head CI green but Codex found predecessor-rollback ambiguity plus missing shared compatibility-race scenario coverage.
- `9a7ace36d1716f10c5edb362c76c5e461c1fdb0c`: historical; exact-head CI green but Codex found missing recovery signing-key/profile revocation revalidation at COMMIT in profile/main normative checklists.
- `77d619b509cd8775ad7d21fc49b4879a2aa17422`: historical; exact-head CI/self-audit green but Codex found the four repair-cycle-12 edge cases listed above.

### Current generation

- final head: pending after this task-synchronization commit;
- exact-head CI: pending;
- exact-head self-audit: pending;
- final independent Codex audit: pending and budgeted for one terminal invocation after local gates pass;
- unresolved material findings: the four findings on `77d619...` are repaired; pending final exact-head audit.

## PR and closeout

- delivery PR: #109;
- merge policy: squash only after all exact-head gates are green and the head is unchanged;
- lifecycle ownership release: separate closeout PR following the FND-03 precedent;
- runtime/Platform/persistence implementation remains outside this task.

## Context checkpoint

```yaml
last_progress: Exact-head Codex review on 77d619b509cd8775ad7d21fc49b4879a2aa17422 found four final edge cases. Repair cycle 12 now requires current recovery signing-key/profile trust at the post-grace new-GameSession commit; pins post-PREPARE revocation to RECOVERY_GRANT_AUTHENTICATION_FAILED in the shared failure catalogue; defines RECOVERY_TARGET_NOT_ELIGIBLE for authoritative no-recovery-target state; and defines RECONNECT_PREPARED_EXPIRED separately from same-session grace expiry. Recovery profile, canonical refinement, main FND-04 contract and failure catalogue are synchronized. Codex usage is now budgeted as one final independent review only after local exact-head audit and CI are green.
status: validating
branch: docs/OTV2-20260808-fnd04-session-admission-final
pr: 109
head_sha: null
final_head_sha: null
final_head_frozen_at: null
ci_check_generation: post-final-recovery-edge-repair
repair_cycles_for_current_gate: 12
owner_action_required: null
blocker: null
next_action: treat this task-synchronization commit as the final exact-head candidate; verify seven-file scope, current main and pinned inputs; run exact-head Agent governance, Dependency review and CodeQL; perform full self-audit and resolve the four repaired review threads; then invoke exactly one terminal independent Codex review. Squash merge only if that review reports zero material findings and the head/main/CI/thread state remains unchanged.
```