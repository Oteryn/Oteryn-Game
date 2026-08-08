# OTV2-20260808-fnd04-final-contract

```yaml
task_id: OTV2-20260808-fnd04-final-contract
title: Finalize FND-04 identity session admission and lease contract
mode: CONTRACT
status: blocked
repository: blakinio/Oteryn-v2
base_branch: main
branch: docs/OTV2-20260808-fnd04-final-contract
pr: 110
base_sha: 27f7f647f04e3b1a4151f9b124401986910f03d8
head_sha: null
final_head_sha: null
final_head_frozen_at: null
owner: unassigned_pending_explicit_server_authorization
created_at: 2026-08-08T21:29:00+02:00
updated_at: 2026-08-08T21:48:00+02:00
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
  - explicit project-owner authorization to resume any server-repository work
cross_repository_coordination_id: OTV2-NATIVE-FOUNDATION
external_repositories:
  - blakinio/Oteryn-Platform (read-only evidence only)
```

## Mandatory owner scope checkpoint — 2026-08-08 21:48 CEST

This FND-04 package was started by the current assistant while the project owner intended the active session to work **only on the WWW Platform (`blakinio/Oteryn-Platform`)**.

The project owner explicitly corrected the scope and ordered that server repositories must **not be touched again without asking first and receiving explicit permission**.

Therefore this task is intentionally left `blocked` and unowned. No agent continuing Platform/WWW work may treat this task as implicit authority to modify `blakinio/Oteryn-v2`, the game server, protocol, runtime, persistence or related server repositories.

### Required disposition of the work already created

The changes already made in this branch/PR are **not to be silently discarded or treated as accepted**. When the project owner later explicitly authorizes server work, the authorized server agent must:

1. re-read the complete PR #110 diff and all canonical FND-04 inputs from current `main`;
2. continue/repair the candidate FND-04 contract rather than assuming the current draft is final;
3. specifically resolve any remaining audit findings and semantic races before acceptance;
4. perform an independent architecture/security audit of the exact final head;
5. run the repository-required exact-head CI/governance checks;
6. merge only with zero unresolved material findings and then perform normal task archive/ownership release;
7. keep runtime/protocol/persistence implementation unauthorized unless the project owner separately authorizes implementation.

This checkpoint is the final server-repository mutation made by the current Platform-focused session unless the project owner explicitly asks to resume server work.

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
- **CONFLICT:** this package is currently blocked by owner scope; no server work may resume without explicit authorization.

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
- [x] Platform remained read-only during this server package; no Rust/runtime/protocol-schema/persistence/deployment/production implementation was introduced.
- [ ] Resolve remaining semantic/audit findings after explicit server-work authorization.
- [ ] Full exact candidate diff review and independent architecture/security audit have zero material findings.
- [ ] Exact-head required CI passes with zero unresolved review threads before merge.

## Excluded scope

- no Rust code, listener/codec/schema registration, database migration or runtime implementation;
- no Oteryn-Platform write from this server task;
- no production keys/credentials/traffic/sessions/deployment;
- no cryptographic library/KMS/HSM vendor choice;
- no guessed grant TTL/skew/key-cache, reconnect-proof size, liveness cadence/hysteresis, lease TTL/renewal/safety-margin or rate-limit values;
- no DUR-02 physical schema/transaction choice;
- no combat/logout formula redesign or final client UX wording.

## Implementation / findings

Candidate contract: `docs/architecture/FND-04_IDENTITY_GAME_SESSION_ADMISSION_LEASE_CONTRACT.md`.

Shared failure catalogue adds exactly the two replay scenarios recommended by the repaired analysis.

### Author-review repairs before scope stop

1. **Credential-type confusion:** initial draft placed fresh entry and reauthenticated recovery under one PreAdmissionGrant-purpose section. Repaired so `PreAdmissionGrant = FRESH_ENTRY` only; recovery is a distinct future `RecoveryAuthorizationGrant`, with mutually exclusive validation and a separately authorized Platform producer/profile requirement.
2. **Liveness state gap:** initial draft transitioned from ACTIVE directly to CONTROL_LOST_GRACE on insufficient evidence, which could collapse the accepted two-second declaration window. Repaired with explicit `CONTROL_SUSPECT`; grace begins only at `control_loss_declared_at`.
3. **Lease/session lifetime coupling:** initial wording could imply an ACTIVE GameSession was required to renew CharacterLease. Repaired so lease renewal follows current authoritative actor/runtime writer lifecycle and may continue for a mandatory `PRESENT_UNCONTROLLED` actor, while player-originated control still requires an active current GameSession/TransportBinding.
4. **Recovery error ambiguity:** fresh-entry credential failure and recovery credential failure now have distinct internal symbolic codes.
5. **Further audit work was in progress when owner corrected repository scope.** A future explicitly authorized server agent must not assume PR #110 is audit-complete.

## Validation

### Focused

- semantic cross-check: accepted FND-ID-01/FND-02/FND-03 + repaired FND-04 analysis + current pinned Platform contracts
- result: `PARTIAL / NEEDS CONTINUATION AFTER EXPLICIT SERVER AUTHORIZATION`
- current Platform revision last rechecked: `8e2514b8721d385b626ead7ffa47fc74067b0a0b`
- current Oteryn-v2 base last rechecked: `27f7f647f04e3b1a4151f9b124401986910f03d8`

### Component/integration

- result: `NOT_APPLICABLE` — architecture/documentation only; no executable component changed

### E2E

- result: `NOT_APPLICABLE` — this task introduces no executable admission/session capability; future implementation E2E remains required by the candidate contract

### Exact-head CI

- result: `NOT COMPLETED BEFORE OWNER SCOPE STOP`

## Independent audit

- result: `NOT COMPLETED BEFORE OWNER SCOPE STOP`
- mandatory before merge after explicit server-work authorization

## PR and closeout

- delivery PR: #110
- PR must remain unmerged until future explicitly authorized server continuation completes audit/CI/review gates
- post-merge task archive/ownership release remains required

## Context checkpoint

```yaml
invocation_started_at: 2026-08-08T21:29:00+02:00
last_progress_at: 2026-08-08T21:48:00+02:00
last_progress: Project owner corrected scope: active agent must work only on Oteryn-Platform WWW. Existing FND-04 draft is preserved for future explicitly authorized server continuation and mandatory independent audit.
status: blocked
branch: docs/OTV2-20260808-fnd04-final-contract
head_sha: null
pr: 110
final_head_sha: null
final_head_frozen_at: null
ci_trigger_source: pull_request
ci_check_generation: not-final
ci_checks_for_current_head: 0
ci_run_ids: []
ci_job_ids: []
runner_assignment_state: not_applicable_until_authorized_continuation
terminal_ci_wait_started_at: null
terminal_ci_checks_for_current_generation: 0
unchanged_state_checks: 0
identical_failure_retries: 0
repair_cycles_for_current_gate: 1
ci_recovery_actions_for_current_head: 0
stall_warnings: 0
owner_action_required: explicit_permission_to_resume_server_repository_work
blocker: owner_scope_prohibits_server_work_without_explicit_permission
next_action: Do not touch Oteryn-v2 or any server repository. Work only on blakinio/Oteryn-Platform WWW. If the owner later explicitly authorizes server work, resume PR #110 from current main, complete semantic repairs, independent audit, exact-head CI and lifecycle closeout.
```
