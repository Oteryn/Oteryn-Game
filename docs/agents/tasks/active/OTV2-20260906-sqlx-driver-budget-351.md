# OTV2-20260906-sqlx-driver-budget-351

```yaml
task_id: OTV2-20260906-sqlx-driver-budget-351
title: Enforce SQLx PostgreSQL driver accounting within accepted admission budgets
mode: IMPLEMENT
status: waiting
admission_state: NOT_ADMITTED
repository: Oteryn/Oteryn-Game
base_branch: main
branch: agent/sqlx-driver-budget-351
issue: 351
pr: null
allocation_source_main_sha: d9d1b566acb57b537ff901d9765c32a95110c259
admission_main_sha: NOT_ADMITTED
base_sha: NOT_ADMITTED
head_sha: null
final_head_sha: null
final_head_frozen_at: null
owner: prospective sole SQLx driver worker
coordinator: Oteryn Work Delivery Coordinator
created_at: 2026-09-06
updated_at: 2026-09-06
execution_budget_minutes: 60
large_budget_reason: null
owned_paths:
  - vendor/sqlx-postgres-0.9.0/**
  - Cargo.toml
  - Cargo.lock
  - docs/agents/tasks/active/OTV2-20260906-sqlx-driver-budget-351.md
  - docs/superpowers/plans/2026-09-06-sqlx-driver-budget.md
public_contracts: [DUR-FRESH-RESOURCE-ENVELOPE-V1]
depends_on: [341, 342, protected_351_allocation, exclusive_247_Cargo_lease]
blocks: [B329_complete_driver_resource_qualification]
cross_repository_coordination_id: null
external_repositories: []
```

## Outcome and authority

Provide a reviewed SQLx PostgreSQL0.9.0 driver capability that denies size-controlled allocations before exhausting the existing admission slot balance. This prerequisite does not activate B or release247. Read the accepted resource decision337/341, RESOURCE_LIMITS_REGISTRY and the exact LIVE351 lease. The implementation plan is the single canonical plan for this task.

PROVEN: protected main d9d1b566acb57b537ff901d9765c32a95110c259 contains accepted resource341 and completed registry342. Actual SQLx0.9.0 receive buffering reserves peer-announced lengths before body arrival; row count decoders and retained status/type/table storage require coverage. Preserved evidence: PR350 at0131d990eab956462b389374b2e725427c6de251, resource-blocker section. DERIVED: a private bounded PostgreSQL stream using public SQLx-core socket interfaces is the smallest candidate patch. UNKNOWN: whether this single-crate scope suffices for every dependency, particularly TLS/socket allocations. No impossibility or complete-bound claim follows from the audit.

## Acceptance criteria

- [ ] Exact upstream crate provenance, licenses, checksum and complete patch manifest; no unrelated dependency changes.
- [ ] Checked preallocation ledger debits the same B active-slot balance, covers actual capacity and temporary overlap, and cannot mint per-connection budgets.
- [ ] Receive/send, decoded strings/vectors, status/errors, metadata/caches, retained shared backing/clones and idle/cancellation/close lifetimes are covered; ownership proves release or charged transfer.
- [ ] Hostile length/count gates precede allocation; complete accepted operation and reconnect maxima remain supported. No arbitrary cap or truncation.
- [ ] TLS behavior remains unchanged and its dependency bounds are proved; exact additional paths are reported for amendment if needed.
- [ ] Independent negative/positive allocation tests, actual configured PostgreSQL17.6 qualification, strict affected validation and complete source/delta self-review.
- [ ] Independent exact-head review, canonical CI, normal Merge Queue and protected readback before separately serialized B activation.

## High-risk authority/recovery qualification

Driver parsing/resource safety applies. New PREPARE/COMMIT authority, session/controller installation and persisted recovery interpretation are NOT_APPLICABLE: no owning Game/B/SQL surface is leased. Existing durable behavior must remain compatible; denial after possible submission cannot classify an ambiguous outcome as uncommitted or release B custody. Negative tests independently exercise malformed peer inputs and resource exhaustion without weakening unrelated validity controls.

## Excluded scope

No B/other Game runtime, SQL/migrations, Foundation, registry, workflow, SQLx-core, sibling vendor crate, TLS implementation, deployment/secret/live-data or external-repository edits. Root Cargo permits only the path patch, necessary workspace exclusion and lock consequences. Preserve TLS and unrelated SQLx behavior; no authentication/transport downgrade, test suppression, arbitrary default or new architecture policy. New concrete scope requires another protected allocation.

## Implementation / findings

NOT_STARTED / NOT_ADMITTED. No product write before Work's protected readback, exact Cargo overlap check and immutable admission grant. Parallel-first: B,338 and346 retain disjoint lanes;351 has one exclusive worktree/branch. Cargo publication/integration is serial because247 and Dependabot259/260/261 overlap. No second driver writer.

## Validation and review

Focused: allocation-denial RED/GREEN and hostile-input/lifetime matrix in the plan. Component: vendored unit tests plus all affected normal workspace tests, fmt, strict Clippy, dependency/provenance checks and governance. Actual PostgreSQL17.6 is mandatory for driver integration; unconfigured/skipped tests are not SQL proof. E2E Server Seam/live source qualification remains outside scope and unclaimed. Self-review and genuinely independent full-change parser/resource review are required before final material freeze. Exact-head canonical CI and normal protected Merge Queue/readback remain pending. Audit result and unresolved findings: NOT_STARTED.

## PR and closeout

One admitted branch/PR; Work retains publication/integration and lease-release control. Preserve immutable admission and all counters across normal merge-up and bounded windows. No force-push/rebase/reset, no-op retrigger or self-approval. Final head belongs in PR/check evidence, not a self-referential metadata commit. Work archives/releases only after terminal protected readback; technical scope insufficiency is not completion.

## Context checkpoint

```yaml
last_progress: exact prospective driver scope and247 Cargo lease prepared
status: waiting
admission_state: NOT_ADMITTED
execution_window_number: 0
execution_windows_completed: 0
worker_rotations: 0
identical_failure_retries: 0
repair_cycles_for_current_gate: 0
owner_action_required: null
blocker: protected_allocation_and_exclusive_Cargo_readback
next_action: Work independently qualifies and protects the exact351 allocation package
```
