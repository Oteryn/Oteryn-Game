# WP3 SQLx blocking-runtime owner allocation amendment

Coordinator: #162. Existing worker: #351 / PR #356. Programme: #364.
Preparation authority: #351 comment `5574706577`, following independent Lane G disposition `5567988757`.

## Status and activation boundary

This is a prospective amendment of the existing admitted SQLx driver task, not a
new worker, budget reset, resource maximum, architecture programme or runtime
activation. It is **NOT_ACTIVE** until independent exact-head review, canonical
repository checks, normal protected integration/readback and explicit Work
application after fresh custody verification.

```yaml
amendment_id: OTV2-WP3-BLOCKING-RUNTIME-OWNER-20260907
repository: Oteryn/Oteryn-Game
coordinator_issue: 162
worker_issue: 351
worker_pr: 356
worker_task_id: OTV2-20260906-sqlx-driver-budget-351
worker_branch: refs/heads/agent/sqlx-driver-budget-351
original_admission_main_sha: 53c6bdf06a2282d893035a995c46052c88f935b4
observed_worker_head_sha: f8c2530284e9fad20d97cae5140d4c9c5c1933bd
allocation_base_main_sha: 0f72ffeb6218981bbef29ce783a8c3efa7e10964
amendment_state: NOT_ACTIVE
worker_launch: EXISTING_WORKER_ONLY
numeric_limit_change: FORBIDDEN
runtime_backend_fork: FORBIDDEN_WITHOUT_SEPARATE_AMENDMENT
postgres_decoder_expansion: HELD
b_activation: FORBIDDEN
additional_permitted_paths:
  - vendor/sqlx-core-0.9.0/src/rt/mod.rs
  - vendor/sqlx-core-0.9.0/src/rt/resource_owner.rs
  - vendor/sqlx-core-0.9.0/src/rt/resource_owner_tests.rs
existing_accounting_paths_reaffirmed:
  - vendor/sqlx-core-0.9.0/src/net/resource_budget.rs
  - vendor/sqlx-core-0.9.0/src/net/tls/mod.rs
  - vendor/sqlx-core-0.9.0/src/net/tls/tls_rustls.rs
  - vendor/sqlx-core-0.9.0/src/net/tls/resource_budget_tests.rs
  - vendor/sqlx-core-0.9.0/src/net/mod.rs
  - vendor/sqlx-postgres-0.9.0/src/connection/tls.rs
  - docs/agents/tasks/active/OTV2-20260906-sqlx-driver-budget-351.md
  - docs/superpowers/plans/2026-09-06-sqlx-driver-budget.md
shared_target_lease_preserved:
  - apps/game-server/tests/durability_postgres.rs
excluded_dependency_paths:
  - tokio/**
  - async-std/**
  - smol/**
  - async-global-executor/**
  - rustls/**
external_repositories: []
```

The existing broad vendor/Cargo admission and all prior evidence remain
immutable provenance. This amendment exists because the later exact accounting
restriction did not include the runtime task/scheduler boundary now proven
material. It adds no path outside the existing SQLx driver family except the
already protected include-only PostgreSQL test-target lease.

## Verified blocker being repaired

Current #356 source proves two complementary facts:

1. `vendor/sqlx-core-0.9.0/src/net/tls/mod.rs` already contains an accounted
   synchronous `read_certificate_file_accounted` primitive and explicitly states
   that it is awaiting a qualified execution owner. It warns that a future
   blocking-job adapter must separately charge closure, scheduler and result
   custody.
2. `vendor/sqlx-core-0.9.0/src/rt/mod.rs::spawn_blocking` directly dispatches to
   Tokio, async-global-executor, smol or async-std and receives no resource-owner
   capability. Lane G independently verified that the active Tokio path creates
   task backing before returning a handle and may enqueue/spawn/retain backing
   that the existing SQLx allocation does not own.

A result/closure reservation created *inside* the blocking closure is therefore
too late. A job is not qualified merely because the certificate bytes themselves
are charged.

## Required implementation contract

The amended worker may introduce a sealed SQLx-core blocking-job ownership
surface only to support the already allocated TLS/configuration loading proof.
The surface MUST be fail-closed and MUST NOT become a second resource budget.

Before the implementation invokes any runtime backend's generic blocking spawn,
it must have a capability whose proof covers all allocations/lifetimes causally
owned by that job, including at minimum:

- closure/environment retained by the runtime;
- result and enclosing handoff representation until consumed/dropped;
- runtime task/Cell or equivalent backend task backing;
- queue/map/list slot or capacity growth attributable to admission of the job;
- worker/thread retained backing attributable to the job or a separately
  registered owner that is already charged;
- overlap while task state moves between queued, running, completed, cancelled,
  detached and dropped states;
- any idle retention that remains after the closure body returns;
- shutdown/cancellation paths, including a dropped join handle.

Reservation must occur **before** the allocation/admission it covers. Releasing
a reservation is lawful only after the corresponding backing is actually gone
or custody has been transferred to another already charged owner. A timeout,
future cancellation or dropped handle is not evidence of release.

The existing `ResourceBudget` supplied by the owning operation remains the
preferred ledger. If a backing is genuinely process/runtime-shared rather than
operation-owned, this amendment permits only a typed *reference to* a legitimate
registered shared owner whose storage/capacity was already charged elsewhere.
It does not create that owner, capacity or numeric allowance. Absence of such an
owner is a blocker, not permission to invent one.

## Fail-closed backend rule

The worker must prove the contract for **every runtime backend actually enabled
by the pinned root build/features used by Oteryn**. It must not silently assume
Tokio merely because production currently prefers Tokio while compiled fallback
branches remain reachable under supported feature combinations.

If the SQLx-level code cannot prove preallocation and retained custody of the
backend task/queue/worker boundary without modifying a dependency outside this
amendment, the worker MUST stop before making that unallocated change and return:

`BLOCKED_RUNTIME_BACKEND_OWNER`

with:

- exact backend and pinned dependency/version/commit;
- exact missing allocation/retention mechanism;
- exact dependency paths required;
- why existing hooks/callbacks cannot deny before allocation or retain custody;
- smallest proposed next amendment;
- a positive source/test witness showing the requested hook would make the proof
  feasible, when such a witness can be produced without the unallocated write.

No Tokio fork, runtime monkey patch, new registry row or Game runtime owner is
implicitly authorized by this document.

## Required tests before PostgreSQL decoder work resumes

The blocking-owner checkpoint precedes substantial PostgreSQL decoder expansion.
The existing #351 writer must prove, on its own canonical branch:

1. **RED** — current generic blocking path cannot satisfy the owner contract;
   test must fail for the missing pre-spawn owner, not for unrelated compilation.
2. **Positive admission** — a bounded test owner with sufficient existing ledger
   capacity permits one configuration-loading job and transfers the returned
   charged certificate backing without double release.
3. **Preallocation denial** — insufficient owner capacity fails before the
   blocking backend receives/allocates the job.
4. **Cancellation before run**, **cancellation while queued/running** and
   **dropped/detached result** preserve charge until actual backend/job release.
5. **Queue/worker retained backing** is either operation-charged or demonstrably
   held by an already registered shared owner; no uncharged third state.
6. **Shutdown/idle** proof establishes when retained backing is released.
7. Existing TLS modes, certificate/hostname verification, protocol versions and
   cache behavior are unchanged for valid funded execution.
8. The existing hostile TLS/certificate capacity/lifetime matrix remains and
   gains an actual positive funded loader case; no arbitrary certificate/count
   truncation is introduced.

A test that replaces the runtime with a zero-allocation fake is useful as a unit
control but is insufficient as the backend ownership proof.

## Scope and sequencing

This amendment adds **zero B/Foundation/SQL migration/Server Seam paths** and no
new Cargo dependency. The current root Cargo/lock lease of #351 remains exactly
as already protected. The include-only `durability_postgres.rs` lease remains
serialized away from B until Work explicitly returns it.

Sequence remains:

`WP3 blocking-owner proof -> complete TLS capacity/lifetime proof -> PostgreSQL driver accounting -> actual TLS-positive + PostgreSQL 17.6 qualification -> independent exact-head review -> protected driver integration/readback -> separately serialized B/WP4 continuation`.

WP2 may progress on its disjoint Foundation paths only when its own Work gates
permit. This amendment does not make WP4, Server Seam, G0 or G1 ready.

## Review and integration requirements

The allocation document itself requires one genuinely independent exact-head
review because it expands a high-risk parser/resource implementation boundary.
After protected integration, Work must verify the exact worker branch/task head,
existing sole-writer identity, current root Cargo overlap and shared test-target
lease before setting this amendment ACTIVE.

The subsequent material #356 checkpoint requires its own independent full-diff
review, exact-head CI, actual configured PostgreSQL/TLS evidence where applicable,
normal Merge Queue and protected readback. Green allocation CI is never runtime
qualification.

No direct merge, force push/rebase/reset, no-op retrigger, production mutation,
credential use, remote-desktop bypass or external-repository write is authorized.
