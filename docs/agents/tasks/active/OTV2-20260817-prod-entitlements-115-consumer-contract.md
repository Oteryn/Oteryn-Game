# OTV2-20260817-prod-entitlements-115-consumer-contract

```yaml
task_id: OTV2-20260817-prod-entitlements-115-consumer-contract
title: PROD-ENTITLEMENTS-01 Oteryn-v2 consumer/enforcement contract
mode: CONTRACT
status: validating
repository: blakinio/Oteryn-v2
base_branch: main
branch: docs/prod-entitlements-115-consumer-contract
pr: 317
base_sha: bf2a2ae279516f62626a5d8f4dc1aeb587535c62
head_sha: ff86f9c32da9852ab986e77bb6ea845bd874e180
final_head_sha: null
final_head_frozen_at: null
owner: OTV2-ENTITLEMENTS-115/current-session
created_at: 2026-08-17T08:44:00+02:00
updated_at: 2026-08-17T09:01:00+02:00
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

Produce one bounded paper-only `PROD-ENTITLEMENTS-01` candidate defining the Oteryn-v2 game-consumer/enforcement side of the accepted Platform commercial-entitlement authority split. The worker ends at draft PR/coordinator handoff and does not accept the gate, merge its own PR, update coordinator-only overlays or close issue #115.

## Architecture and source of truth

- `PROVEN`: original task base was `main@bf2a2ae279516f62626a5d8f4dc1aeb587535c62`; branch was reconciled with Stage-C main `e0ea9ef87c01dec720a22e8df6d54bfd669cb62c` with no owned-path or entitlement semantic overlap.
- `PROVEN`: issue #115 remains open and requires this consumer/enforcement contract before game-consumed entitlement activation.
- `PROVEN`: Platform producer prerequisite is satisfied by PR #968, final head `27414684ceb77700c7bbf7c6a047c6f3c0c79ad9`, merge `afaa6d1d8340e44b1152b62d6d27e5fd1649804a`.
- `PROVEN`: exact producer contract consumed is `blakinio/Oteryn-Platform/docs/contracts/OTERYN_V2_ENTITLEMENT_GAME_DELIVERY_CONTRACT.md@afaa6d1d8340e44b1152b62d6d27e5fd1649804a`; Platform remained read-only.
- `PROVEN`: canonical baseline records `PROD-ENTITLEMENTS-01 = PROPOSED / PLANNED / NOT_STARTED`, consumer contract unaccepted, runtime/Premium-VIP activation unauthorized.
- `PROVEN`: FND-04 is accepted/lifecycle-closed and owns admission, GameSession, CharacterLease and reconnect/recovery; this candidate consumes and does not redefine it.
- `PROVEN`: admission search found no competing active entitlement task, matching branch or overlapping open PR; Stage-C work explicitly excluded entitlements.
- `DERIVED`: owner invocation of exact alias `OTV2-ENTITLEMENTS-115` launches this bounded lane outside the older #258 A-F allocation; the stricter alias boundary keeps the PR draft and coordinator-only for merge/acceptance.
- `UNKNOWN`: physical entitlement persistence/revision-fingerprint/inbox/cursor design, transport/IDL, topology, concrete product catalogue/benefits, numeric lease/refresh/skew values, trusted-time recovery mechanism, UI behavior and production rollout remain future evidence-backed decisions.

## Acceptance criteria

- [x] Consume exact immutable Platform producer semantics without copying Platform commercial/payment authority into Oteryn-v2.
- [x] Define deterministic authority classifications and producer-compatible restrictive precedence.
- [x] Define fail-closed handling for authenticated same-ordered-revision contradiction at current or historical revisions.
- [x] Define durable lifecycle/authority high-water fencing, rollback/replay behavior and crash-consistent producer consume-progress coupling.
- [x] Define conservative trusted-time/skew evaluation that only narrows producer-issued authority intervals.
- [x] Define fresh admission, reconnect/recovery and running-session boundaries without redefining FND-04 or converting session continuity into entitlement grace.
- [x] Define explicit product/version consumer surface policy that can be stricter but never more permissive than producer policy.
- [x] Define stable game-affecting delivery identity/idempotency/reconciliation without duplicating Platform entitlement/payment truth.
- [x] Define exact producer/consumer compatibility, mixed-version rollout and rollback invariants.
- [x] Define bounded secret-safe observability/audit.
- [x] Define negative security scenarios including unseen revoke, crash-before-fence, equivocation, expiry, restart/rollback, clock rollback, duplicate grant, semantic downgrade and refresh failure without runtime PASS claims.
- [x] Apply architecture decision timing and preserve decisions not taken.
- [x] Record typed `REPORT_ONLY` cross-domain findings without foreign-owner mutation.
- [x] Keep changed scope to task + candidate; coordinator-only overlays untouched.
- [ ] Complete final unchanged-head full-diff self-review and exact-head repository CI.
- [x] Require genuinely independent exact-head review before canonical acceptance.
- [ ] End in `INTEGRATION_READY — DRAFT PR — COORDINATOR/OWNER ACTION REQUIRED` with one next action after exact-head CI classification.

## Excluded scope

- Runtime/client/server/protocol implementation.
- PostgreSQL DDL/migrations or physical storage/revision-fingerprint/inbox/cursor schema.
- Payment/order/Wallet implementation or commercial-policy mutation.
- Platform repository writes.
- Premium/VIP or other game-consumed entitlement activation.
- Production/protected environment, secrets, live accounts/sessions/data or deployment.
- Concrete entitlement transport/crypto/broker/serializer/topology without separate evidence-backed gate.
- Product catalogue/pricing/legal/tax/exact benefits or numeric authority values.
- Reopening accepted FND-04/DUR-02/DUR-03/ANL-01 except verified conflict reporting.
- Coordinator-only status/register/horizon/README/handoff/allocation/governance files.
- Owner-funded Codex/OpenAI/paid review without exact separate authorization.

## Implementation / findings

The candidate freezes:

1. Platform commercial authority vs Oteryn gameplay enforcement without shared commercial truth.
2. Durable `(lifecycle_revision, authority_revision)` high water.
3. Detected authenticated producer equivocation for one ordered revision always fails closed, even when historical/stale.
4. Crash-consistent consume-and-fence: producer ack/cursor/continuity cannot outrun durable high water; crash after restrictive evidence cannot silently restore old active cache.
5. Conservative `[now_lower, now_upper]` evaluation; uncertainty only narrows authority.
6. Typed/equivalent `REVOKED`, `EXPIRED`, `NOT_YET_EFFECTIVE`, `CURRENT_AUTHORITY`, `STALE_WITHIN_BOUND`, `AUTHORITY_UNAVAILABLE`, `INVALID_OR_CONFLICTING` semantics.
7. Explicit fresh-admission/reconnect/running-session surface policy; Oteryn may only narrow producer authority.
8. Base FND-04 session may continue when benefit disappears; no authoritative action after cutoff may rely on benefit.
9. Stable game-delivery operation identity/reconciliation for separately approved gameplay-mutating profiles.
10. Exact producer/consumer compatibility and rollback that never lowers fence, rewinds consumption progress, loses claimed equivocation evidence, resets lease or guesses semantics.
11. Secret-safe observability plus future fault matrix.

### Repair cycle 1 — authority/state audit

Closed three material ambiguities: current-authority conflict precedence, unseen revocation during partition, and explicit producer-governed product/version migration. Also added clock-rollback, semantic-downgrade and refresh-failure scenarios.

### Repair cycle 2 — crash-window anti-resurrection audit

Closed one P1 gap where a crash after observing restrictive evidence but before persisting its high water could otherwise resurrect an older active cache. Added crash-consistent consume-and-fence/ack-continuity requirements independent of concrete transport or storage.

### Repair cycle 3 — exact producer-fidelity audit

The final producer comparison found that the pinned Platform contract requires **any** same-ordered-revision authenticated contradiction to fail closed, while the second draft treated contradiction at an already superseded historical revision as ordinary stale replay.

Repaired by:

- detecting authenticated contradiction before stale-order rejection;
- classifying any detected same-revision equivocation as `INVALID_OR_CONFLICTING`;
- requiring enough bounded revision/fingerprint evidence to support the claimed replay/rollback/recovery horizon;
- adding historical-equivocation matrix/observability/rollback requirements.

Repair budget is now **`3/3` exhausted**. Any further material finding is a `BLOCKED` outcome rather than a fourth ordinary repair.

## Validation

### Focused

- source/authority/ownership audit: PASS
- exact Platform producer contract: PASS at `afaa6d1d8340e44b1152b62d6d27e5fd1649804a` after repair cycle 3
- Stage-C main drift reconciliation: PASS; disjoint paths and entitlement explicitly excluded
- changed-file scope before final metadata commit: exactly two owned paths
- adversarial authority/security/failure-path audit: repair cycles `3/3` completed
- governance/document/link validation: pending exact frozen head

### Component/integration

- `NOT_APPLICABLE` — architecture/contract docs only; no executable component changed

### E2E

- `NOT_APPLICABLE` — future scenarios are specified but no runtime/client/product behavior exists

### Exact-head CI

- final head: pending after this repository metadata commit and any current-main reconciliation
- trigger source: pull request #317
- workflow/run/job: pending
- runner assignment: pending
- classification/result: pending

## Self-review

- exact head: pending final freeze
- method/reviewer: implementing domain architecture agent; full two-path diff + exact producer fidelity + authority/security/failure-path audit
- material findings: three repair cycles closed all findings identified before freeze; terminal unchanged-head pass pending
- verdict: pending

## Independent review

- required: `YES` — authorization/security-sensitive cross-repository authority, durable anti-rollback, consume-progress and equivocation semantics
- exact head: pending coordinator acceptance stage
- method/auditor: genuinely independent non-authoring reviewer/session/workflow; no owner-funded AI without exact authorization
- material findings/verdict: pending

## PR and closeout

- PR: #317 draft
- changed-file review: exactly task + candidate expected
- unresolved review threads: pending
- related/superseded: PR #305 closed/superseded prompt history only; Stage-C/current work disjoint and entitlement-excluding
- protected auto-merge: forbidden for worker lane
- merge: coordinator-only / pending
- ownership release: coordinator-only after accepted merge/closeout

## Context checkpoint

```yaml
last_progress: Exact producer-fidelity repair completed; detected same-ordered-revision equivocation now always fails closed. Repair budget 3/3 exhausted and candidate is ready only for terminal unchanged-head review/CI.
status: validating
branch: docs/prod-entitlements-115-consumer-contract
head_sha: ff86f9c32da9852ab986e77bb6ea845bd874e180
pr: 317
final_head_sha: null
final_head_frozen_at: null
ci_trigger_source: pull_request #317
ci_check_generation: pending final freeze
ci_checks_for_current_head: 0
ci_run_ids: []
ci_job_ids: []
runner_assignment_state: unknown
terminal_ci_wait_started_at: null
terminal_ci_checks_for_current_generation: 0
unchanged_state_checks: 0
identical_failure_retries: 0
repair_cycles_for_current_gate: 3
ci_recovery_actions_for_current_head: 0
stall_warnings: 0
owner_action_required: null
blocker: null
next_action: verify current main, freeze exact PR head, perform terminal full-diff self-review, then classify exact-head merge-gate CI without further content repair
```
