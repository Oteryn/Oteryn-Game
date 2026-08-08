# OTV2-20260808-fnd04-session-admission-final

```yaml
task_id: OTV2-20260808-fnd04-session-admission-final
title: Finalize FND-04 identity Game Session admission and character lease contract
mode: CONTRACT
status: investigating
repository: blakinio/Oteryn-v2
base_branch: main
branch: docs/OTV2-20260808-fnd04-session-admission-final
pr: null
base_sha: 27f7f647f04e3b1a4151f9b124401986910f03d8
head_sha: null
final_head_sha: null
final_head_frozen_at: null
owner: GPT-5.6 Sol architecture continuation session
created_at: 2026-08-08T21:22:00+02:00
updated_at: 2026-08-08T21:22:00+02:00
execution_budget_minutes: 60
large_budget_reason: null
owned_paths:
  - docs/agents/tasks/active/OTV2-20260808-fnd04-session-admission-final.md
  - docs/architecture/FND-04_IDENTITY_GAME_SESSION_ADMISSION_CHARACTER_LEASE_CONTRACT.md
  - docs/contracts/FND-04_PRE_ADMISSION_GRANT_PROFILE_V1.md
  - docs/contracts/FOUNDATION_FAILURE_SCENARIOS.md
  - docs/architecture/FOUNDATION_PROGRAMME_CURRENT_STATUS.md
public_contracts:
  - docs/architecture/FND-04_IDENTITY_GAME_SESSION_ADMISSION_CHARACTER_LEASE_CONTRACT.md
  - docs/contracts/FND-04_PRE_ADMISSION_GRANT_PROFILE_V1.md
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
  - Platform native pre-admission producer rollout
  - production protocol-oteryn admission/reconnect traffic
cross_repository_coordination_id: OTV2-NATIVE-FOUNDATION
external_repositories:
  - blakinio/Oteryn-Platform (read-only reconciliation evidence)
```

## Outcome

Deliver the complete architecture-only FND-04 contract needed before native identity/admission/reconnect/lease implementation can be designed without guessing security or authority semantics.

The package must freeze the final authority state machine, a concrete interoperable PreAdmissionGrant security profile, post-issuance Platform-security freshness semantics, route/runtime generation applicability, issuance and consume idempotency, reconnect proof/rebind ambiguity handling, liveness/reconnect timing, account/character fencing, post-grace actor recovery, takeover/handoff behavior, stable failure scenarios/errors and downstream evidence gates.

Acceptance completes the FND-04 architecture gate only. It does not authorize runtime, Platform, persistence, protocol codec or production implementation.

## Architecture and source of truth

- **PROVEN:** FND-04 analysis baseline and Platform reconciliation refinement are canonical on `main` after PRs #104/#107 and replacement closeout #108.
- **PROVEN:** duplicate/superseded reconciliation PR #106 is closed unmerged and contributes no separate authority.
- **PROVEN:** current external Platform evidence is pinned read-only at `216f5b2817e9d102337608609e344518512c2a0d`.
- **PROVEN:** Platform Identity/Gateway authorizes a bounded attempt; Oteryn-v2 remains final admission/GameSession/CharacterLease authority.
- **PROVEN:** FND-02 fixes TLS/bootstrap, bounded opaque admission/reconnect material, GameSessionId issuance boundary, connection_generation and command/reconciliation semantics.
- **PROVEN:** FND-03 fixes runtime owner/fencing/time semantics and executes accepted 2s/5s/4s behavior after FND-04 classifications.
- **DERIVED:** a narrow JWS/JWT profile with one Ed25519/EdDSA algorithm, explicit typing, fixed issuer/audience and dedicated key purpose is currently the strongest interoperability/security fit for Platform PHP↔game Rust while remaining library-neutral.
- **DERIVED:** Platform account-security changes after grant issuance require bounded freshness/revocation evidence; nominal JWT expiry alone is insufficient for all security transitions.
- **UNKNOWN until final review:** whether all selected numeric grant/security freshness values have sufficient rationale or should be expressed as hard upper bounds plus later tunable defaults.

## Acceptance criteria

### Authority and lifecycle

- [ ] Freeze AccountPresenceClaim, CharacterLease, GameSession, TransportBinding and RuntimeScopeAuthority relationship.
- [ ] Freeze fresh admission linearization and no-partial-authority rule.
- [ ] Freeze duplicate login / healthy incumbent / intentional takeover semantics.
- [ ] Freeze GameSession terminality versus mandatory actor presence and same-character fresh-session reattachment.
- [ ] Freeze Channel↔Instance and Channel↔Channel session continuity.

### Admission security profile

- [ ] Freeze exact v1 signed grant envelope/profile and cryptographic algorithm class independent from implementation library/vendor.
- [ ] Freeze protected-header allowlist, explicit token type, issuer/audience, required claims, size/count limits and rejection of token-directed key discovery.
- [ ] Freeze AdmissionAttemptRef versus GrantNonce semantics and bounded replay/idempotency retention.
- [ ] Freeze post-issuance Platform account-security generation/revocation freshness semantics.
- [ ] Freeze runtime observation / route / ownership-generation binding and stale-grant invalidation.
- [ ] Define key rotation/emergency revocation and mixed-version downgrade behavior.

### Reconnect and liveness

- [ ] Freeze reconnect secret security properties and exact successor/lost-response reconciliation state machine.
- [ ] Freeze connection_generation transition/winner semantics.
- [ ] Freeze exact 15-second same-session grace start/end relative to 2-second loss boundary and 5-second transport cleanup.
- [ ] Freeze protection eligibility at actor/control-loss episode level so rebind/session replacement cannot manufacture duplicate 4-second protection.
- [ ] Freeze reauthenticated recovery and current-placement resolution rules.
- [ ] Define liveness cadence/lease numeric evidence gates without guessing performance-sensitive constants when not architecturally forced.

### Failure, compatibility and progression

- [ ] Add explicit stable failure scenarios for admission-grant replay and reconnect-credential replay.
- [ ] Map final internal/public failure categories without leaking security state/secrets.
- [ ] Freeze producer/consumer compatibility and independent fixture requirements.
- [ ] Synchronize `FOUNDATION_PROGRAMME_CURRENT_STATUS.md` so FND-03 is durably complete and FND-04 delivery is transition-safe.

### Governance

- [x] No open PR existed at final task start after duplicate #106 was closed.
- [ ] No Rust/runtime/protocol codec/persistence schema/Platform write/key deployment/production activation.
- [ ] Full exact-head five-path architecture/security review has zero material conflicts.
- [ ] Exact-head Agent governance, Dependency review and CodeQL pass.
- [ ] Independent exact-head architecture/security audit passes with zero open material findings.
- [ ] Squash merge only after final-head gates; archive/release ownership separately.

## Excluded scope

This task does not implement or authorize:

- Oteryn-v2 Game Session/admission/reconnect/lease Rust code;
- protocol listener/codec/schema-registry production implementation;
- PostgreSQL/Redis schema, transaction isolation or migration;
- Platform/Gateway code or external-repository writes;
- KMS/HSM/vendor/library selection;
- production key creation/rotation;
- production liveness probe or lease timer rollout;
- deployment or live traffic.

## Implementation / findings

The final contract may select protocol/security standards when they are part of the cross-language contract, but it must not make a specific library/framework/vendor a canonical architecture dependency without evidence.

Current security-profile candidate uses JWS Compact JWT under RFC 7515/7519, RFC 8725 BCP validation rules and RFC 8037 Ed25519/EdDSA JOSE interoperability. Exact header/claim profile and resource limits will be reviewed before being frozen.

Proof-of-possession remains a future extension point rather than a first-release requirement unless final threat review shows a bearer grant/reconnect secret is unacceptable despite TLS, short lifetime, one-time consume and replay fencing.

## Validation

### Focused

- accepted-input reconciliation: in progress
- current standards/security-profile review: in progress

### Component/integration

- `NOT_APPLICABLE` — architecture contract only.

### E2E

- `NOT_APPLICABLE` for this docs delivery; contract must define future exact-revision fixtures/fault scenarios required before implementation acceptance.

### Exact-head CI

- final head: pending
- trigger source: pull_request
- result: pending

## Independent audit

- exact head: pending
- verdict: pending

## PR and closeout

- final delivery PR: pending
- changed-file review: pending
- unresolved review threads: pending
- merge policy: squash after exact-head validation
- ownership release: separate lifecycle closeout after delivery merge

## Context checkpoint

```yaml
last_progress: Repaired FND-04 analysis is closed out on main and duplicate PR #106 is closed. Final FND-04 architecture task now owns the complete session/admission/lease contract, grant security profile, failure-scenario additions and current-status synchronization.
status: investigating
branch: docs/OTV2-20260808-fnd04-session-admission-final
head_sha: null
pr: null
final_head_sha: null
final_head_frozen_at: null
ci_trigger_source: null
ci_check_generation: null
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
next_action: Freeze the concrete v1 PreAdmissionGrant profile and final FND-04 authority/reconnect contract, then reconcile failure scenarios and current foundation status before opening the final PR.
```
