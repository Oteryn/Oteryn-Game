# OTV2-20260808-fnd04-crossrepo-security-reconciliation

```yaml
task_id: OTV2-20260808-fnd04-crossrepo-security-reconciliation
title: Reconcile FND-04 analysis with current Platform pre-admission security semantics
mode: CONTRACT
status: implementing
repository: blakinio/Oteryn-v2
base_branch: main
branch: docs/OTV2-20260808-fnd04-crossrepo-security-reconciliation
pr: null
base_sha: c638ad524772f227dabc90e88a1381cc01e907ce
head_sha: null
final_head_sha: null
final_head_frozen_at: null
owner: GPT-5.6 Sol architecture continuation session
created_at: 2026-08-08T21:09:00+02:00
updated_at: 2026-08-08T21:09:00+02:00
execution_budget_minutes: 60
large_budget_reason: null
owned_paths:
  - docs/agents/tasks/active/OTV2-20260808-fnd04-crossrepo-security-reconciliation.md
  - docs/architecture/FND-04_CROSS_REPOSITORY_SECURITY_RECONCILIATION_ADDENDUM.md
public_contracts:
  - docs/architecture/FND-04_CROSS_REPOSITORY_SECURITY_RECONCILIATION_ADDENDUM.md
depends_on:
  - docs/architecture/FND-04_SESSION_ADMISSION_LEASE_ANALYSIS_BASELINE.md
  - docs/architecture/FND-02_PROTOCOL_OTERYN_V1_CONTRACT.md
  - docs/architecture/FND-03_RUNTIME_EXECUTION_CONTRACT.md
  - blakinio/Oteryn-Platform@216f5b2817e9d102337608609e344518512c2a0d read-only architecture/contracts
blocks:
  - final FND-04 Identity Game Session Admission and Character Lease Contract
cross_repository_coordination_id: OTV2-NATIVE-FOUNDATION
external_repositories:
  - blakinio/Oteryn-Platform (read-only evidence only)
```

## Outcome

Add one non-overlapping architecture correction that makes the final FND-04 contract consume current Platform pre-admission/runtime-status security semantics that were not fully dispositioned by merged analysis PR #104, without rewriting the historical analysis baseline or implementing runtime behavior.

## Architecture and source of truth

- **PROVEN:** PR #104 merged the FND-04 analysis baseline as `c638ad524772f227dabc90e88a1381cc01e907ce`; that baseline is analysis input only, not final FND-04 authority.
- **PROVEN:** current Platform `OTERYN_V2_PRE_ADMISSION_HANDOFF_CONTRACT.md` requires explicit disposition of account-security changes after grant issuance, stale runtime/owner-generation binding and producer-side ambiguous-issuance idempotency.
- **PROVEN:** current Platform runtime-status contract separates configured routing policy from fresh current-owner runtime evidence and fails closed for stale/superseded ownership evidence.
- **PROVEN:** the still-active PR #104 task owns the merged analysis-baseline path, so this correction uses a separate addendum path and does not mutate that task's owned files.
- **DERIVED:** final FND-04 would remain security-incomplete if it froze hybrid signed grants without defining these three cross-repository semantics.
- **UNKNOWN:** exact account-security revocation primitive, exact runtime-generation field encoding, exact producer issuance API/transport and storage implementation.

## Acceptance criteria

- [ ] Preserve the merged FND-04 analysis baseline unchanged and add an explicit normative-input addendum for the final contract.
- [ ] Require a testable disposition for Platform account-security changes after a PreAdmissionGrant is issued but before game admission.
- [ ] Require an explicit rule for issuance-time runtime observation/ownership-generation binding and invalidation of stale unexpired grants.
- [ ] Preserve Platform admission-attempt idempotency/reconciliation identity as distinct from game-domain grant-consume nonce unless equivalence is explicitly proven.
- [ ] State decision timing, blocked downstream work, risks, superseding evidence and deliberately deferred implementation choices.
- [ ] Keep Platform read-only and introduce no runtime/protocol/persistence/deployment implementation.
- [ ] Full changed-file review, independent architecture/security audit and exact-head required CI pass before merge.

## Excluded scope

- no edits to the already-merged `FND-04_SESSION_ADMISSION_LEASE_ANALYSIS_BASELINE.md`;
- no edits to the existing PR #104 active task;
- no final FND-04 contract yet;
- no Rust code, protobuf/schema registration, PostgreSQL schema or migration;
- no Platform/Gateway write;
- no JWT/JWS library, KMS/HSM vendor, mTLS product, revocation datastore or numeric TTL selection;
- no production traffic, keys, sessions, accounts or deployment changes.

## Implementation / findings

A separate addendum is required because the merged analysis remains useful but its external reconciliation evidence predates the current Platform native pre-admission/runtime-status contracts. The correction must constrain the later final FND-04 contract without pretending to retroactively change PR #104 history.

## Validation

### Focused

- command/run: full semantic reconciliation against current Platform ADR 0031, `OTERYN_V2_PRE_ADMISSION_HANDOFF_CONTRACT.md`, `OTERYN_V2_RUNTIME_STATUS_PROJECTION_CONTRACT.md` and merged FND-04 analysis
- result: pending

### Component/integration

- command/run: `NOT_APPLICABLE` — architecture/documentation only
- result: `NOT_APPLICABLE`

### E2E

- scenario: `NOT_APPLICABLE` — no executable capability changes
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
- method/auditor: independent cross-repository architecture/security review
- material findings: pending
- verdict: pending

## PR and closeout

- changed-file review: pending
- unresolved review threads: pending
- related/superseded PRs: merged #104 is dependency/history, not superseded
- protected auto-merge: pending
- merge commit/result: pending
- ownership release: pending

## Context checkpoint

```yaml
last_progress: Created a non-overlapping follow-up task after independent review found three current Platform security semantics not fully dispositioned by merged FND-04 analysis PR #104.
status: implementing
branch: docs/OTV2-20260808-fnd04-crossrepo-security-reconciliation
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
next_action: Add the bounded FND-04 cross-repository security reconciliation addendum and inspect the two-path diff.
```
