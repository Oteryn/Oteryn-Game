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

Historical allocation state: NOT_STARTED / NOT_ADMITTED before protection. Admitted by Work comment5560220858 at protected main `53c6bdf06a2282d893035a995c46052c88f935b4`, tree `ae5ef391163f79158dd723940edf819a275985c2`, branch `agent/sqlx-driver-budget-351`. Window1 began 2026-09-06T15:25:35Z. First checkpoint remains OPEN: exact two-crate import and same-owner reservation primitive are implemented; TLS accounting is not activated and PostgreSQL decoders remain upstream. Parallel-first: B,338 and346 retain disjoint lanes;351 has one exclusive worktree/branch. Cargo publication/integration is serial because247 and Dependabot259/260/261 overlap. No second driver writer.

## Validation and review

Focused: allocation-denial RED/GREEN and hostile-input/lifetime matrix in the plan. Component: vendored unit tests plus all affected normal workspace tests, fmt, strict Clippy, dependency/provenance checks and governance. Actual PostgreSQL17.6 is mandatory for driver integration; unconfigured/skipped tests are not SQL proof. E2E Server Seam/live source qualification remains outside scope and unclaimed. Self-review and genuinely independent full-change parser/resource review are required before final material freeze. Exact-head canonical CI and normal protected Merge Queue/readback remain pending. Checkpoint validation: owner-ledger RED/GREEN; vendored core library 11/11 tests (9 new), strict vendor Clippy; pinned Game server check, strict Clippy and 340/340 library tests pass. Vendor unit tests use its unchanged upstream lock (rustls0.23.40); Game check/tests use pinned rustls0.23.43. Initial isolated vendor run lacked cached dev dependencies; an initial compiler-shim invocation attempted the dependency toolchain and failed, then explicit Rust1.94 compiler paths resolved it. No tests or dependency declarations were suppressed. Governance passes. Full import whitespace inspection finds20 trailing-whitespace lines already byte-identical in upstream PostgreSQL; they are preserved and disclosed, not normalized or suppressed. Authored delta whitespace passes. These results qualify the primitive and import, not TLS or PostgreSQL17.6 integration. Complete TLS proof remains UNKNOWN: nested decoder capacities, private configuration/cache layouts and phased pre-call ownership bounds require closure. See vendored provenance for exact source terms and patch manifest.

## Window1 durable checkpoint

Work published draft PR356 at native `703da76a870356f2ab94d6e7af11675dd24f6c04`, tree `07591d3c8668dec06c4190eb4784be04b3351469`. Independent review found no primitive P0/P1; P2 complete-license evidence was repaired additively in both provenance files with VCS-pinned notices/digests, preserving imported bytes/modes. One repair cycle is recorded. Full TLS gate remains OPEN. Independent source review supports a behavior-equivalent private write-discard session store, but no adapter is implemented. Correlated ECH upper3276720 plus span327680/request524288/deframer65536 leaves80 bytes before other required owners; current conservative bounds therefore cannot certify accepted maxima. This is an unresolved proof, not an impossibility claim or permission to change limits. Complete source findings and conditional hook paths are recorded in core provenance.

## PR and closeout

One admitted branch/PR; Work retains publication/integration and lease-release control. Preserve immutable admission and all counters across normal merge-up and bounded windows. No force-push/rebase/reset, no-op retrigger or self-approval. Final head belongs in PR/check evidence, not a self-referential metadata commit. Work archives/releases only after terminal protected readback; technical scope insufficiency is not completion.

Window1 stopped at 2026-09-06T16:20:49Z: 3314 productive seconds conservatively charged, 286 unused seconds, no deducted pauses or reset. A further implementation window requires explicit Work continuation.

## Context checkpoint

```yaml
last_progress: published tested primitive and independently reviewed source bounds; TLS proof remains open
status: in_progress
admission_state: ADMITTED
execution_window_number: 1
execution_windows_completed: 1
worker_rotations: 0
identical_failure_retries: 0
repair_cycles_for_current_gate: 1
owner_action_required: null
blocker: complete_TLS_capacity_lifetime_proof
next_action: independently review TLS proof checkpoint before substantial PostgreSQL decoder work
```
