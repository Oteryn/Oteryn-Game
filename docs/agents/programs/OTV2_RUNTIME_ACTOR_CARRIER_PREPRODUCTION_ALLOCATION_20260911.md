# OTV2 Runtime Actor Carrier Pre-Production Allocation — 2026-09-11

```yaml
allocation_id: OTV2-ALLOC-RUNTIME-ACTOR-CARRIER-PREPROD-530-20260911
repository: Oteryn/Oteryn-Game
coordinator_issue: 162
governing_issue: 530
architecture_gate: 540
architecture_decision_pr: 570
protected_admission_main: c6103b325748fd31268c7b332defcddf76c59a66
active_control_plane_profile: OTV2_WORK_DELIVERY_COORDINATOR
worker_profile: OTV2_IMPL_FOUNDATION_RUNTIME
worker_alias: "Oteryn: impl foundation"
future_task_id: OTV2-20260911-runtime-actor-carrier-preproduction-530
future_branch: agent/runtime-actor-carrier-preproduction-530
allocation_state: NOT_ACTIVE
worker_state: NOT_STARTED
implementation_authority: PRE_PRODUCTION_FOUNDATION_RUNTIME_ONLY
production_authority: NONE
resource_registry_mutation_authority: NONE
ability_508_authority: NONE
movement_139_authority: NONE
external_repository_write_authority: NONE
```

## Purpose

Record the smallest prospective implementation authority allowed after protected integration of
`RUNTIME-ACTOR-CAPACITY-DEFERRED-UNTIL-MEASURABLE-V1` from PR #570.

This allocation does **not** start a worker. Only after this allocation is protected-integrated,
read back from `main`, and #162 performs a fresh overlap/custody/concurrency preflight may the exact
future branch above receive mutation authority.

The intended result is one bounded **pre-production** Channel actor carrier plus direct exact lookup
inside the existing Foundation runtime boundary. It exists only to make representative capacity
measurement technically possible without pretending that a production actor maximum or VPS class is
already known.

## Proven admission facts

- Protected `main` is `c6103b325748fd31268c7b332defcddf76c59a66`, the protected Merge Queue
  integration of PR #570.
- PR #570 established that Synology is NON_PRODUCTION development/integration/measurement only and
  that production VPS class/capacity remains evidence-gated.
- `RUNTIME-ACTOR-RL-01` remains exactly `PERF_REFERENCE_CELL_REQUIRED`; this allocation must not
  serialize a numeric production maximum into `RESOURCE_LIMITS_REGISTRY.json` or another authority.
- The prior non-production prototype PR #568 is merged as
  `90a3f92434e32354ff1aaeac96d038bbc49eba9c`; its final source head is
  `3f9eea232e7bd40d2f6a3185b06e7e1d84985328` and its native Merge Queue run `34606613550`
  completed successfully. Its old four-path prototype custody is terminal. This coordinator change
  records the canonical archive/release; protected integration and readback make that closeout
  canonical on `main`.
- Current Foundation contains `RuntimeScopeRefV1`, `ScopeOwnershipGeneration` and
  `ScopeRuntimeFence`, but the protected WP5 scope-assignment allocation explicitly classifies
  `ScopeRuntimeFence` as a **consumer of a supplied grant, not producer authority**. This allocation
  therefore must not treat a fence object or copied generation as proof that a Channel ownership
  grant was authoritatively issued.
- The durable Channel scope-assignment producer and its later Foundation consumer integration remain
  separate dependencies. This allocation neither implements nor activates them.
- FND-03 requires at most one current logical authoritative owner per `WorldId + ChannelId`, stale
  ownership generations to fail closed, and runtime-local slot reuse to carry a local generation.
- The protected #568 prototype proved the accepted direct slot/generation shape and fail-closed
  same-generation reconstruction behavior without a separately growing lookup index or
  retirement-history store. It is evidence, not production/runtime ownership authority.

## Fresh ownership / overlap census

The allocation was prepared against live GitHub state after PR #570 protected readback.

### Active material writers preserved

- PR #356 / `agent/sqlx-driver-budget-351` is an open draft Durability/SQLx/vendor/Cargo worker. Its
  changed paths do not include `apps/game-server/src/foundation/mod.rs` or the new carrier path below.
- PR #335 / `agent/durable-fresh-admission-child-b-329` is an open Durability worker whose 14 changed
  paths are migration/Durability/test/task/plan paths. It does not own the new carrier path or
  `apps/game-server/src/foundation/mod.rs`.
- Issue #247 preserves `agent/otv2-gameplay-server-seam-01` in `WAITING_DEPENDENCY`. Its current delta
  includes `apps/game-server/src/lib.rs`, `foundation/protocol.rs`, `gameplay_transport/**` and Cargo
  surfaces. This allocation therefore **excludes** all of those paths and does not borrow the Server
  Seam shared lease.
- The historical `agent/perf-actor-reference-cell-prep-543` branch is not runtime implementation
  authority; Issue #543 is closed `not_planned` and explicitly excludes actor-carrier/runtime work.
- No live branch named for `runtime-actor` and no open runtime actor-carrier implementation PR was
  found during this census.

### Shared-surface rule

`apps/game-server/src/foundation/mod.rs` is the only existing source file this future worker may edit,
and only for the minimal private/crate-visible module declaration/export required by the new file.
It is treated as serialized for this allocation. Fresh #162 activation must prove it is still free.

`apps/game-server/src/lib.rs` remains excluded because the preserved Server Seam branch already has a
material one-line delta there.

## Future worker owned paths — only after explicit activation

```text
apps/game-server/src/foundation/runtime_actor_carrier.rs
apps/game-server/src/foundation/mod.rs
docs/agents/evidence/OTV2-20260911-runtime-actor-carrier-preproduction.md
docs/agents/tasks/active/OTV2-20260911-runtime-actor-carrier-preproduction-530.md
```

After activation, this allocation document and all other `docs/agents/programs/**` remain read-only to
the worker.

No other path is implicitly writable.

## Required implementation shape

### 1. One shared Channel carrier

Implement exactly one Foundation-owned carrier shape for one semantic Channel scope. Do not create a
parallel Ability, AI, Movement, transport, session or persistence registry.

The carrier must be bound to an exact `RuntimeScopeRefV1::Channel` and supplied ownership generation.
It must preserve the accepted local actor identity generation semantics from the #539/#541 chain and
from the protected #568 prototype.

### 2. Ownership authority is not minted or inferred here

This allocation does not select or implement the durable scope-assignment producer. A
`ScopeRuntimeFence`, a `ScopeOwnershipGeneration`, or an actor reference is not by itself proof that
the caller is the current Channel owner.

The carrier must preserve fail-closed namespace continuity without manufacturing a second ownership
authority:

- no public/runtime constructor may establish current Channel ownership from raw
  `WorldId + ChannelId + ScopeOwnershipGeneration` facts alone;
- the private `ScopeRuntimeFence::from_external_grant` visibility must not be widened or used by the
  carrier as a substitute producer; do not make `ScopeRuntimeFence` Clone/Copy;
- before the separately accepted durable producer/consumer integration exists, any bootstrap capable
  of creating a carrier namespace from supplied scope/generation facts must be test-only or an
  explicitly non-production crate-private development fixture path;
- that pre-production bootstrap must consume a move-only/non-replayable continuity grant supplied
  from outside the carrier and must not expose an issuer or reconstruct that grant from raw facts;
- carrier loss must not permit same-generation namespace reconstruction that can revive a stale actor
  reference; same-generation rebootstrap fails closed;
- a fresh namespace may be established only after the caller supplies an independently authorized
  strictly newer ownership generation through the applicable pre-production fixture boundary;
- none of those fixture/continuity mechanics becomes production grant, assignment or readiness
  authority.

If progress requires the actual durable current-owner producer/consumer rather than the bounded
pre-production fixture boundary, classify the lane `WAITING_DEPENDENCY` on that protected work. If it
requires new or conflicting ownership semantics, use `ARCHITECTURE_ESCALATION_REQUIRED`. Do not invent
a producer inside this carrier allocation.

### 3. Explicit finite non-production bound, no numeric product decision

Every carrier construction must require an explicit finite **non-production** capacity input. There is
no production default, no environment-derived fallback and no registry-backed production value in
this allocation.

Required behavior:

- zero/missing capacity rejects before allocation;
- all slot-count/byte/index arithmetic is checked before allocation;
- backing allocation is fallible and allocation failure is a typed fail-closed result;
- backing does not grow beyond the admitted per-instance bound;
- exact M succeeds only when resources are available; M+1 rejects without mutating retained state;
- any test/sample M is labeled fixture/evidence-only and cannot be copied into a production limit;
- the development bound cannot be read, exported, serialized or reported as an accepted production
  capacity, deployment default or production readiness fact;
- any path or request labeled production must fail closed / remain unsupported because this allocation
  has no accepted production-capacity source or production constructor;
- no constructor or helper may silently substitute `AI01-ACTIVE-ACTORS=256`, prototype M=1..4, or
  any other existing numeric limit as the Channel production maximum.

This per-run bound exists to make later representative measurements possible. It is not acceptance of
that bound for production.

### 4. Direct exact lookup only

The first actor reference/lookup must be exact and scoped to the supplied Channel namespace and
ownership generation. The reference itself is not proof that its generation is currently authoritative:

```text
WorldId
+ ChannelId
+ ScopeOwnershipGeneration
+ actor-local identity
+ actor-local generation
```

Lookup/removal must be direct to the actor slot/generation cell; there must be no independently growing
secondary index or unbounded retirement/tombstone history.

Reject independently and without mutation:

- wrong World;
- wrong Channel;
- stale scope ownership generation under the supplied pre-production continuity boundary;
- missing/out-of-range actor-local identity;
- valid but vacant slot;
- stale actor-local generation after reuse;
- actor-local generation exhaustion/wrap risk.

### 5. No geometry or gameplay semantics

Exact lookup does **not** authorize range, line-of-sight, floor, pathfinding, dynamic retargeting,
movement legality, combat legality, Ability target selection or AI behavior.

PR #508 Phase A and Issue #139 Movement remain separate and unactivated.

### 6. Bounded lifecycle and rollback

Admission/removal/reuse must preserve:

- immutable local-ID-to-slot binding;
- generation retention across vacancy;
- generation advance on successful reuse;
- no local-generation wrap/reuse;
- complete state rollback for any failure after a candidate slot is selected;
- bounded deterministic insertion/lookup/removal work with no hidden unbounded scan/history growth.

## Acceptance evidence for the implementation worker

The future task must prove on one unchanged exact head:

- focused RED -> minimal GREEN for the carrier module;
- non-production capacity is explicit, finite, mandatory and fail-closed;
- checked arithmetic and allocation-failure paths execute before partial authority/state publication;
- M/M+1 behavior is exercised across multiple injected fixture bounds without promoting any fixture
  value to product policy;
- attempts to read/export the development bound as production capacity or invoke a production-labeled
  construction path fail closed / remain structurally unavailable;
- same-scope/current-generation exact lookup succeeds under the supplied pre-production continuity
  boundary and every stale/cross-scope case above rejects;
- carrier loss cannot reconstruct a same-generation namespace from raw scope/generation facts;
- the pre-production continuity grant is move-only/non-replayable and exposes no issuer/reissuer path;
- removal/reuse and actor-local generation exhaustion are deterministic and preserve unrelated state;
- `ScopeRuntimeFence` is not made Clone/Copy, its private raw-grant constructor is not widened, and the
  carrier does not promote it to durable assignment-producer authority;
- `apps/game-server/src/foundation/mod.rs` changes only minimal module wiring for this allocation;
- no Cargo/workspace/registry/workflow/protocol-ID/public-wire/production configuration changes;
- `cargo +1.94.0 fmt --all --check` succeeds;
- focused `oteryn-game-server` unit tests for the module succeed;
- strict Clippy for the affected server target succeeds;
- required exact-head Merge Gate / `game-gate` succeeds;
- mandatory adversarial whole-diff self-review reports zero unresolved material findings;
- genuinely independent exact-head review is clean because this changes multichannel runtime/fencing
  semantics;
- unresolved review threads and requested changes are zero.

Physical production-capacity E2E is **NOT_APPLICABLE** to this worker: it is deliberately not a
production capacity qualification. Later PERF evidence must run against representative composed
artifacts/environment and remains required before any production actor maximum or VPS class is
accepted.

## Explicitly excluded

The future worker has no authority to edit or claim:

```text
apps/game-server/src/lib.rs
apps/game-server/src/main.rs
apps/game-server/src/foundation/protocol.rs
apps/game-server/src/foundation/admission.rs
apps/game-server/src/foundation/admission_recovery_inner.rs
apps/game-server/src/durability/**
apps/game-server/src/gameplay_transport/**
apps/game-server/Cargo.toml
Cargo.toml
Cargo.lock
workspace-boundaries.toml
.github/**
docs/architecture/**
docs/contracts/**
docs/architecture/RESOURCE_LIMITS_REGISTRY.json
```

Also excluded:

- production configuration/deployment/ports/secrets/hardware provisioning;
- VPS provider/SKU/CPU/RAM/storage/network/topology selection;
- production Channel actor maximum or admission capacity;
- durable scope-assignment producer/consumer composition;
- #508 activation or Ability mutation;
- #139 activation or Movement mutation;
- gameplay-domain state/value/formulas;
- persistence/schema/migration changes;
- transport/TLS/listener changes;
- external-repository writes.

## Activation rule

Protected integration of this document is necessary but not sufficient to release mutation.

After protected-main readback, #162 must fresh-read:

1. protected `main` and this exact allocation;
2. open PRs, branches and active task records;
3. current ownership of `foundation/mod.rs` and all future owned paths;
4. current mutating-writer count under the Terra/Sol scheduler;
5. Issue #530/#540 state and any newer accepted architecture;
6. the current status of the durable scope-assignment producer/consumer and whether the planned
   pre-production-only bootstrap can remain isolated from that authority;
7. whether any prerequisite became `UNKNOWN` or `CONFLICT`.

Only then may #162 explicitly activate the exact future branch/task. If activating it would become a
third concurrent mutating lead, the scheduler's explicit third-writer conditions must be proven and a
concrete throughput reason recorded; otherwise the allocation remains `NOT_ACTIVE` until a writer slot
is released.

The worker must not infer activation from this file, an alias, branch existence, tool availability or
chat history.

## Integration / closeout

The implementation candidate must remain frozen after material qualification. Integration is only via
the bound META 3.1 native exact-head Merge Queue contract, followed by real `merge_group` aggregate
`game-gate` success and protected-main readback.

Direct merge, generic auto-merge, bypass, force/rebase, no-op/retrigger commits and protection
weakening are forbidden.

After protected implementation readback, #162 archives the task/releases all owned paths and only then
recomputes readiness for representative PERF measurement, #508 and #139. No production readiness or
Reference-parity claim follows from this allocation.
