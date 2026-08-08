# OTV2-20260808-fnd04-final-contract

```yaml
task_id: OTV2-20260808-fnd04-final-contract
title: Finalize FND-04 identity session admission and lease contract
mode: CONTRACT
status: validating
repository: blakinio/Oteryn-v2
base_branch: main
branch: docs/OTV2-20260808-fnd04-final-contract
pr: 110
base_sha: 27f7f647f04e3b1a4151f9b124401986910f03d8
head_sha: null
final_head_sha: null
final_head_frozen_at: null
owner: GPT-5.6 Sol architecture continuation session
created_at: 2026-08-08T21:29:00+02:00
updated_at: 2026-08-08T21:46:00+02:00
execution_budget_minutes: 60
large_budget_reason: null
owned_paths:
  - docs/agents/tasks/active/OTV2-20260808-fnd04-final-contract.md
  - docs/architecture/FND-04_IDENTITY_GAME_SESSION_ADMISSION_LEASE_CONTRACT.md
  - docs/contracts/FOUNDATION_FAILURE_SCENARIOS.md
public_contracts:
  - docs/architecture/FND-04_IDENTITY_GAME_SESSION_ADMISSION_LEASE_CONTRACT.md
  - docs/contracts/FOUNDATION_FAILURE_SCENARIOS.md
depends_on:
  - docs/architecture/FND-04_SESSION_ADMISSION_LEASE_ANALYSIS_BASELINE.md
  - docs/architecture/FND-04_PLATFORM_PRE_ADMISSION_RECONCILIATION_REFINEMENT.md
  - docs/architecture/FND-ID-01_FOUNDATION_IDENTIFIER_CONTRACT.md
  - docs/architecture/FND-02_PROTOCOL_OTERYN_V1_CONTRACT.md
  - docs/architecture/FND-03_RUNTIME_EXECUTION_CONTRACT.md
  - docs/architecture/DISCONNECT_REENTRY_PVE_PROTECTION_OWNER_DECISION.md
  - docs/contracts/FOUNDATION_ERROR_VOCABULARY.md
  - docs/contracts/FOUNDATION_FAILURE_SCENARIOS.md
  - blakinio/Oteryn-Platform@8e2514b8721d385b626ead7ffa47fc74067b0a0b read-only integration evidence
blocks:
  - production admission GameSession reconnect takeover and CharacterLease implementation claims
  - FND-04-dependent persistence and first native vertical-slice admission work
cross_repository_coordination_id: OTV2-NATIVE-FOUNDATION
external_repositories:
  - blakinio/Oteryn-Platform (read-only evidence only)
```

## Outcome

Deliver one canonical architecture-only FND-04 semantic contract covering fresh admission, account-global presence exclusion, CharacterLease fencing, canonical GameSession lifecycle, transport reconnect/recovery, duplicate-login/takeover, runtime-placement validation and Channel/Instance handoff continuity.

The package deliberately gates cryptographic/profile products and security/liveness/lease numeric values behind bounded preimplementation evidence instead of guessing them in architecture.

## Architecture and source of truth

- **PROVEN:** Oteryn-v2 `main` at task start and current recheck is `27f7f647f04e3b1a4151f9b124401986910f03d8`.
- **PROVEN:** the repaired FND-04 analysis is archived and ownership released; canonical inputs are `FND-04_SESSION_ADMISSION_LEASE_ANALYSIS_BASELINE.md` plus `FND-04_PLATFORM_PRE_ADMISSION_RECONCILIATION_REFINEMENT.md`.
- **PROVEN:** current Platform `main` rechecked during validation remains `8e2514b8721d385b626ead7ffa47fc74067b0a0b`; pre-admission/runtime-status contract blobs remain `a7a98b943c528b9f21c0cdc2ee90b308045706f8` and `5e45a4318716b62d53fd8bdf67b3b55676286ad1`.
- **PROVEN:** Platform remains reusable account/security and bounded authorization authority; Oteryn-v2 remains final admission, current placement, account presence, CharacterLease/fencing and GameSession authority.
- **DERIVED:** semantic FND-04 can close without inventing benchmark/security-sensitive values if implementation remains blocked on explicit versioned security/interchange and liveness/lease/abuse profiles plus DUR-02 fencing semantics.
- **UNKNOWN:** exact cryptographic container/algorithm/library/KMS, Platform recovery producer implementation, security-projection transport, liveness/lease numeric values and PostgreSQL physical schema.
- **CONFLICT:** none open after author review repairs below.

## Acceptance criteria

- [x] Distinct Platform security/authorization, AccountPresenceClaim, CharacterLease, GameSession, TransportBinding and RuntimeScopeAuthority layers.
- [x] One authoritative fresh-admission linearization point; canonical GameSessionId exists only after successful commit.
- [x] Signed short-lived `FRESH_ENTRY` PreAdmissionGrant plus authoritative one-time GrantNonce consumption; AdmissionAttemptRef remains separate producer idempotency/correlation state.
- [x] Post-issuance Platform security generation/revocation freshness is explicit and fail-closed without granting Platform post-admission gameplay authority.
- [x] Fresh-entry route/runtime observation/ownership-generation applicability and stale-owner rejection are explicit.
- [x] Primary same-GameSession reconnect uses rotating opaque game proof and atomic newer connection_generation winner.
- [x] Lost rebind response cannot revive predecessor proof; accepted fallback is a distinct future Platform `RecoveryAuthorizationGrant` with `RECOVER_EXISTING_CONTROL` semantics.
- [x] `PreAdmissionGrant` and `RecoveryAuthorizationGrant` are non-confusable credential types; the current Platform pre-admission contract remains intact.
- [x] Session liveness distinguishes `CONTROL_SUSPECT` from declared loss and freezes the accepted `T0+2s`, `T0+5s`, `control_loss+15s`, one-shot 4s protection composition.
- [x] Post-grace same-character recovery may attach a fresh GameSessionId to the exact existing PRESENT_UNCONTROLLED actor without reset/duplication; another CharacterId remains blocked.
- [x] Healthy incumbent duplicate-login protection and logout-eligible fenced takeover are explicit.
- [x] CharacterLease is actor/runtime-writer authority, can outlive a GameSession for mandatory uncontrolled actor presence, and never grants player command authority by itself.
- [x] Lease acquisition, renewal, uncertainty, replacement and release semantics are frozen without physical schema/TTL guessing.
- [x] Same-GameSession recovery across GameNode replacement requires reconstructable FND-02/FND-04 state; otherwise safe fresh-session recovery.
- [x] Channel↔Instance continuous handoff may preserve GameSessionId; Channel→Channel uses fresh route/PreAdmissionGrant/GameSessionId while account presence stays continuous.
- [x] Shared catalogue adds `FS-ADMISSION-GRANT-REPLAY` and `FS-RECONNECT-CREDENTIAL-REPLAY`.
- [x] Stable internal admission/reconnect/recovery/takeover error vocabulary maps to foundation categories and requires redacted public presentation.
- [x] Cross-language authorization/security profile, liveness/lease/abuse parameter profile and DUR-02 are explicit preimplementation gates.
- [x] Platform remained read-only; no Rust/runtime/protocol-schema/persistence/deployment/production implementation was introduced.
- [ ] Full exact candidate diff review and independent architecture/security audit have zero material findings.
- [ ] Exact-head required CI passes with zero unresolved review threads before merge.

## Excluded scope

- no Rust code, listener/codec/schema registration, database migration or runtime implementation;
- no Oteryn-Platform write;
- no production keys/credentials/traffic/sessions/deployment;
- no cryptographic library/KMS/HSM vendor choice;
- no guessed grant TTL/skew/key-cache, reconnect-proof size, liveness cadence/hysteresis, lease TTL/renewal/safety-margin or rate-limit values;
- no DUR-02 physical schema/transaction choice;
- no combat/logout formula redesign or final client UX wording.

## Implementation / findings

Candidate contract: `docs/architecture/FND-04_IDENTITY_GAME_SESSION_ADMISSION_LEASE_CONTRACT.md`.

Shared failure catalogue adds exactly the two replay scenarios recommended by the repaired analysis.

### Author-review repairs before freeze

1. **Credential-type confusion:** initial draft placed fresh entry and reauthenticated recovery under one PreAdmissionGrant-purpose section. Repaired so `PreAdmissionGrant = FRESH_ENTRY` only; recovery is a distinct future `RecoveryAuthorizationGrant`, with mutually exclusive validation and a separately authorized Platform producer/profile requirement.
2. **Liveness state gap:** initial draft transitioned from ACTIVE directly to CONTROL_LOST_GRACE on insufficient evidence, which could collapse the accepted two-second declaration window. Repaired with explicit `CONTROL_SUSPECT`; grace begins only at `control_loss_declared_at`.
3. **Lease/session lifetime coupling:** initial wording could imply an ACTIVE GameSession was required to renew CharacterLease. Repaired so lease renewal follows current authoritative actor/runtime writer lifecycle and may continue for a mandatory `PRESENT_UNCONTROLLED` actor, while player-originated control still requires an active current GameSession/TransportBinding.
4. **Recovery error ambiguity:** fresh-entry credential failure and recovery credential failure now have distinct internal symbolic codes.

No author-review finding remains open after these repairs.

## Validation

### Focused

- semantic cross-check: accepted FND-ID-01/FND-02/FND-03 + repaired FND-04 analysis + current pinned Platform contracts
- result: `PASS` after four author-review repairs; independent exact-head audit still required
- current Platform revision recheck: `8e2514b8721d385b626ead7ffa47fc74067b0a0b`, unchanged from pin
- current Oteryn-v2 base recheck: `27f7f647f04e3b1a4151f9b124401986910f03d8`, unchanged

### Component/integration

- result: `NOT_APPLICABLE` — architecture/documentation only; no executable component changed

### E2E

- result: `NOT_APPLICABLE` — this task introduces no executable admission/session capability; section 32 defines future implementation E2E obligations

### Exact-head CI

- final head: pending final task synchronization
- trigger source: pull_request
- workflow/run/job: pending
- runner assignment: pending
- classification: pending
- result: pending

## Independent audit

- exact head: pending final task synchronization
- method/auditor: independent exact-diff architecture/security review against both canonical analysis documents, accepted FND-ID/FND-02/FND-03 authority and pinned Platform contracts
- material findings: pending
- verdict: pending

## PR and closeout

- delivery PR: #110
- changed-file scope: exactly three declared documentation paths
- unresolved review threads: pending
- related PRs: #107/#108 are canonical repaired-analysis dependency/closeout; no competing open FND-04 delivery existed at task start
- merge: forbidden until exact-head audit/CI/review gates pass
- post-merge task archive/ownership release: required

## Context checkpoint

```yaml
invocation_started_at: 2026-08-08T21:29:00+02:00
last_progress_at: 2026-08-08T21:46:00+02:00
last_progress: Candidate FND-04 contract and two replay failure scenarios are complete; four material author-review ambiguities were repaired before final freeze.
status: validating
branch: docs/OTV2-20260808-fnd04-final-contract
head_sha: null
pr: 110
final_head_sha: null
final_head_frozen_at: null
ci_trigger_source: pull_request
ci_check_generation: pending-final-head
ci_checks_for_current_head: 0
ci_run_ids: []
ci_job_ids: []
runner_assignment_state: unknown
terminal_ci_wait_started_at: null
terminal_ci_checks_for_current_generation: 0
unchanged_state_checks: 0
identical_failure_retries: 0
repair_cycles_for_current_gate: 1
ci_recovery_actions_for_current_head: 0
stall_warnings: 0
owner_action_required: null
blocker: null
next_action: Inspect the exact three-path PR #110 diff, perform independent architecture/security audit, repair any finding, then freeze the resulting head for required CI.
```
