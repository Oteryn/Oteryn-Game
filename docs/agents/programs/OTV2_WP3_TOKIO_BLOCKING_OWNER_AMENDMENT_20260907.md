# WP3 Tokio 1.53.1 blocking-resource-owner amendment

Coordinator: #162. Existing worker: #351 / PR #356. Programme: #364.
Preparation authority: #351 comment `5575132779`, following protected WP3 amendment #401 and exact worker blocker `BLOCKED_RUNTIME_BACKEND_OWNER` at #356 head `4af4fb6af18f4e9f38e0d4425690ceb46a4e096a`.

## Status and activation boundary

This is a prospective dependency-level amendment of the SAME #351/#356 worker.
It does not create a new driver worker, reset branch/history/budgets, choose a
production runtime topology or activate B. It is **NOT_ACTIVE** until independent
exact-head review, canonical repository checks, protected Merge Queue integration,
main readback and explicit Work application after fresh Cargo/path custody checks.

```yaml
amendment_id: OTV2-WP3-TOKIO-BLOCKING-OWNER-20260907
repository: Oteryn/Oteryn-Game
coordinator_issue: 162
worker_issue: 351
worker_pr: 356
worker_task_id: OTV2-20260906-sqlx-driver-budget-351
worker_branch: refs/heads/agent/sqlx-driver-budget-351
observed_worker_head_sha: 4af4fb6af18f4e9f38e0d4425690ceb46a4e096a
allocation_base_main_sha: 494723b0d271e675eb44e63661c8575936ed6144
amendment_state: NOT_ACTIVE
worker_launch: EXISTING_WORKER_ONLY
tokio_version: 1.53.1
tokio_crates_io_checksum: 202cae7e3bd8c284e1a94d64a2a7fe0833222ffc970666a15586686e1481e6ed
tokio_upstream_commit: 75fef53d0a8590c2d1dbb63672aa7b7d1ef51155
existing_resource_contract: DUR-FRESH-RESOURCE-ENVELOPE-V1
existing_queued_work_count: 8
existing_active_work_count: 2
existing_executor_resident_budget_bytes: 12582912
new_resource_maximum: FORBIDDEN
ordinary_tokio_spawn_blocking_semantics: PRESERVE
b_activation: FORBIDDEN
external_repositories: []
```

## Why this dependency-level amendment is necessary

The active SQLx writer first exercised the protected SQLx-level amendment and
stopped correctly. Its fresh RED proves `BlockingJobOwner` is absent; inspection
of every enabled SQLx runtime branch then proved that wrapping a closure or result
outside the runtime cannot own the allocation Tokio performs before returning a
join handle. The worker returned exact `BLOCKED_RUNTIME_BACKEND_OWNER` rather
than inventing an approximation.

Pinned Tokio source confirms the material private sequence:

- `tokio/src/runtime/blocking/pool.rs::spawn_blocking_inner` constructs the
  blocking future and calls `task::unowned(...)`, which allocates the generic
  task backing before the public caller receives a handle;
- `Spawner::spawn_task` subsequently pushes `Task` into private
  `VecDeque<Task>` backing;
- when no idle worker exists, that same path may call `spawn_thread` and insert a
  `JoinHandle` into private `HashMap<usize, JoinHandle<()>>` backing;
- the native worker thread and its stack may remain alive through the blocking
  pool keep-alive interval after the originating closure returns;
- public `tokio::task::spawn_blocking` / runtime `Handle` accepts no owner or
  resource-custody capability.

Therefore a SQLx-only wrapper can at most charge closure/result bytes. It cannot
prove preallocation or retained custody of task Cell, queue growth or worker/
thread backing.

## Existing budget authority — no new capacity

This amendment introduces **no new numeric resource maximum**.
`DUR-FRESH-RESOURCE-ENVELOPE-V1` already establishes one B executor per process
and explicitly requires driver/runtime allocations retained on behalf of B work
to be charged or finitely reserved inside the accepted envelope:

- `DFR-QUEUED-WORK`: at most 8 not-yet-submitted operations;
- `DFR-ACTIVE-WORK`: at most 2 end-to-end active slots;
- 4,194,304 queued resident bytes + 8,388,608 active resident bytes =
  12,582,912 charged resident bytes per single logical B executor.

The existing SQLx `ResourceBudget`/`ResourceReservation` remains the owner ledger
and deliberately contains no driver-local default/fallback maximum. The Tokio
path receives a capability backed by that existing executor owner; Tokio must not
create another allowance.

A worker-stack byte reservation is an implementation allocation inside that
existing owner balance, not a new accepted hard maximum. Missing/zero/unfunded
stack configuration fails closed. Production acceptance requires measured
positive evidence for the configured stack allocation; this amendment does not
select a production stack size.

## Exact vendoring authority

The worker may vendor the complete Tokio 1.53.1 crate from the exact crates.io
package checksum above. Untouched source/package/license bytes must match the
pinned package/upstream. Add complete provenance and a machine-checkable delta
manifest. No source may be copied from an unpinned moving branch.

Additional paths authorized only after protected application:

```text
vendor/tokio-1.53.1/**                        # complete exact upstream package
```

Within that package, Oteryn-authored semantic modifications are restricted to the
smallest blocking/task owner path and package feature/provenance/test surfaces:

```text
vendor/tokio-1.53.1/Cargo.toml
vendor/tokio-1.53.1/Cargo.toml.orig
vendor/tokio-1.53.1/src/task/blocking.rs
vendor/tokio-1.53.1/src/runtime/blocking/mod.rs
vendor/tokio-1.53.1/src/runtime/blocking/pool.rs
vendor/tokio-1.53.1/src/runtime/blocking/task.rs
vendor/tokio-1.53.1/src/runtime/task/mod.rs
vendor/tokio-1.53.1/src/runtime/task/raw.rs
vendor/tokio-1.53.1/src/runtime/task/core.rs
vendor/tokio-1.53.1/src/runtime/task/harness.rs      # only if final task-release hook needs it
vendor/tokio-1.53.1/tests/**                         # focused owner-path tests only
vendor/tokio-1.53.1/OTERYN_PROVENANCE.md
vendor/tokio-1.53.1/OTERYN_DELTA_MANIFEST.json
```

If exact implementation proves one additional Tokio source file is materially
required, stop before modifying it and return `SHARED_LEASE_REQUIRED` with exact
symbol/path and reason; this document is not `vendor/tokio/**` carte blanche.

The existing #351 serialized root Cargo lease is reaffirmed for only the patch
consequences required to select the vendored Tokio package:

```text
Cargo.toml
Cargo.lock
```

The root patch must keep the exact Tokio version/source identity and must not
upgrade/downgrade Tokio or change unrelated dependencies/features. Any workspace
exclude needed solely because the vendored crate is nested under the repository
is part of that existing root Cargo lease. Supply-chain/license/provenance checks
remain mandatory.

## Required Oteryn-specific Tokio API contract

Ordinary upstream `spawn_blocking` remains byte/semantic compatible for all
unowned callers. The patch adds one non-default Oteryn resource-owner path only.
It must be impossible for denial in the owned path to silently fall back to the
ordinary unowned path.

A valid design may use an Oteryn-only non-default Cargo feature and a deliberately
narrow public/doc-hidden owner API so SQLx can supply a capability without Tokio
depending on SQLx types. The capability must be generic over an owner/accounting
interface; SQLx adapts the existing `ResourceBudget` into it. Avoid a dependency
cycle and avoid exposing Foundation/B semantic authority through Tokio.

### Before task allocation

Owned spawn must reserve all inline/external task backing before
`task::unowned(...)` / `Cell` allocation. Tokio knows the concrete generic task
layout and must use checked `Layout`/capacity arithmetic. Reserve before Box/Arc
or equivalent allocation; overflow/exhaustion returns bounded denial without
creating the task.

The owner/accounting handle stored in task backing is itself part of the charged
layout. A reference-count clone that does not allocate a new backing may share the
existing owner identity; any new Box/Arc backing requires its own charge.

### Resource-owned queue

Do not charge unpredictable growth of the ordinary global `VecDeque` after the
allocation happened. Resource-owned jobs must instead enter a separately bounded
owner queue or equivalent preallocated owner-controlled structure whose capacity
is supplied from the accepted DFR queued-work count. Queue backing is reserved
before allocation/registration and remains charged to the executor owner while
that backing exists.

The ordinary Tokio blocking queue remains unchanged for non-owned callers. Owned
queue capacity denial occurs before task admission and cannot spill into the
ordinary queue.

The accepted count is supplied/configured by the owning executor from the
protected registry/DFR contract; Tokio itself does not invent or widen it. Any
attempt to configure more than the owner-provided finite capacity is denied.

### Worker/thread custody

If an owned queued job causes a new blocking worker to be created, the owner path
must reserve the attributable worker bookkeeping, native thread handle and an
explicit configured stack allocation before OS thread creation. The owner permit
must stay attached to that worker until actual thread exit; idle/keep-alive time
remains charged. `JoinHandle` insertion/map backing caused by the owned worker is
charged before allocation/growth or stored in an owner-preallocated structure.

A missing/unfunded worker-stack configuration is a closed error for the owned
path. Do not assume the operating system's default stack is a finite owned
allocation. Thread creation failure releases only reservations whose backing was
never created.

If an already existing blocking worker services the owned job, no duplicate
worker-stack charge is created for that job. Task and owner-queue custody still
apply. If an owned job caused the worker's creation, that worker remains charged
to its original executor owner until exit even if later unowned work reuses it.

The implementation may retain the current keep-alive duration; it must charge the
worker throughout it. Changing the keep-alive, global thread cap or scheduler
policy is outside this amendment.

### Cancellation, result and shutdown

Task Cell/result backing remains charged until final task destruction, not merely
closure return. Queue backing remains charged until queue storage is actually
released. Worker charge remains until thread exit. Aborting before run may release
task-specific custody only after Tokio has actually removed/destroyed the task.
A started blocking task remains charged despite abort/dropped join handle. Runtime
shutdown timeout is not proof that backing disappeared.

No callback that fires before final backing release may release its reservation.
If exact final drop ordering requires the listed task harness file, the worker may
use it only for the owner permit lifecycle and must prove destruction-before-
release.

## Required SQLx integration after the Tokio patch

The existing #351 SQLx owner surface may then implement the currently RED
`BlockingJobOwner` using its protected `ResourceBudget` and invoke only the new
resource-owned Tokio path for certificate/configuration blocking work.

Required behavior:

1. owner/queue/active/stack configuration is supplied before spawn;
2. denial propagates as bounded resource-unavailable before blocking backend
   task/queue/thread allocation;
3. no fallback to generic `spawn_blocking` after owner denial;
4. returned certificate backing and blocking task/result charges transfer/release
   exactly once under existing SQLx owner semantics;
5. every enabled SQLx runtime backend must remain fail closed. This amendment
   authorizes only the Tokio implementation path; non-Tokio enabled backends must
   either have their own already-proven owner implementation or return exact
   `BLOCKED_RUNTIME_BACKEND_OWNER`. Do not weaken features merely to avoid proof.

## TDD / qualification matrix

The same #351/#356 worker must preserve the existing RED head and add a distinct
post-amendment GREEN generation.

### Tokio unit/loom-style controls where supported

- owner denied before task Cell allocation;
- owned queue full denied before task admission;
- valid task allocation holds exact task charge through final destroy;
- queued abort releases only after actual removal/drop;
- started blocking abort/dropped handle retains task/result custody;
- owned worker creation reserves before spawn and releases at thread exit;
- idle keep-alive retains worker charge;
- OS thread-spawn failure does not leak reservation;
- queue/task/worker accounting overflow is fail closed;
- ordinary unowned `spawn_blocking` behavior/regressions unchanged.

Use exact allocation counters/owner witnesses, not timing-only inference.

### SQLx / repository controls

- prior RED still fails without the vendored owner hook;
- GREEN funded TLS certificate load through actual Tokio owner path;
- insufficient executor budget denied before backend allocation;
- cancellation/detach/shutdown ownership controls;
- hostile certificate/path/size matrix remains;
- actual configured TLS-positive + PostgreSQL 17.6 integration;
- root dependency graph uses only exact vendored Tokio 1.53.1 for the selected
  patched dependency and no second incompatible Tokio source for the target;
- strict Rust 1.94 fmt/Clippy/test, supply chain/provenance/delta validation;
- complete diff independent high-risk review and canonical CI/Merge Queue.

A fake zero-allocation runtime or closure-only reservation is not positive proof.

## Excluded scope

No Tokio version change, generic scheduler/runtime semantics, IO driver, async
worker scheduler, timers, sync primitives, production Game runtime topology,
FND03 numeric worker policy, resource-registry row, B/Foundation/SQL migration,
Server Seam, workflow/protection, production/config/secrets/live-data or external
repository mutation.

This amendment does not claim the existing default Tokio pool globally bounded.
It only permits one separate resource-owned blocking admission path used by #351.
If proving the required owner path needs a global scheduler/allocator/runtime
architecture change beyond the exact listed source, stop and return one precise
new architecture blocker; do not broaden scope.

## Sequencing and terminal boundary

After protected allocation/readback, Work must freshly verify the #351 branch,
root Cargo lease, B shared PostgreSQL target exclusion and exact Tokio source
provenance before applying it. Continue on the SAME #356 branch/history and
preserve all prior RED/blocker evidence.

`protected Tokio amendment -> worker vendor/TDD GREEN -> TLS/PG driver completion -> independent review -> protected WP3 integration/readback -> release shared PG target -> only then WP4 application`.

This document may be protected while WP2 runs because it is disjoint prospective
allocation authority. It cannot make WP3 complete or activate WP4 by itself.

The allocation document itself requires one independent exact-head deep review,
canonical repository checks, normal protected Merge Queue and main readback.
Runtime E2E is NOT_APPLICABLE to this docs-only allocation; material vendor/driver
E2E remains required on #356.
