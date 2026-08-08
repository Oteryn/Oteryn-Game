# OTV2-20260808-fnd04-final-contract

```yaml
task_id: OTV2-20260808-fnd04-final-contract
title: Finalize FND-04 identity session admission and lease contract
mode: CONTRACT
status: implementing
repository: blakinio/Oteryn-v2
base_branch: main
branch: docs/OTV2-20260808-fnd04-final-contract
pr: null
base_sha: 27f7f647f04e3b1a4151f9b124401986910f03d8
head_sha: null
final_head_sha: null
final_head_frozen_at: null
owner: GPT-5.6 Sol architecture continuation session
created_at: 2026-08-08T21:29:00+02:00
updated_at: 2026-08-08T21:29:00+02:00
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
  - docs/architecture/FND-ID-01_GAME_SESSION_ID_OWNER_ISSUER_BASELINE.md
  - docs/architecture/FND-ID-01_GAME_SESSION_RECONNECT_GENERATION_OWNER_BASELINE.md
  - docs/architecture/FND-ID-01_ACCOUNT_SINGLE_ONLINE_CHARACTER_OWNER_BASELINE.md
  - docs/architecture/FND-02_PROTOCOL_OTERYN_V1_CONTRACT.md
  - docs/architecture/FND-03_RUNTIME_EXECUTION_CONTRACT.md
  - docs/architecture/DISCONNECT_REENTRY_PVE_PROTECTION_OWNER_DECISION.md
  - docs/contracts/FOUNDATION_ERROR_VOCABULARY.md
  - docs/contracts/FOUNDATION_FAILURE_SCENARIOS.md
  - blakinio/Oteryn-Platform@8e2514b8721d385b626ead7ffa47fc74067b0a0b read-only current integration evidence
blocks:
  - production admission GameSession reconnect takeover and CharacterLease implementation claims
  - FND-04-dependent persistence and first native vertical-slice admission work
cross_repository_coordination_id: OTV2-NATIVE-FOUNDATION
external_repositories:
  - blakinio/Oteryn-Platform (read-only evidence only)
```

## Outcome

Deliver one canonical architecture-only FND-04 contract that converts the completed analysis and Platform reconciliation into implementation-blocking semantic authority for fresh admission, account-global presence exclusion, CharacterLease fencing, canonical GameSession lifecycle, transport reconnect/recovery, duplicate-login/takeover, runtime-placement validation and Channel/Instance handoff continuity.

The package must remain technology-disciplined: it may freeze semantic credential/security-profile requirements and evidence gates, but it must not invent cryptographic libraries, KMS/HSM products, PostgreSQL table layouts, Redis authority, runtime implementation or production configuration without evidence.

## Architecture and source of truth

- **PROVEN:** Oteryn-v2 `main` at task start is `27f7f647f04e3b1a4151f9b124401986910f03d8`; the repaired FND-04 analysis task is archived and ownership is released.
- **PROVEN:** there are no open Oteryn-v2 PRs at task start.
- **PROVEN:** the canonical FND-04 analysis consists of `FND-04_SESSION_ADMISSION_LEASE_ANALYSIS_BASELINE.md` plus `FND-04_PLATFORM_PRE_ADMISSION_RECONCILIATION_REFINEMENT.md`.
- **PROVEN:** current Oteryn Platform `main` observed read-only is `8e2514b8721d385b626ead7ffa47fc74067b0a0b`; its pre-admission and runtime-status contract blobs remain `a7a98b943c528b9f21c0cdc2ee90b308045706f8` and `5e45a4318716b62d53fd8bdf67b3b55676286ad1` respectively, matching the revisions consumed by the repaired FND-04 analysis.
- **PROVEN:** Platform remains reusable account/security policy and bounded admission-attempt authority; Oteryn-v2 remains final gameplay admission, CharacterLease/fencing and GameSession authority.
- **DERIVED:** the final contract can safely freeze semantic mechanisms without guessing benchmark/security-sensitive numeric values if it makes their versioned preimplementation profiles explicit implementation gates.
- **UNKNOWN:** exact grant encoding/signature algorithm profile, signing/verification library, KMS/HSM product, security-projection transport, liveness cadence/hysteresis numeric values, CharacterLease TTL/renewal numeric values and PostgreSQL physical schema.
- **CONFLICT:** none identified at task start.

## Acceptance criteria

- [ ] Freeze AccountPresenceClaim, CharacterLease, GameSession, TransportBinding and RuntimeScopeAuthority as distinct semantic authority layers.
- [ ] Freeze one linearizable fresh-admission transition and exact canonical GameSessionId creation point.
- [ ] Accept hybrid signed short-lived PreAdmissionGrant plus game-domain one-time `GrantNonce` consumption, with Platform `AdmissionAttemptRef` remaining a separate producer idempotency/correlation identity.
- [ ] Freeze post-issuance Platform-security freshness semantics and fail-closed behavior without making Platform a post-admission gameplay authority.
- [ ] Freeze fresh-entry route/runtime observation/ownership-generation applicability and stale-owner rejection.
- [ ] Freeze same-GameSession reconnect, rotating reconnect proof, lost-response reconciliation and `connection_generation` commit semantics.
- [ ] Accept the 15-second same-GameSession grace as `control_loss_declared_at + 15s` and reconcile it with the accepted 2s/5s/4s behavior.
- [ ] Freeze post-grace same-character recovery to an existing `PRESENT_UNCONTROLLED` actor using a fresh GameSessionId when safe.
- [ ] Freeze healthy-incumbent duplicate-login protection, logout-eligible takeover and account-global one-character exclusion.
- [ ] Freeze CharacterLease generation, acquisition, renewal-uncertainty, replacement, release and storage-authority semantics without inventing physical schema/TTL values.
- [ ] Freeze same-session recovery requirements across GameNode replacement and mandatory fresh-session fallback when reconstructability is insufficient.
- [ ] Freeze Channel↔Instance versus Channel↔Channel GameSession continuity and continuous account-presence semantics.
- [ ] Add stable `FS-ADMISSION-GRANT-REPLAY` and `FS-RECONNECT-CREDENTIAL-REPLAY` scenarios to the shared catalogue.
- [ ] Define stable internal admission/reconnect/takeover error codes mapped to the existing foundation categories with redacted public presentation.
- [ ] State concrete downstream implementation gates for the cross-language grant security profile and numeric security/liveness/lease parameter profiles.
- [ ] Keep Platform read-only; introduce no Rust/runtime/protocol-schema/persistence/deployment/production implementation.
- [ ] Full changed-file review, independent architecture/security audit and exact-head required CI pass before merge.

## Excluded scope

- no Rust code, listener/codec/schema registration, PostgreSQL migration or runtime implementation;
- no Oteryn-Platform write;
- no production keys, credentials, KMS/HSM configuration, traffic, sessions or account mutation;
- no exact cryptographic library/vendor choice;
- no exact grant TTL, clock-skew, verification-key cache window, reconnect-secret byte length, liveness cadence/hysteresis, CharacterLease TTL/renewal/safety-margin or rate-limit values without dedicated evidence;
- no final DUR-02 schema/isolation/transaction implementation;
- no gameplay combat/logout formula redesign;
- no client UI wording/product UX freeze.

## Implementation / findings

The completed analysis already narrowed the final decision space. This task will convert recommendations into one normative semantic contract while preserving measured/security-sensitive implementation profiles as explicit preimplementation gates rather than guessed architecture constants.

## Validation

### Focused

- command/run: semantic cross-check against accepted FND-ID-01/FND-02/FND-03, repaired FND-04 analysis and current pinned Platform contracts
- result: in progress

### Component/integration

- command/run: `NOT_APPLICABLE` — architecture/documentation only
- result: `NOT_APPLICABLE`

### E2E

- scenario: `NOT_APPLICABLE` — no executable admission/session/runtime capability is introduced by this task
- result: `NOT_APPLICABLE`

### Exact-head CI

- final head: pending
- trigger source: pull_request
- workflow/run/job: pending
- runner assignment: pending
- classification: pending
- result: pending

## Independent audit

- exact head: pending
- method/auditor: independent architecture/security review against exact final diff and pinned cross-repository evidence
- material findings: pending
- verdict: pending

## PR and closeout

- changed-file review: pending
- unresolved review threads: pending
- related/superseded PRs: repaired analysis PR #107/#108 are canonical dependencies, not competing delivery
- protected auto-merge: pending
- merge commit/result: pending
- ownership release: pending

## Context checkpoint

```yaml
invocation_started_at: 2026-08-08T21:29:00+02:00
last_progress_at: 2026-08-08T21:29:00+02:00
last_progress: Claimed one bounded final FND-04 architecture-only contract package from clean Oteryn-v2 main after repaired analysis closeout.
status: implementing
branch: docs/OTV2-20260808-fnd04-final-contract
head_sha: null
pr: null
final_head_sha: null
final_head_frozen_at: null
ci_trigger_source: pull_request
ci_check_generation: pending
ci_checks_for_current_head: 0
ci_run_ids: []
ci_job_ids: []
runner_assignment_state: unknown
terminal_ci_wait_started_at: null
terminal_ci_checks_for_current_generation: 0
unchanged_state_checks: 0
identical_failure_retries: 0
repair_cycles_for_current_gate: 0
ci_recovery_actions_for_current_head: 0
stall_warnings: 0
owner_action_required: null
blocker: null
next_action: Write the canonical final FND-04 semantic contract and add the two reviewed replay failure scenarios, then inspect the complete package diff.
```
