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
updated_at: 2026-08-08T21:38:00+02:00
execution_budget_minutes: 60
large_budget_reason: null
owned_paths:
  - docs/agents/tasks/active/OTV2-20260808-fnd04-session-admission-final.md
  - docs/architecture/FND-04_IDENTITY_GAME_SESSION_ADMISSION_CHARACTER_LEASE_CONTRACT.md
  - docs/contracts/FND-04_PRE_ADMISSION_GRANT_PROFILE_V1.md
  - docs/contracts/FND-04_REAUTHENTICATED_RECOVERY_GRANT_PROFILE_V1.md
  - docs/contracts/FOUNDATION_FAILURE_SCENARIOS.md
  - docs/architecture/FOUNDATION_PROGRAMME_CURRENT_STATUS.md
public_contracts:
  - docs/architecture/FND-04_IDENTITY_GAME_SESSION_ADMISSION_CHARACTER_LEASE_CONTRACT.md
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

Deliver the complete architecture-only FND-04 contract needed before native identity/admission/reconnect/lease implementation can be designed without guessing security or authority semantics.

Acceptance completes the FND-04 architecture gate only. It does not authorize runtime, Platform, persistence, protocol codec, key or production implementation.

## Architecture and source of truth

- **PROVEN:** FND-04 analysis baseline plus Platform reconciliation refinement are canonical on `main` after #104/#107 and closeout #108.
- **PROVEN:** duplicate/superseded PR #106 is closed unmerged and contributes no separate authority.
- **PROVEN:** current external Platform evidence is pinned read-only at `216f5b2817e9d102337608609e344518512c2a0d`.
- **PROVEN:** Platform Identity/Gateway authorizes bounded attempts; Oteryn-v2 remains final admission/GameSession/CharacterLease authority.
- **PROVEN:** FND-02 fixes TLS/bootstrap, GameSessionId issuance boundary, connection_generation and command/reconciliation semantics.
- **PROVEN:** FND-03 fixes runtime owner/fencing/time semantics and executes accepted 2s/5s/4s effects after FND-04 classifications.
- **PROVEN current standard:** RFC 9864 registers fully specified JOSE `Ed25519` and deprecates polymorphic `EdDSA`; both FND-04 grant profiles use exact `alg=Ed25519` and reject `EdDSA` fallback.
- **DERIVED AND FROZEN BY CANDIDATE:** fresh entry and reauthenticated existing-actor recovery use mutually exclusive signed profiles so Channel-bound fresh-entry authority cannot be reused to move a current actor.
- **DERIVED AND FROZEN BY CANDIDATE:** reconnect uses a two-phase PREPARE/COMMIT transition so lost responses/crashes cannot create ambiguous current generations.
- **DERIVED AND FROZEN BY CANDIDATE:** Platform account-security validity is bounded by signed generation + <=5-second trusted security-projection freshness for new admission/recovery; this does not grant Platform post-admission gameplay mutation authority.
- **DEFERRED BY EVIDENCE:** production liveness probe cadence/anti-flap hysteresis and CharacterLease TTL/renew/safety-margin values require measured fault/performance evidence before implementation acceptance.

## Acceptance criteria

### Authority and lifecycle

- [x] Freeze AccountPresenceClaim, CharacterLease, GameSession, TransportBinding and RuntimeScopeAuthority relationship.
- [x] Freeze fresh admission linearization and no-partial-authority rule.
- [x] Freeze duplicate-login / healthy-incumbent / intentional-takeover semantics.
- [x] Freeze GameSession terminality versus mandatory actor presence and same-character post-grace fresh-session attachment.
- [x] Freeze Channel↔Instance continuous-session and Channel↔Channel fresh-session continuity classes.

### Admission / recovery security profiles

- [x] Freeze exact v1 JWS Compact JWT profiles using fully specified JOSE `alg=Ed25519`, independent from application library/vendor.
- [x] Freeze exact protected-header allowlists, explicit `typ`, issuer/audience/purpose, required claims, parser/size limits and rejection of token-directed key discovery.
- [x] Freeze fresh-entry and recovery validators as mutually exclusive credential purposes.
- [x] Freeze AdmissionAttemptRef versus GrantNonce/RecoveryGrantNonce semantics and bounded replay/idempotency retention.
- [x] Freeze post-issuance Platform account-security generation/revocation freshness semantics.
- [x] Freeze fresh-entry route/runtime observation/ownership-generation binding and default stale-grant invalidation after owner-generation change.
- [x] Freeze key-purpose separation, rotation/emergency revocation and no-downgrade behavior.

### Reconnect / liveness

- [x] Freeze 32-byte game-domain reconnect secret properties.
- [x] Freeze reconnect PREPARE/COMMIT state machine, successor proof and lost-response/crash reconciliation.
- [x] Freeze one-winner connection_generation transition semantics.
- [x] Freeze exact 15-second same-session grace from the accepted 2-second loss declaration; keep 5-second transport cleanup independent.
- [x] Freeze actor/session ControlLossEpoch so routine rebind/session replacement cannot manufacture duplicate 4-second protection.
- [x] Freeze Platform-reauthenticated same-session recovery and current game-domain placement resolution.
- [x] Freeze post-grace same-character fresh GameSession attachment to the exact existing `PRESENT_UNCONTROLLED` actor without reset/recreation.
- [x] Define measured liveness cadence/hysteresis evidence gate instead of guessing production values.
- [x] Define measured CharacterLease TTL/renew/safety-margin evidence gate instead of guessing production values.

### Failure, compatibility and progression

- [x] Add stable `FS-ADMISSION-GRANT-REPLAY` and `FS-RECONNECT-CREDENTIAL-REPLAY` scenarios.
- [x] Freeze stable internal symbolic error codes and safe public presentation classes without leaking security/fencing details.
- [x] Freeze producer/consumer compatibility matrix and independent fixture requirements.
- [x] Synchronize `FOUNDATION_PROGRAMME_CURRENT_STATUS.md` through FND-03 completion, repaired FND-04 analysis and current final FND-04 delivery.

### Governance

- [x] No open PR existed at final-task start after duplicate #106 was closed.
- [x] No Rust/runtime/protocol codec/persistence schema/Platform write/key deployment/production activation introduced by this package.
- [ ] Full exact-head six-path architecture/security review has zero material conflicts.
- [ ] Exact-head Agent governance, Dependency review and CodeQL pass.
- [ ] Independent exact-head architecture/security audit passes with zero open material findings.
- [ ] Zero unresolved review threads.
- [ ] Squash merge only after final-head gates; archive/release ownership separately.

## Excluded scope

This task does not implement or authorize Oteryn-v2 GameSession/admission/reconnect/lease Rust code; protocol listener/codec/schema registration; PostgreSQL/Redis schema; Platform/Gateway code; recovery-locator code; KMS/HSM/vendor/library selection; production keys; production liveness/lease values; deployment or live traffic.

## Implementation / findings

### Final authority model

```text
AccountPresenceClaim
-> AccountId-global one playable/mandatory-presence CharacterId
CharacterLease
-> CharacterId writer/control fence + generation
GameSession
-> one logical player-control lifecycle
TransportBinding
-> GameSessionId + current connection_generation
RuntimeScopeAuthority
-> current FND-03 ChannelRuntime/InstanceRuntime owner generation
```

### Signed Platform capabilities

Fresh entry uses `oteryn-pre-admission-v1`; reauthenticated recovery uses `oteryn-reauth-recovery-v1`. Both are strict JWS Compact JWT profiles with fully specified `alg=Ed25519`, 30-second maximum token lifetime, 5-second verifier skew, <=5-second current Platform-security evidence and explicit replay/idempotency state. Exact libraries/vendors remain implementation choices.

### Reconnect ambiguity elimination

Reconnect PREPARE reserves one candidate generation/successor secret but grants no transport authority. COMMIT after successor proof atomically changes connection_generation/current transport/current reconnect verifier and fences predecessor. The system never guesses between predecessor-current and successor-current states.

### Grace / recovery

```text
T0 last sufficient control
T0+2s control loss declared
T0+5s stale concrete transport cleanup
loss declaration + 15s same-session grace expiry
```

Protection is one activation per eligible ControlLossEpoch. Post-grace mandatory actor becomes `PRESENT_UNCONTROLLED`; same-character reauthenticated recovery may create a new GameSession attached to the same actor, never respawn/reset it. Different CharacterId remains blocked until legal actor absence.

## Validation

### Focused

- accepted-input reconciliation: complete pending exact-head diff audit;
- current standards review: updated to RFC 9864 fully specified `Ed25519`; deprecated `EdDSA` is explicit negative fixture;
- profile separation/replay/route/security-freshness/reconnect/lease/liveness review: complete pending independent final audit.

### Component/integration

- `NOT_APPLICABLE` — architecture contract delivery only.

### E2E

- `NOT_APPLICABLE` for this documentation delivery. Future implementation evidence is explicitly defined by the profiles/contract.

### Exact-head CI

- final head: pending after this PR-binding commit
- trigger source: pull_request
- result: pending

## Independent audit

- exact head: pending
- verdict: pending

## PR and closeout

- final delivery PR: 109
- changed-file review: expected exactly six owned documentation paths
- unresolved review threads: pending
- merge policy: squash after exact-head validation
- ownership release: separate lifecycle closeout after delivery merge

## Context checkpoint

```yaml
last_progress: Final FND-04 PR #109 is open with six declared documentation paths. Contract/profiles use current fully specified JOSE Ed25519 and now enter exact-head security/architecture/CI validation.
status: validating
branch: docs/OTV2-20260808-fnd04-session-admission-final
head_sha: null
pr: 109
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
next_action: Freeze current PR #109 head, inspect all six paths, require fresh Agent governance/Dependency review/CodeQL and independent exact-head architecture/security audit before squash merge.
```
