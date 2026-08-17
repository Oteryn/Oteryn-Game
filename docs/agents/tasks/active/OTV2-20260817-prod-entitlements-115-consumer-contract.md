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
head_sha: 2d2a7402e152e7bbe7f1c6c8c5da8248dd0a65a0
final_head_sha: null
final_head_frozen_at: null
owner: OTV2-ENTITLEMENTS-115/current-session
created_at: 2026-08-17T08:44:00+02:00
updated_at: 2026-08-17T08:55:00+02:00
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

- `PROVEN`: original task base was `main@bf2a2ae279516f62626a5d8f4dc1aeb587535c62`; branch was later reconciled with Stage-C main `e0ea9ef87c01dec720a22e8df6d54bfd669cb62c` with no owned-path or entitlement semantic overlap.
- `PROVEN`: issue #115 remains open and requires a bounded Oteryn-v2 consumer/enforcement contract before game-consumed entitlement activation.
- `PROVEN`: Platform producer prerequisite is satisfied by PR #968, final head `27414684ceb77700c7bbf7c6a047c6f3c0c79ad9`, merge `afaa6d1d8340e44b1152b62d6d27e5fd1649804a`.
- `PROVEN`: exact producer contract consumed is `blakinio/Oteryn-Platform/docs/contracts/OTERYN_V2_ENTITLEMENT_GAME_DELIVERY_CONTRACT.md@afaa6d1d8340e44b1152b62d6d27e5fd1649804a`; Platform remained read-only.
- `PROVEN`: canonical baseline records `PROD-ENTITLEMENTS-01 = PROPOSED / PLANNED / NOT_STARTED`, consumer contract unaccepted, runtime/Premium-VIP activation unauthorized.
- `PROVEN`: FND-04 is accepted/lifecycle-closed and owns admission, GameSession, CharacterLease and reconnect/recovery; this candidate consumes and does not redefine it.
- `PROVEN`: admission search found no competing active entitlement task, matching branch or overlapping open PR. PRs #311/#314 explicitly exclude entitlements.
- `DERIVED`: owner invocation of exact alias `OTV2-ENTITLEMENTS-115` launches this bounded lane. It is outside the older canonical #258 A-F allocation; this task uses the alias's safer draft-only/coordinator-merge restriction without claiming a new global allocation.
- `UNKNOWN`: physical entitlement persistence/inbox/cursor, transport/IDL, topology, concrete product catalogue/benefits, numeric lease/refresh/skew values, trusted-time recovery mechanism, UI behavior and production rollout values remain future evidence-backed decisions.

## Acceptance criteria

- [x] Pin and consume exact immutable Platform producer contract/revision without copying Platform commercial/payment authority into Oteryn-v2.
- [x] Define deterministic authority classifications, current-revision conflict handling and producer-compatible restrictive precedence.
- [x] Define durable lifecycle/authority high-water fencing, rollback/replay behavior and crash-consistent producer consume-progress coupling.
- [x] Define conservative trusted-time/skew evaluation that only narrows producer-issued authority intervals.
- [x] Define fresh admission, reconnect/recovery and running-session boundaries without redefining FND-04 or converting session continuity into entitlement grace.
- [x] Define explicit product/version consumer surface policy that can be stricter but never more permissive than producer policy.
- [x] Define stable game-affecting delivery identity/idempotency/reconciliation without duplicating Platform entitlement/payment truth.
- [x] Define exact producer/consumer compatibility, mixed-version rollout and rollback invariants.
- [x] Define bounded secret-safe observability/audit.
- [x] Define negative security scenarios including unseen revoke in partition, crash after observed revoke before fence commit, expiry, restart/rollback, clock rollback, duplicate grant, semantic downgrade and refresh failure without runtime PASS claims.
- [x] Apply architecture decision timing and preserve decisions not taken.
- [x] Record typed `REPORT_ONLY` cross-domain findings without foreign-owner mutation.
- [x] Keep changed scope to task + candidate; coordinator-only overlays untouched.
- [ ] Complete final unchanged-head full-diff self-review and exact-head repository CI.
- [x] Require genuinely independent exact-head review before canonical acceptance.
- [ ] End in `INTEGRATION_READY — DRAFT PR — COORDINATOR/OWNER ACTION REQUIRED` with one next action after exact-head CI classification.

## Excluded scope

- Runtime/client/server/protocol implementation.
- PostgreSQL DDL/migrations or physical storage/inbox/cursor schema.
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

The candidate now freezes:

1. Platform commercial authority vs Oteryn gameplay enforcement without shared commercial truth.
2. Durable `(lifecycle_revision, authority_revision)` high water.
3. Crash-consistent consume-and-fence: producer ack/cursor/continuity may not outrun durable high water; crash after observing restrictive evidence cannot silently fall back to old active cache.
4. Current-revision conflict handling distinct from stale lower-revision rejection.
5. Conservative `[now_lower, now_upper]` evaluation; uncertainty only narrows authority.
6. Typed/equivalent `REVOKED`, `EXPIRED`, `NOT_YET_EFFECTIVE`, `CURRENT_AUTHORITY`, `STALE_WITHIN_BOUND`, `AUTHORITY_UNAVAILABLE`, `INVALID_OR_CONFLICTING` semantics.
7. Explicit fresh-admission/reconnect/running-session surface policy; Oteryn may only narrow producer authority.
8. Base FND-04 session may continue when benefit disappears; no authoritative action after cutoff may rely on benefit.
9. Stable game-delivery operation identity/reconciliation for separately approved gameplay-mutating profiles.
10. Exact producer/consumer compatibility and rollback that never lowers fence, rewinds consumption progress, resets lease or guesses semantics.
11. Secret-safe observability plus future fault matrix.

### Repair cycle 1 — pre-freeze authority/state audit

Repaired three material ambiguities:

1. current high-water authenticated conflict now precedes time/state classification; stale lower revisions are rejected without poisoning newer truth;
2. unseen revoke during partition now explicitly permits only previously provable bounded stale behavior until the existing cutoff, not fictitious instantaneous revocation;
3. product/version provenance may change only through a strictly newer Platform lifecycle decision plus explicit producer migration/replacement compatibility rule.

Also added clock-rollback, semantic-downgrade and refresh-failure cases.

### Repair cycle 2 — crash-window anti-resurrection audit

The next exact-diff security pass found one P1 design gap: `deny immediately, persist later` alone cannot prove that a crash between observing a revoke and persisting its high water will not resurrect the older active cache.

Repaired by requiring an implementation-independent **crash-consistent consume-and-fence property**:

- producer event ack/cursor/receipt progress cannot advance past evidence whose high-water fence is not durably committed;
- query/snapshot integrations must refetch/re-prove current authority after uncertain restart before old cached active evidence can authorize;
- restrictive evidence may deny immediately, but entitlement remains denied/quarantined until durable fence or current authority continuity is re-established;
- future persistence and transport owners must jointly prove this property without this candidate choosing a concrete transaction/inbox/cursor mechanism.

Repair budget: `2/3`. Any third material repair remains possible; a fourth is not authorized by the ordinary gate budget.

## Validation

### Focused

- source/authority/ownership audit: PASS
- exact Platform producer contract: PASS at `afaa6d1d8340e44b1152b62d6d27e5fd1649804a`
- Stage-C main drift reconciliation: PASS; disjoint paths and entitlement explicitly excluded
- changed-file scope before final metadata commit: exactly two owned paths
- adversarial authority/security/failure-path audit: two repair cycles completed
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
- method/reviewer: implementing domain architecture agent; full two-path diff + producer fidelity + authority/security/failure-path audit
- material findings: repair cycles 1-2 closed four material ambiguities before freeze; terminal unchanged-head pass pending
- verdict: pending

## Independent review

- required: `YES` — authorization/security-sensitive cross-repository authority and durable anti-rollback/consume-progress semantics
- exact head: pending coordinator acceptance stage
- method/auditor: genuinely independent non-authoring reviewer/session/workflow; no owner-funded AI without exact authorization
- material findings/verdict: pending

## PR and closeout

- PR: #317 draft
- changed-file review: exactly task + candidate expected
- unresolved review threads: pending
- related/superseded: PR #305 closed/superseded prompt history only; #311/#314 disjoint/entitlement-excluding
- protected auto-merge: forbidden for worker lane
- merge: coordinator-only / pending
- ownership release: coordinator-only after accepted merge/closeout

## Context checkpoint

```yaml
last_progress: Closed crash-window anti-resurrection gap by coupling producer consumption/continuity to durable authority high water; candidate is ready for current-main reconciliation, final unchanged-head self-review and CI.
status: validating
branch: docs/prod-entitlements-115-consumer-contract
head_sha: 2d2a7402e152e7bbe7f1c6c8c5da8248dd0a65a0
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
repair_cycles_for_current_gate: 2
ci_recovery_actions_for_current_head: 0
stall_warnings: 0
owner_action_required: null
blocker: null
next_action: reconcile against current main if needed, then perform terminal unchanged-head full-diff self-review and classify exact-head merge-gate CI for draft PR #317
```
