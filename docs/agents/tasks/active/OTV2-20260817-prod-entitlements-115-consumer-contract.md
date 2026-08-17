# OTV2-20260817-prod-entitlements-115-consumer-contract

```yaml
task_id: OTV2-20260817-prod-entitlements-115-consumer-contract
title: PROD-ENTITLEMENTS-01 Oteryn-v2 consumer/enforcement contract
mode: CONTRACT
status: implementing
repository: blakinio/Oteryn-v2
base_branch: main
branch: docs/prod-entitlements-115-consumer-contract
pr: null
base_sha: bf2a2ae279516f62626a5d8f4dc1aeb587535c62
head_sha: null
final_head_sha: null
final_head_frozen_at: null
owner: OTV2-ENTITLEMENTS-115/current-session
created_at: 2026-08-17T08:44:00+02:00
updated_at: 2026-08-17T08:44:00+02:00
execution_budget_minutes: 60
large_budget_reason: null
owned_paths:
  - docs/agents/tasks/active/OTV2-20260817-prod-entitlements-115-consumer-contract.md
  - docs/architecture/PROD-ENTITLEMENTS-01_GAME_CONSUMER_ENFORCEMENT_CONTRACT_CANDIDATE.md
public_contracts:
  - PROD-ENTITLEMENTS-01 consumer/enforcement candidate
depends_on:
  - Oteryn-v2 issue #115
  - docs/architecture/PROD-ENTITLEMENTS-01_PLATFORM_GAME_ENFORCEMENT_DEPENDENCY.md
  - blakinio/Oteryn-Platform@afaa6d1d8340e44b1152b62d6d27e5fd1649804a
  - FND-04
  - DUR-02
  - DUR-03
  - ANL-01
blocks:
  - Profile-B/Premium/VIP and equivalent game-consumed entitlement implementation or activation
cross_repository_coordination_id: OTV2-PROD-ENTITLEMENTS
external_repositories:
  - blakinio/Oteryn-Platform (read-only producer authority)
```

`MERGE_AUTHORITY: ARCHITECTURE_COORDINATOR_ONLY`
`IMPLEMENTATION_AUTHORITY: NONE`

## Outcome

Produce one bounded paper-only `PROD-ENTITLEMENTS-01` candidate that defines the Oteryn-v2 game-consumer/enforcement side of the already accepted Platform commercial-entitlement authority split. The package must make stale/replayed entitlement authority finite and non-resurrectable, define deterministic game-side state evaluation and session boundaries, preserve idempotent game-affecting delivery, and define exact producer/consumer rollout evidence without implementing or activating any entitlement.

The worker ends at a draft PR and coordinator/owner handoff. It does not canonically accept the gate, merge its own PR, update coordinator-only programme overlays or close issue #115.

## Architecture and source of truth

- `PROVEN`: live task base is `main@bf2a2ae279516f62626a5d8f4dc1aeb587535c62`.
- `PROVEN`: issue #115 is open and requires a bounded Oteryn-v2 consumer/enforcement contract before Profile-B/Premium/VIP or equivalent game-consumed entitlement activation.
- `PROVEN`: `docs/architecture/PROD-ENTITLEMENTS-01_PLATFORM_GAME_ENFORCEMENT_DEPENDENCY.md` pins the satisfied producer prerequisite to Oteryn-Platform PR #968, final head `27414684ceb77700c7bbf7c6a047c6f3c0c79ad9`, merge `afaa6d1d8340e44b1152b62d6d27e5fd1649804a`.
- `PROVEN`: the exact producer contract consumed by this task is `blakinio/Oteryn-Platform/docs/contracts/OTERYN_V2_ENTITLEMENT_GAME_DELIVERY_CONTRACT.md@afaa6d1d8340e44b1152b62d6d27e5fd1649804a`; Platform remains read-only.
- `PROVEN`: current canonical status records `PROD-ENTITLEMENTS-01 = PROPOSED / PLANNED / NOT_STARTED`, consumer contract unaccepted, runtime and Premium/VIP activation unauthorized.
- `PROVEN`: FND-04 is accepted/lifecycle-closed and owns admission, GameSession, CharacterLease and reconnect/recovery authority; this task may consume but not redefine those semantics.
- `PROVEN`: the current active-task set contains no entitlement consumer-contract owner; branch search found no competing `prod-entitlements-115` branch; open-PR search found no competing entitlement delivery, while PRs #311 and #314 explicitly exclude `PROD-ENTITLEMENTS-01`.
- `DERIVED`: the repository owner invoked the exact `OTV2-ENTITLEMENTS-115` lane on 2026-08-17. This lane is not part of the older canonical #258 first-wave A-F allocation; therefore this task records the invocation explicitly and applies the alias's stricter draft-only/coordinator-merge boundary without claiming a new global allocation.
- `UNKNOWN`: exact game-side storage/schema, transport/IDL, entitlement service topology, product catalogue, Premium/VIP feature set, per-product numeric lease/refresh/skew values, trusted-time recovery mechanism, UI presentation details and production rollout values. They remain outside this generic candidate unless evidence makes one indispensable now.

## Acceptance criteria

- [ ] Pin and consume the exact immutable Platform producer contract/revision without copying Platform payment/order/commercial authority into Oteryn-v2.
- [ ] Define deterministic consumer authority states and restrictive precedence for `REVOKED`, `EXPIRED`, `NOT_YET_EFFECTIVE`, `CURRENT_AUTHORITY`, `STALE_WITHIN_BOUND`, `AUTHORITY_UNAVAILABLE` and invalid/conflicting evidence.
- [ ] Define durable lifecycle/authority high-water fencing, replay/rollback behavior and equal-revision conflict handling.
- [ ] Define conservative trusted-time/skew evaluation that can only narrow producer-issued authority intervals.
- [ ] Define fresh-admission, reconnect/recovery and already-running-session boundaries without redefining FND-04 or turning session continuity into entitlement grace.
- [ ] Define product/version consumer policy as fail-closed and never more permissive than the exact producer policy; missing policy cannot become an implicit allow.
- [ ] Define game-affecting delivery operation identity/idempotency/reconciliation without duplicating Platform entitlement or payment truth.
- [ ] Define cross-repository revision pinning, mixed-version compatibility, rollout and rollback invariants.
- [ ] Define bounded observability/audit requirements with secret/private-data redaction.
- [ ] Define the mandatory negative-path/security scenario matrix and expected fail-closed outcomes without claiming runtime PASS.
- [ ] Apply the mandatory architecture decision timing test and explicitly list decisions not taken.
- [ ] Record typed cross-domain findings as `REPORT_ONLY`; do not mutate foreign-owner contracts.
- [ ] Keep changed scope to this task plus one candidate contract and preserve coordinator-only global overlays unchanged.
- [ ] Perform final changed-file/full-diff self-review, security/failure-path audit and applicable exact-head repository CI on one unchanged final head.
- [ ] Require genuinely independent exact-head review before any coordinator/owner canonical acceptance because this contract governs commercial authorization/security and durable anti-rollback behavior.
- [ ] Leave the PR draft with exactly one next action: Architecture Coordinator/owner audit and integration decision.

## Excluded scope

- Rust/client/server/protocol runtime implementation.
- PostgreSQL DDL, migrations or physical entitlement storage schema.
- Payment/order/Wallet implementation or commercial-policy mutation.
- Platform repository writes or competing copies of the Platform producer contract.
- Premium/VIP or any game-consumed entitlement activation.
- Production/protected-environment changes, secrets, live account/session/data mutation or deployment.
- New entitlement transport, crypto primitive, broker, serializer or service topology selection without a separate evidence-backed gate.
- Product catalogue, pricing, tax/legal policy, exact entitlement benefits or per-product numeric authority values.
- Reopening FND-04, DUR-02, DUR-03, ANL-01 or other accepted domain contracts except to report a verified conflict.
- Coordinator-only status/register/horizon/README/handoff/allocation/governance files.
- Owner-funded Codex/OpenAI/paid review without exact separate authorization.

## Implementation / findings

Admission/ownership checks are complete for this bounded lane. The producer prerequisite is satisfied and immutable; the remaining work is genuinely consumer-side. No competing active task, branch or open PR owns the two declared paths.

Design will mirror producer semantics where they are already accepted and add only Oteryn-v2 enforcement choices required to make them executable later. Any missing persistence, product-effect, transport or Platform decision will remain typed `CROSS_DOMAIN_FINDING`/`DECISIONS_NOT_TAKEN` rather than being silently invented here.

## Validation

### Focused

- source/authority/ownership audit: PASS for task admission
- producer-contract verification: PASS at `blakinio/Oteryn-Platform@afaa6d1d8340e44b1152b62d6d27e5fd1649804a`
- governance/document/link validation: pending final head

### Component/integration

- command/run: `NOT_APPLICABLE` — architecture/contract documentation only; no executable component is changed
- result: `NOT_APPLICABLE`

### E2E

- scenario: `NOT_APPLICABLE` — this task defines future executable scenarios but changes no runtime/client/product behavior
- result: `NOT_APPLICABLE`

### Exact-head CI

- final head: pending final freeze
- trigger source: pending draft PR
- workflow/run/job: pending
- runner assignment: pending
- classification: pending
- result: pending

## Self-review

- exact head: pending final freeze
- method/reviewer: implementing domain architecture agent; adversarial full-diff + authority/security/failure-path review
- material findings: pending
- verdict: pending

## Independent review

- required: `YES` — security/authorization-sensitive cross-repository entitlement authority and durable anti-rollback semantics
- exact head: pending coordinator acceptance stage
- method/auditor: genuinely independent non-authoring reviewer/session/workflow; no owner-funded AI without exact authorization
- material findings: pending
- verdict: pending

## PR and closeout

- changed-file review: expected exactly the two declared owned paths
- unresolved review threads: pending
- related/superseded PRs: PR #305 is closed/superseded prompt-package history only; PRs #311/#314 are disjoint and explicitly exclude entitlements
- protected auto-merge: forbidden for this worker lane
- merge commit/result: coordinator-only / pending
- ownership release: coordinator-only after any accepted merge/closeout

## Context checkpoint

```yaml
last_progress: Verified exact main, issue #115, immutable Platform producer remediation/contract, accepted FND-04 boundary, active ownership and open-PR/branch non-overlap; dedicated branch created and task admitted.
status: implementing
branch: docs/prod-entitlements-115-consumer-contract
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
next_action: create the bounded PROD-ENTITLEMENTS-01 game-consumer/enforcement candidate contract and open/update its draft PR
```
