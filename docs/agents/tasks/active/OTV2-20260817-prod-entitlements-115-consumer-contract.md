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
head_sha: 580eb22add154c51efb7f0bd77a511c626e7861e
final_head_sha: null
final_head_frozen_at: null
owner: OTV2-ENTITLEMENTS-115/current-session
created_at: 2026-08-17T08:44:00+02:00
updated_at: 2026-08-17T08:51:00+02:00
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

- `PROVEN`: task base is `main@bf2a2ae279516f62626a5d8f4dc1aeb587535c62`.
- `PROVEN`: issue #115 remains open and requires a bounded Oteryn-v2 consumer/enforcement contract before game-consumed entitlement activation.
- `PROVEN`: Platform producer prerequisite is satisfied by PR #968, final head `27414684ceb77700c7bbf7c6a047c6f3c0c79ad9`, merge `afaa6d1d8340e44b1152b62d6d27e5fd1649804a`.
- `PROVEN`: exact producer contract consumed is `blakinio/Oteryn-Platform/docs/contracts/OTERYN_V2_ENTITLEMENT_GAME_DELIVERY_CONTRACT.md@afaa6d1d8340e44b1152b62d6d27e5fd1649804a`; Platform remained read-only.
- `PROVEN`: canonical live baseline records `PROD-ENTITLEMENTS-01 = PROPOSED / PLANNED / NOT_STARTED`, consumer contract unaccepted, runtime/Premium-VIP activation unauthorized.
- `PROVEN`: FND-04 is accepted/lifecycle-closed and owns admission, GameSession, CharacterLease and reconnect/recovery; this candidate consumes and does not redefine it.
- `PROVEN`: admission search found no competing active entitlement task, matching branch or overlapping open PR. PRs #311/#314 explicitly exclude entitlements.
- `DERIVED`: owner invocation of exact alias `OTV2-ENTITLEMENTS-115` launches this bounded lane. It is outside the older canonical #258 A-F allocation; the task therefore records the invocation and uses the alias's safer draft-only/coordinator-merge restriction without claiming a new global allocation.
- `UNKNOWN`: physical entitlement persistence, transport/IDL, service topology, concrete product catalogue/benefits, numeric lease/refresh/skew values, trusted-time recovery mechanism, UI behavior and production rollout values remain future evidence-backed decisions.

## Acceptance criteria

- [x] Pin and consume the exact immutable Platform producer contract/revision without copying Platform commercial/payment authority into Oteryn-v2.
- [x] Define deterministic authority classifications, current-revision conflict handling and producer-compatible restrictive precedence.
- [x] Define durable lifecycle/authority high-water fencing, replay/rollback behavior and fence-before-permissive-authorize.
- [x] Define conservative trusted-time/skew evaluation that only narrows producer-issued authority intervals.
- [x] Define fresh admission, reconnect/recovery and already-running-session boundaries without redefining FND-04 or converting session continuity into entitlement grace.
- [x] Define explicit product/version consumer surface policy that can be stricter but never more permissive than producer policy.
- [x] Define stable game-affecting delivery operation identity/idempotency/reconciliation without duplicating Platform entitlement/payment truth.
- [x] Define exact producer/consumer revision compatibility, mixed-version rollout and rollback invariants.
- [x] Define bounded observability/audit requirements with secret/private-data redaction.
- [x] Define negative security scenarios including unseen revocation during partition, expiry, restart/rollback, clock rollback, duplicate grant, semantic downgrade and refresh failure without claiming runtime PASS.
- [x] Apply architecture decision timing and explicitly preserve decisions not taken.
- [x] Record typed `REPORT_ONLY` cross-domain findings without mutating foreign-owner contracts.
- [x] Keep changed scope to this task plus one candidate contract; coordinator-only overlays remain untouched.
- [ ] Complete final exact-head changed-file/full-diff self-review and ordinary repository CI.
- [x] Candidate explicitly requires genuinely independent exact-head review before canonical acceptance because this gate is authorization/security sensitive.
- [ ] End in `INTEGRATION_READY — DRAFT PR — COORDINATOR/OWNER ACTION REQUIRED` with one next action after exact-head CI is classified.

## Excluded scope

- Rust/client/server/protocol runtime implementation.
- PostgreSQL DDL/migrations or physical entitlement storage schema.
- Payment/order/Wallet implementation or commercial-policy mutation.
- Platform repository writes.
- Premium/VIP or other game-consumed entitlement activation.
- Production/protected environment, secrets, live accounts/sessions/data or deployment.
- Concrete entitlement transport/crypto/broker/serializer/service topology without a separate evidence-backed gate.
- Product catalogue/pricing/legal/tax/exact benefits or numeric authority values.
- Reopening accepted FND-04/DUR-02/DUR-03/ANL-01 contracts except to report verified conflict.
- Coordinator-only status/register/horizon/README/handoff/allocation/governance files.
- Owner-funded Codex/OpenAI/paid review without exact separate authorization.

## Implementation / findings

The candidate freezes:

1. Platform commercial authority vs Oteryn gameplay enforcement without shared truth.
2. Durable `(lifecycle_revision, authority_revision)` high-water and fence-before-permissive-use.
3. Fail-closed current-revision conflict handling distinct from stale lower-revision rejection.
4. Conservative `[now_lower, now_upper]` evaluation: uncertainty delays start and advances local deny near end; receipt/restart time never creates lease.
5. Typed/equivalent `REVOKED`, `EXPIRED`, `NOT_YET_EFFECTIVE`, `CURRENT_AUTHORITY`, `STALE_WITHIN_BOUND`, `AUTHORITY_UNAVAILABLE`, `INVALID_OR_CONFLICTING` semantics.
6. Explicit fresh-admission/reconnect/running-session surface policy; missing policy is deny and Oteryn may only narrow producer authority.
7. Base FND-04 session may continue when entitlement benefit disappears; no authoritative action after cutoff may rely on the benefit.
8. Stable delivery operation identity/reconciliation for separately approved gameplay-mutating entitlement profiles.
9. Exact producer/consumer compatibility records and rollback that never lowers high water, resets lease or guesses semantic compatibility.
10. Secret-safe observability plus future implementation matrix covering outage/revoke/restart/rollback/clock/idempotency/mixed-version failures.

### Repair cycle 1 — adversarial pre-freeze security self-audit

The initial full two-path diff exposed three material ambiguities and they were repaired before final freeze:

1. **Conflict precedence:** an authenticated contradiction at the current high-water revision could be read as lower-priority than a time-derived classification. Repaired by separating input/high-water validation from producer-compatible state precedence; current-revision conflict is now `INVALID_OR_CONFLICTING`, while stale lower revisions are rejected without poisoning newer truth.
2. **Unseen revocation during partition:** the first matrix did not state that a revoke not yet observable to Oteryn cannot be claimed instantaneous. Repaired with an explicit scenario: only the previously provable bounded stale state may continue when allowed, never beyond the pre-existing finite cutoff; after healing, the newer revoke fences old active evidence.
3. **Product/version provenance migration:** initial immutable-provenance wording could over-block a legitimate explicit producer migration. Repaired so contradiction remains fail closed unless a strictly newer Platform lifecycle decision plus accepted producer migration/replacement compatibility rule authorizes that exact transition.

The same repair expanded clock-rollback, semantic-downgrade and refresh-failure scenarios. Repair budget: `1/3`.

## Validation

### Focused

- source/authority/ownership audit: PASS
- exact Platform producer-contract verification: PASS at `afaa6d1d8340e44b1152b62d6d27e5fd1649804a`
- changed-file scope: exactly the two owned paths before final metadata freeze
- security/failure-path pre-freeze audit: repair cycle 1 completed
- governance/document/link validation: pending exact frozen head

### Component/integration

- command/run: `NOT_APPLICABLE` — architecture/contract documentation only; no executable component changed
- result: `NOT_APPLICABLE`

### E2E

- scenario: `NOT_APPLICABLE` — candidate defines future executable scenarios but introduces no runtime/client/product behavior
- result: `NOT_APPLICABLE`

### Exact-head CI

- final head: pending after this final repository metadata commit
- trigger source: pull request #317
- workflow/run/job: pending
- runner assignment: pending
- classification: pending
- result: pending

## Self-review

- exact head: pending after final repository metadata commit
- method/reviewer: implementing domain architecture agent; full two-path diff + producer fidelity + authority/security/failure-path audit
- material findings: repair cycle 1 repaired three material ambiguities before freeze; terminal unchanged-head pass pending
- verdict: pending

## Independent review

- required: `YES` — security/authorization-sensitive cross-repository entitlement authority and durable anti-rollback semantics
- exact head: pending coordinator acceptance stage
- method/auditor: genuinely independent non-authoring reviewer/session/workflow; no owner-funded AI without exact authorization
- material findings: pending
- verdict: pending

## PR and closeout

- PR: #317 draft
- changed-file review: exactly task + candidate expected
- unresolved review threads: pending
- related/superseded PRs: PR #305 closed/superseded prompt history only; #311/#314 disjoint and entitlement-excluding
- protected auto-merge: forbidden for worker lane
- merge commit/result: coordinator-only / pending
- ownership release: coordinator-only after accepted merge/closeout

## Context checkpoint

```yaml
last_progress: Candidate authored and repaired after adversarial full-diff security audit; producer fidelity, session boundary, finite-time authority, durable anti-rollback, partition, rollback and idempotency semantics are now frozen for exact-head validation.
status: validating
branch: docs/prod-entitlements-115-consumer-contract
head_sha: 580eb22add154c51efb7f0bd77a511c626e7861e
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
repair_cycles_for_current_gate: 1
ci_recovery_actions_for_current_head: 0
stall_warnings: 0
owner_action_required: null
blocker: null
next_action: perform final unchanged-head full-diff self-review and classify exact-head merge-gate CI for draft PR #317
```
