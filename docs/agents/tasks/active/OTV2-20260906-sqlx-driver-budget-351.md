# OTV2-20260906-sqlx-driver-budget-351

## Current prospective351 hosted-test lease — Work162

Work162 [comment5560691505](https://github.com/Oteryn/Oteryn-Game/issues/162#issuecomment-5560691505) authorizes this five-document allocation package on `coord/sqlx-351-test-lease`, based on protected main `b61f9d8cc1c0a7289ffdaf1bf4e42b851d2c0f9a`. The new test-target lease is **NOT_ACTIVE** until independent qualification, protected integration/readback and an explicit Work grant. Existing351 implementation admission continues; this is not a new worker admission or budget reset.

Current custody overrides older prospective/NOT_ADMITTED prose below. B329 retains immutable admission `b8ae4c965cc7f686b89b4d5c0ba2bc04af6e07fd`, branch `agent/durable-fresh-admission-child-b-329`/PR335, window6/completed5/rotation1 under [329 comment5560373810](https://github.com/Oteryn/Oteryn-Game/issues/329#issuecomment-5560373810). Native `6a2cccb5f448fc9f3b8ca07e1e4a66dc7aadec29` is the actual restart qualification checkpoint recorded in [329 comment5560643661](https://github.com/Oteryn/Oteryn-Game/issues/329#issuecomment-5560643661), not full B acceptance. Newer canonical B head `834db1d7118d751e31287715d3eaac7780a0c7b9`, tree `b11f20a35e4c205c7e3320469616ccd4aaa96bc5`, is the independently reviewed sealed-completion checkpoint in [329 comment5560718303](https://github.com/Oteryn/Oteryn-Game/issues/329#issuecomment-5560718303); its hosted CI is pending, not covered by the earlier366/0 result. At this checkpoint B reports50 productive minutes used in window6, approximately10 remaining, with waiting paused; this amendment adds no minutes. Driver351 retains immutable admission `53c6bdf06a2282d893035a995c46052c88f935b4`, branch `agent/sqlx-driver-budget-351`/draft PR356 and window2 under [351 comment5560554622](https://github.com/Oteryn/Oteryn-Game/issues/351#issuecomment-5560554622), following native `1363c9b5b238f4922615eda9b502866c305e83bf`. Window1 remains55m14s productive/4m46s unused, completed1/repair1/rotation0. These immutable checkpoints do not replace later canonical branch heads or cumulative findings. Preserve all branch/task history and subsequent windows/repairs through normal merge-up; old zero counters below are historical allocation evidence.

### Exact prospective transfer

After the activation gate, temporarily remove `apps/game-server/tests/durability_postgres.rs` from B329's active write scope and lease it exclusively to the sole351 writer **only** to add this module inclusion:

```rust
#[path = "../../../vendor/sqlx-postgres-0.9.0/tests/oteryn_resource_budget.rs"]
mod oteryn_resource_budget;
```

The included `vendor/sqlx-postgres-0.9.0/tests/oteryn_resource_budget.rs` stays within351's existing vendor subtree. No other change to the shared target is authorized: preserve every existing B test, import, fixture, gate and assertion; no reformatting or test suppression. No workflow, Cargo feature/dependency, production B, Foundation or source scope is added. Driver retains its separately protected two-crate Cargo lease and exclusions.

B keeps every other owned path and its canonical branch/worktree. Work verifies exact overlap before granting the lease and before integration. While active, B must not write the shared target or integrate overlapping target changes;351 may not use the lease for any additional edits. Work serializes ordinary merge-up and reviews the resulting exact delta, retaining prior B material. Return the target to B only after protected351 delivery/integration/readback and Work's explicit release/readmission for this file; no concurrent writer or automatic lease inheritance. Earlier14-path B lists remain historical during the active transfer.

The existing canonical PostgreSQL17.6 target must actually execute the included tests on the pinned root dependency graph. Vendor-only test results and successful compilation do not establish hosted SQL execution. Keep all existing workflows and tests intact. If the service is plaintext, it supplies no TLS-positive evidence:351 must separately qualify actual TLS without security/feature downgrade or treating skipped/unconfigured tests as success. This amendment alone proves no TLS/driver/B acceptance and does not release Server Seam247.

### Driver351 task-record handoff

This is a coordinated ownership correction under Work162 comment5560691505. This task's earlier metadata and all historical evidence remain intact. It grants no reset/replacement of the existing admitted worker. Before activation, existing runtime custody remains unchanged; after activation, the exact shared-target exception above supersedes earlier conflicting path lists only. Preserve the worker branch's newer task evidence when normally merging this coordinator amendment.

Next action: Work qualifies/protects the amendment and verifies exact branch overlap before granting the target lease.

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
  - vendor/sqlx-core-0.9.0/**
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

PROVEN: protected main d9d1b566acb57b537ff901d9765c32a95110c259 contains accepted resource341 and completed registry342. Actual SQLx0.9.0 receive buffering reserves peer-announced lengths before body arrival; row count decoders and retained status/type/table storage require coverage. Preserved evidence: PR350 at0131d990eab956462b389374b2e725427c6de251, resource-blocker section. DERIVED: a private bounded PostgreSQL stream using public SQLx-core socket interfaces is the smallest candidate patch. UNKNOWN: whether the amended two-crate accounting scope suffices for every dependency. The pre-admission audit could not prove a complete untouched-TLS reservation within remaining B capacity; it did not prove impossibility. No impossibility or complete-bound claim follows from the audit.

## Acceptance criteria

- [ ] Exact upstream crate provenance, licenses, checksum and complete patch manifest; no unrelated dependency changes.
- [ ] Checked preallocation ledger debits the same B active-slot balance, covers actual capacity and temporary overlap, and cannot mint per-connection budgets.
- [ ] Receive/send, decoded strings/vectors, status/errors, metadata/caches, retained shared backing/clones and idle/cancellation/close lifetimes are covered; ownership proves release or charged transfer.
- [ ] Hostile length/count gates precede allocation; complete accepted operation and reconnect maxima remain supported. No arbitrary cap or truncation.
- [ ] TLS modes, versions, certificate/hostname verification, features and unrelated behavior remain unchanged; complete TLS capacity/lifetime proof is the first implementation checkpoint before substantial decoder work. Exact additional dependency paths require amendment.
- [ ] Independent negative/positive allocation tests, actual configured PostgreSQL17.6 qualification, strict affected validation and complete source/delta self-review.
- [ ] Independent exact-head review, canonical CI, normal Merge Queue and protected readback before separately serialized B activation.

## High-risk authority/recovery qualification

Driver parsing/resource safety applies. New PREPARE/COMMIT authority, session/controller installation and persisted recovery interpretation are NOT_APPLICABLE: no owning Game/B/SQL surface is leased. Existing durable behavior must remain compatible; denial after possible submission cannot classify an ambiguous outcome as uncommitted or release B custody. Negative tests independently exercise malformed peer inputs and resource exhaustion without weakening unrelated validity controls.

## Excluded scope

No B/other Game runtime, SQL/migrations, Foundation, registry, workflow, rustls, other dependency, deployment/secret/live-data or external-repository edits. SQLx-core changes are restricted to the exact accounting files below; no unrelated TLS changes. Root Cargo permits only the two exact path patches, necessary workspace exclusions and lock consequences. Preserve TLS and unrelated SQLx behavior; no authentication/transport downgrade, test suppression, arbitrary default or new architecture policy. New concrete scope requires another protected allocation.

## Exact core accounting amendment

The exact sqlx-core0.9.0 import preserves upstream bytes/licenses except these permitted accounting paths: `src/net/tls/mod.rs`, `src/net/tls/tls_rustls.rs`, `src/net/mod.rs`, new `src/net/resource_budget.rs`, new `src/net/tls/resource_budget_tests.rs`, and `OTERYN_PROVENANCE.md`. Paths are relative to `vendor/sqlx-core-0.9.0/`; its other imported files remain byte-identical upstream. PostgreSQL `src/connection/tls.rs` permits accounting-only budget plumbing. Preserve TLS modes, protocol versions, certificate/hostname verification, selected features and unrelated behavior. Rustls and every other dependency remain excluded; another concrete dependency need requires a protected amendment.

Core provenance: checksum `05b44e85bf579a8eeb4ceaa77a3a523baf2bf0e9bac7e40f405d537b5d2d5ccb`, VCS `003b698e99e024f3621b8043a2426fde5b741171`, subdirectory `sqlx-core`. Issue351 comment5559999529 governs this pre-admission correction. Original352 head7140 green CI is historical; fresh allocation review/CI is required.

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
