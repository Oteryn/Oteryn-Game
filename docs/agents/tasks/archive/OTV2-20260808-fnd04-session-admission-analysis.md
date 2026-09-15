# OTV2-20260808-fnd04-session-admission-analysis — archived

```yaml
task_id: OTV2-20260808-fnd04-session-admission-analysis
title: Analyze FND-04 identity session admission and lease semantics
mode: CONTRACT
status: completed
repository: blakinio/Oteryn-v2
base_branch: main
branch: docs/OTV2-20260808-fnd04-session-admission-analysis-repair
pr: 107
base_sha: c638ad524772f227dabc90e88a1381cc01e907ce
head_sha: 7ebb0818b771692de36c3b5323f68e7bb8d011fe
final_head_sha: 7ebb0818b771692de36c3b5323f68e7bb8d011fe
final_head_frozen_at: 2026-08-08T21:16:00+02:00
owner: GPT-5.6 Sol architecture continuation session
created_at: 2026-08-08T20:46:00+02:00
updated_at: 2026-08-08T21:17:00+02:00
execution_budget_minutes: 60
large_budget_reason: null
owned_paths:
  - docs/agents/tasks/active/OTV2-20260808-fnd04-session-admission-analysis.md
  - docs/architecture/FND-04_SESSION_ADMISSION_LEASE_ANALYSIS_BASELINE.md
  - docs/architecture/FND-04_PLATFORM_PRE_ADMISSION_RECONCILIATION_REFINEMENT.md
public_contracts:
  - docs/architecture/FND-04_SESSION_ADMISSION_LEASE_ANALYSIS_BASELINE.md
  - docs/architecture/FND-04_PLATFORM_PRE_ADMISSION_RECONCILIATION_REFINEMENT.md
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
  - blakinio/Oteryn-Platform@216f5b2817e9d102337608609e344518512c2a0d:docs/contracts/OTERYN_V2_PRE_ADMISSION_HANDOFF_CONTRACT.md
  - blakinio/Oteryn-Platform@216f5b2817e9d102337608609e344518512c2a0d:docs/contracts/OTERYN_V2_RUNTIME_STATUS_PROJECTION_CONTRACT.md
blocks:
  - final FND-04 Identity Game Session Admission and Character Lease Contract
  - production admission reconnect takeover and character lease implementation claims
cross_repository_coordination_id: OTV2-NATIVE-FOUNDATION
external_repositories:
  - blakinio/Oteryn-Platform (read-only reconciliation evidence)
initial_delivery_pr: 104
initial_delivery_exact_head: e14a386c8cc998f69075f99890e6fe68a930b396
initial_delivery_squash_merge: c638ad524772f227dabc90e88a1381cc01e907ce
premature_closeout_pr: 105
premature_closeout_disposition: CLOSED_UNMERGED_SUPERSEDED_BY_REPAIR
repair_pr: 107
repair_exact_head: 7ebb0818b771692de36c3b5323f68e7bb8d011fe
repair_squash_merge: bcf975f215e9aa86a544e158b9e3d42ece1bc642
closeout_pr: 108
closeout_branch: docs/OTV2-20260808-fnd04-session-admission-analysis-closeout-v2
completed_at: 2026-08-08T21:17:00+02:00
ownership_released: true
next_gate: FND-04 Identity, Game Session, Admission and Character Lease Contract
```

## Outcome

The FND-04 architecture-analysis phase is complete after one post-merge repair cycle and is canonical on `main` as input to the later final FND-04 contract.

Canonical analysis consists of both:

- `FND-04_SESSION_ADMISSION_LEASE_ANALYSIS_BASELINE.md`;
- `FND-04_PLATFORM_PRE_ADMISSION_RECONCILIATION_REFINEMENT.md`.

The refinement supersedes only incomplete Platform pre-admission reconciliation wording in the primary baseline. It does not replace the baseline wholesale.

This phase remains architecture analysis only. It does not complete FND-04 and does not authorize implementation.

## Canonical analysis direction

### Authority model

- Platform Identity authenticates and owns reusable account/security state.
- Platform/Gateway authorizes bounded attempts and consumes fresh runtime observations for routing; it is not final gameplay/session authority.
- Oteryn-v2 revalidates current AccountId→CharacterId ownership, current game/runtime applicability, account presence and character lease state, then creates canonical GameSessionId only on successful final admission.
- AccountPresenceClaim, CharacterLease, GameSession, TransportBinding and FND-03 runtime-scope authority remain distinct semantic layers.
- GameSession terminality cannot release the AccountPresenceClaim while a mandatory actor remains present.
- lease expiry/renewal uncertainty cannot self-grant a replacement character writer.

### Admission and replay

- recommended credential class is a signed short-lived `PreAdmissionGrant` with authoritative game-domain one-time consume semantics;
- Platform `AdmissionAttemptRef`-class producer operation/correlation identity is distinct from game `GrantNonce` consume/replay identity and from GameSessionId;
- one ambiguous Platform issuance retry cannot mint multiple independently usable capabilities for one logical attempt;
- one GrantNonce cannot produce more than one successful game admission;
- no AdmissionId/CharacterLeaseId foundation entity is introduced without later proof.

### Platform-security freshness

- nominal signature/expiry alone is not automatically sufficient after Platform account-security authority changes post issuance;
- final FND-04 must choose a bounded testable revocation/freshness mechanism and maximum staleness/risk window;
- analysis recommends evaluating short-lived signed grants + account-security generation/revision binding + trusted bounded-staleness game-side security-revocation/generation projection, with fail-closed new admission when required freshness cannot be proven and optional exceptional introspection if security evidence requires it;
- no Platform post-admission gameplay mutation authority is implied by this recommendation.

### Runtime route/ownership applicability

- Platform fresh-entry issuance uses a fresh current-owner Oteryn-v2 runtime observation;
- grant profile must preserve enough route/observation/ownership-generation applicability evidence to reject superseded routing;
- Oteryn-v2 current owner/readiness/revisions are revalidated at final admission;
- default safe direction is that target ownership-generation change invalidates an otherwise unexpired fresh-entry grant and requires fresh Platform routing/grant unless equivalent safe carry-forward is explicitly proven;
- existing actor/session recovery resolves current game-domain placement and cannot be driven by stale fresh-entry ChannelId data.

### Reconnect/session

- reconnect proof is game-domain opaque rotating secret material, never GameSessionId or Platform token;
- lost rebind response requires bounded idempotent reconciliation; rotate-and-forget is rejected;
- same-GameSession rebind has one atomic newer `connection_generation` winner;
- routine/pre-loss transport replacement cannot manufacture four-second PvE re-entry protection;
- protection applies only to eligible server-classified unexpected loss episodes;
- same-GameSession recovery across GameNode replacement requires reconstructable FND-02/FND-04 state or falls back to fresh-session recovery;
- after same-session grace expiry, a mandatory actor may remain `PRESENT_UNCONTROLLED`, blocking another CharacterId; a later final contract decides safe fresh-session reattachment to that same actor.

## Acceptance criteria

### Primary analysis

- [x] Reconciled Platform authorization attempt with game-domain final admission/GameSession authority.
- [x] Separated account presence, character lease, logical session and transport binding.
- [x] Preserved Character Authority current ownership revalidation.
- [x] Analyzed hybrid signed/opaque options and recommended hybrid signed + one-time consume direction.
- [x] Analyzed reconnect rotation/replay/lost-response behavior.
- [x] Analyzed exact reconnect grace relation to 2s/5s/4s accepted timing.
- [x] Preserved combat-locked incumbent protection and account-global one-character invariant.
- [x] Defined post-grace mandatory actor state and same-character recovery direction.
- [x] Defined lease acquisition/renewal/expiry/fencing semantics without schema implementation.
- [x] Classified shared foundation failures and identified explicit replay scenario candidates.

### Platform reconciliation repair

- [x] Reconciled current `Oteryn-Platform@216f5b2817e9d102337608609e344518512c2a0d` pre-admission handoff contract.
- [x] Reconciled current Platform runtime-status projection/current-owner generation semantics.
- [x] Made post-issuance account-security change a mandatory final-contract decision with bounded freshness/risk.
- [x] Preserved explicit runtime observation / route / ownership-generation applicability and current game-owner validation.
- [x] Separated Platform AdmissionAttemptRef producer idempotency from GrantNonce consumer replay state.
- [x] Added required account-security revocation, runtime-generation change and ambiguous issuance race cases.
- [x] Kept external Platform repository read-only.

### Governance and evidence

- [x] Initial PR #104 exact head `e14a386c8cc998f69075f99890e6fe68a930b396` passed Agent governance `31273492498`, Dependency review `31273492528`, CodeQL `31273492495` and architecture/security audit review `4889485214` before its merge.
- [x] A delayed exact-head review then identified a material current-Platform reconciliation P1.
- [x] Premature closeout PR #105 was stopped and closed unmerged before it could release ownership.
- [x] Repair PR #107 exact head `7ebb0818b771692de36c3b5323f68e7bb8d011fe` changed exactly the active task plus one companion refinement.
- [x] Repair Agent governance run `31273968716`: `PASS`.
- [x] Repair Dependency review run `31273968706`: `PASS`.
- [x] Repair CodeQL run `31273968729`: `PASS`.
- [x] Repair exact-head architecture/security audit review `4889502302`: `PASS`, zero material findings.
- [x] Repair unresolved review threads at merge: `0`.
- [x] Repair PR #107 squash merge: `bcf975f215e9aa86a544e158b9e3d42ece1bc642`.
- [x] No runtime/Platform/persistence/protocol/key/deployment implementation introduced.
- [x] Replacement lifecycle closeout created only after repair merge.

## Material findings and dispositions

1. **Reconnect lost-response ambiguity** — fixed before initial analysis merge by requiring bounded idempotent reconciliation and forbidding stale generation restoration.
2. **Ordinary rebind protection ambiguity** — fixed before initial merge; connection_generation change alone never creates the four-second defensive PvE effect.
3. **Premature technology freeze** — fixed before initial merge by requiring a versioned cross-language security/interchange profile while leaving application libraries/vendors outside architecture.
4. **Recovery route authority** — fixed before initial merge; current game-domain actor placement controls recovery.
5. **Lease expiry replacement ambiguity** — fixed before initial merge; expiry/uncertainty is not replacement authority.
6. **Current Platform reconciliation gap** — discovered by delayed exact-head review after #104. Repaired in #107 with current Platform pin and companion refinement covering account-security changes after issuance, runtime owner-generation applicability and producer issuance idempotency.

No known material analysis finding remains after repair merge.

## Excluded scope

This phase did not implement or authorize:

- GameSession/admission/lease Rust runtime;
- protocol codecs/listeners/schema registration;
- PostgreSQL/Redis persistence schema;
- Platform/Gateway changes;
- grant issuer/verification/revocation transport;
- KMS/HSM or token library/vendor;
- production TTL, liveness cadence or lease values;
- production routes/keys/traffic/deployment;
- final FND-04 implementation.

## Next safe gate

One bounded final architecture-only `FND-04 Identity, Game Session, Admission and Character Lease Contract` task may now start from current `main`.

It must consume **both** analysis documents plus current Platform contracts and freeze the remaining implementation-blocking decisions, including security/interchange profile semantics, post-issuance Platform-security freshness, runtime-owner applicability, AdmissionAttemptRef/GrantNonce behavior, reconnect lost-response reconciliation, current-placement recovery, post-grace same-character control reattachment, exact 15-second timing composition, liveness and lease evidence gates, handoff continuity, replay failure IDs and stable errors.

## Context checkpoint

```yaml
last_progress: FND-04 analysis was repaired after delayed review exposed a current-Platform reconciliation P1. Repair PR #107 passed exact-head governance, dependency, CodeQL and architecture/security audit at 7ebb0818b771692de36c3b5323f68e7bb8d011fe and squash-merged as bcf975f215e9aa86a544e158b9e3d42ece1bc642. Analysis ownership is now ready for replacement closeout.
status: completed
branch: docs/OTV2-20260808-fnd04-session-admission-analysis-repair
head_sha: 7ebb0818b771692de36c3b5323f68e7bb8d011fe
pr: 107
final_head_sha: 7ebb0818b771692de36c3b5323f68e7bb8d011fe
final_head_frozen_at: 2026-08-08T21:16:00+02:00
ci_trigger_source: pull_request
ci_check_generation: repair-final
ci_checks_for_current_head: 3
ci_run_ids:
  - 31273968716
  - 31273968706
  - 31273968729
ci_job_ids:
  - 93144467427
  - 93144467441
runner_assignment_state: completed
terminal_ci_wait_started_at: null
terminal_ci_checks_for_current_generation: 3
unchanged_state_checks: 0
identical_failure_retries: 0
repair_cycles_for_current_gate: 1
ci_recovery_actions_for_current_head: 0
stall_warnings: 0
owner_action_required: null
blocker: null
next_action: Merge replacement lifecycle closeout, then start one final architecture-only FND-04 contract task from current main.
```
