# OTV2-20260808-fnd04-session-admission-analysis

```yaml
task_id: OTV2-20260808-fnd04-session-admission-analysis
title: Analyze FND-04 identity session admission and lease semantics
mode: CONTRACT
status: validating
repository: blakinio/Oteryn-v2
base_branch: main
branch: docs/OTV2-20260808-fnd04-session-admission-analysis
pr: 104
base_sha: 3c32fb08ddf52939159c0ace5fe607ca4fb18332
head_sha: null
final_head_sha: null
final_head_frozen_at: null
owner: GPT-5.6 Sol architecture continuation session
created_at: 2026-08-08T20:46:00+02:00
updated_at: 2026-08-08T21:00:00+02:00
execution_budget_minutes: 60
large_budget_reason: null
owned_paths:
  - docs/agents/tasks/active/OTV2-20260808-fnd04-session-admission-analysis.md
  - docs/architecture/FND-04_SESSION_ADMISSION_LEASE_ANALYSIS_BASELINE.md
public_contracts:
  - docs/architecture/FND-04_SESSION_ADMISSION_LEASE_ANALYSIS_BASELINE.md
depends_on:
  - docs/architecture/ADR-0003-platform-identity-game-gateway-and-admission-boundary.md
  - docs/architecture/ADR-0012-character-authority-and-platform-lifecycle-boundary.md
  - docs/architecture/FND-ID-01_FOUNDATION_IDENTIFIER_CONTRACT.md
  - docs/architecture/FND-ID-01_GAME_SESSION_ID_OWNER_ISSUER_BASELINE.md
  - docs/architecture/FND-ID-01_GAME_SESSION_RECONNECT_GENERATION_OWNER_BASELINE.md
  - docs/architecture/FND-ID-01_ACCOUNT_SINGLE_ONLINE_CHARACTER_OWNER_BASELINE.md
  - docs/architecture/FND-02_PROTOCOL_OTERYN_V1_CONTRACT.md
  - docs/architecture/FND-03_RUNTIME_EXECUTION_CONTRACT.md
  - docs/architecture/DISCONNECT_REENTRY_PVE_PROTECTION_OWNER_DECISION.md
  - docs/contracts/FOUNDATION_ERROR_VOCABULARY.md
  - docs/contracts/FOUNDATION_FAILURE_SCENARIOS.md
blocks:
  - final FND-04 Identity Game Session Admission and Character Lease Contract
  - production admission reconnect takeover and character lease implementation claims
cross_repository_coordination_id: OTV2-NATIVE-FOUNDATION
external_repositories:
  - blakinio/Oteryn-Platform (read-only reconciliation evidence)
```

## Outcome

Produce a bounded architecture-analysis baseline for `FND-04` before freezing its security-sensitive final contract. The baseline reconciles accepted identity, Platform/Gateway, protocol, runtime, reconnect, duplicate-login, combat-presence and character-ownership decisions; classifies which FND-04 choices must be decided before implementation versus measured/deferred; and recommends one coherent admission/session/lease state model without implementing runtime, persistence or Platform changes.

## Architecture and source of truth

- **PROVEN:** `main@3c32fb08ddf52939159c0ace5fe607ca4fb18332` contains accepted FND-01/FND-ID-01/FND-02/FND-03 foundation semantics and the FND-03 lifecycle closeout.
- **PROVEN:** no open pull request existed at task start.
- **PROVEN:** Platform Identity owns reusable account credentials, OAuth/PKCE, MFA and Game Login Ticket issuance; Game Gateway remains a Platform control-plane component and is not gameplay authority.
- **PROVEN:** canonical `GameSessionId` is game-domain-owned and is issued only after successful authoritative admission.
- **PROVEN:** FND-02 already fixes `connection_generation` as a non-zero post-admission `uint64` fence scoped to one GameSessionId, with strictly newer generation on accepted rebind.
- **PROVEN:** one AccountId may have at most one authoritative online CharacterId; a healthy combat/PZ/logout-locked incumbent cannot be kicked by a duplicate login.
- **PROVEN:** an eligible short reconnect preserves GameSessionId, advances connection generation, and starts from an owner-accepted initial 15-second reconnect-grace policy.
- **PROVEN:** FND-03 owns monotonic execution of accepted 2-second disconnect protection, 5-second stale-transport cleanup and 4-second defensive PvE re-entry effects; FND-04 owns which current-generation evidence is sufficient and the exact logical session/reconnect state semantics.
- **PROVEN:** the canonical current-status overlay on main still contains transition-safe historical wording around PR #102 and therefore needs a later narrow synchronization after the FND-04 analysis/final package selects the correct progression wording.
- **DERIVED:** FND-04 needs separate semantic concepts for account presence exclusion, character player-control authority and transport binding even if later persistence stores them in one transaction/record family.
- **DERIVED:** GameSession terminality cannot release account-global presence while the authoritative actor remains mandatory in world; otherwise a different CharacterId could become playable while the first actor still exists.
- **DERIVED:** lease expiry/renewal uncertainty cannot by itself grant a replacement writer; replacement requires an explicit newer fenced authority after the old generation is unable to commit.
- **UNKNOWN:** exact pre-admission credential container/algorithm/library, reconnect secret primitive/length, physical lease store/schema, exact heartbeat cadence, lease durations/safety margin and final public numeric error codes.

## Acceptance criteria

### Authority and state model

- [x] Reconcile Platform authorization-to-attempt-admission with game-domain final admission and canonical GameSessionId creation.
- [x] Separate account-global online-character exclusion from CharacterId-specific player-control/session authority and from concrete transport binding.
- [x] Define semantic linearization points for fresh admission, duplicate-login takeover, same-session reconnect, fresh-session recovery, logout and terminal session end.
- [x] Preserve Character Authority ownership revalidation at final admission.
- [x] Preserve account presence across GameSession terminality while the actor remains mandatory in world.

### Credential and replay analysis

- [x] Compare signed, opaque and hybrid pre-admission credentials against Platform availability, replay prevention, key rotation, revocation and cross-language rollout needs.
- [x] Define minimum binding fields and validation order without changing FND-02 framing/schema ownership.
- [x] Analyze reconnect credential replay/rotation and lost-response/crash ambiguity.
- [x] Decide whether `AdmissionId` or `CharacterLeaseId` is actually required; current recommendation is no new foundation identity without proof.

### Liveness reconnect and takeover

- [x] Define sufficient current-generation liveness evidence conceptually without using socket-open state or client timestamps as authority.
- [x] Analyze exact server-authoritative start of the accepted 15-second reconnect window relative to the 2-second control-loss boundary.
- [x] Preserve 5-second transport cleanup independence from logical GameSession continuity.
- [x] Define same-character recovery after reconnect-grace expiry while the actor still has mandatory combat/PZ/logout presence.
- [x] Preserve healthy combat-locked incumbent protection and account-global one-online-character invariant under races.
- [x] Identify anti-flap/re-entry-protection abuse risk without inventing an unapproved gameplay penalty.

### Lease fencing and failure

- [x] Define account/character lease-generation semantics and stale-owner behavior without freezing PostgreSQL schema.
- [x] Define renewal failure, expiry, revocation and safe-loss-of-control semantics consistent with FND-03 fencing.
- [x] Require that lease expiry/uncertainty cannot automatically authorize a replacement writer before explicit fencing/recovery.
- [x] Classify every shared foundation failure scenario for FND-04 and identify candidate missing replay scenario IDs without expanding the catalogue prematurely.
- [x] Define key rotation/emergency revocation requirements and fail-closed behavior for revision/route/audience mismatch.

### Governance

- [x] Architecture analysis only; no Rust, protocol runtime, persistence schema, Platform write, deployment or production activation.
- [ ] Full changed-path review has zero unresolved material conflicts.
- [ ] Exact-head Agent governance, Dependency review and CodeQL pass.
- [ ] Independent exact-head architecture/security audit passes with zero open material findings.
- [ ] Squash merge only after all final-head gates pass; archive/release ownership separately.

## Excluded scope

This task does not implement or authorize:

- Rust Game Session/admission/lease runtime;
- Platform/Gateway code or external-repository writes;
- PostgreSQL tables, locks, isolation levels or migrations;
- production key infrastructure or secret material;
- `protocol-oteryn` codecs/listeners/client adapters or new production message registration;
- exact gameplay combat/logout formulas;
- production heartbeat/lease numeric values unless the architecture analysis proves a value is forced by an already accepted invariant;
- production traffic or deployment.

## Implementation / findings

The analysis baseline is now present in PR #104 and recommends four distinct semantic authority layers rather than one overloaded session concept:

1. account-global presence exclusion scoped by AccountId;
2. character authority/lease fencing scoped by CharacterId;
3. logical player-control lifecycle identified by GameSessionId;
4. current concrete transport binding fenced by connection_generation.

It recommends a hybrid signed pre-admission capability with game-side one-time consumption, a game-domain opaque rotating reconnect secret, no new AdmissionId/CharacterLeaseId without evidence, and a full 15-second same-GameSession grace measured from the server-authoritative control-loss declaration rather than the last good probe. These remain analysis recommendations until a final FND-04 contract is accepted.

Material review constraints now explicitly include:

- GameSession end never makes a still-present combat/PZ/logout actor disappear or frees the account for another character;
- lease timeout/uncertainty never acts as automatic replacement authority;
- any same-character post-grace recovery must attach to the same authoritative actor without respawn/reset;
- any recovery/handoff route must resolve the actor's current authoritative placement rather than treat stale client/Platform placement as authority;
- same-session recovery across GameNode replacement may be claimed only if FND-02 command/session high-water, current generation and reconnect/fencing state are safely preserved/reconstructed; otherwise the old GameSession terminates and a fresh-session recovery path is required.

## Validation

### Focused

- accepted-input reconciliation: completed against ADR-0003, ADR-0012, FND-ID-01, FND-02, FND-03, reconnect/duplicate-login/disconnect baselines, error vocabulary and failure catalogue.
- result: `PASS` pending independent final-diff audit.

### Component/integration

- result: `NOT_APPLICABLE` — architecture-analysis delivery only.

### E2E

- result: `NOT_APPLICABLE` — no executable capability introduced.

### Exact-head CI

- final head: pending after this task synchronization commit
- trigger source: pull_request
- workflow/run/job: pending
- result: pending

## Independent audit

- exact head: pending after task synchronization
- method/auditor: pending
- material findings: pending
- verdict: pending

## PR and closeout

- delivery PR: 104
- changed-file review: exactly two declared documentation paths expected
- unresolved review threads: pending
- related/superseded PRs: none at task start
- merge policy: squash after exact-head validation
- merge commit/result: pending
- ownership release: pending separate closeout

## Context checkpoint

```yaml
last_progress: FND-04 analysis baseline is drafted in PR #104 and accepted inputs are reconciled; the package is now in exact-head diff/audit/CI validation with no runtime or external-repository implementation authority.
status: validating
branch: docs/OTV2-20260808-fnd04-session-admission-analysis
head_sha: null
pr: 104
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
repair_cycles_for_current_gate: 0
ci_recovery_actions_for_current_head: 0
stall_warnings: 0
owner_action_required: null
blocker: null
next_action: Perform full exact-head two-path architecture/security review of PR #104 and repair any material finding before freeze/merge.
```
