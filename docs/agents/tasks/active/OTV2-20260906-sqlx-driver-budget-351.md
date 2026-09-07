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

Historical allocation state: NOT_STARTED / NOT_ADMITTED before protection. Admitted by Work comment5560220858 at protected main `53c6bdf06a2282d893035a995c46052c88f935b4`, tree `ae5ef391163f79158dd723940edf819a275985c2`, branch `agent/sqlx-driver-budget-351`. Window1 began 2026-09-06T15:25:35Z. First checkpoint remains OPEN: exact two-crate import and same-owner reservation primitive are implemented; TLS accounting is not activated and PostgreSQL decoders remain upstream. Parallel-first: B,338 and346 retain disjoint lanes;351 has one exclusive worktree/branch. Cargo publication/integration is serial because247 and Dependabot259/260/261 overlap. No second driver writer.

## Validation and review

Focused: allocation-denial RED/GREEN and hostile-input/lifetime matrix in the plan. Component: vendored unit tests plus all affected normal workspace tests, fmt, strict Clippy, dependency/provenance checks and governance. Actual PostgreSQL17.6 is mandatory for driver integration; unconfigured/skipped tests are not SQL proof. E2E Server Seam/live source qualification remains outside scope and unclaimed. Self-review and genuinely independent full-change parser/resource review are required before final material freeze. Exact-head canonical CI and normal protected Merge Queue/readback remain pending. Checkpoint validation: owner-ledger RED/GREEN; vendored core library 11/11 tests (9 new), strict vendor Clippy; pinned Game server check, strict Clippy and 340/340 library tests pass. Vendor unit tests use its unchanged upstream lock (rustls0.23.40); Game check/tests use pinned rustls0.23.43. Initial isolated vendor run lacked cached dev dependencies; an initial compiler-shim invocation attempted the dependency toolchain and failed, then explicit Rust1.94 compiler paths resolved it. No tests or dependency declarations were suppressed. Governance passes. Full import whitespace inspection finds20 trailing-whitespace lines already byte-identical in upstream PostgreSQL; they are preserved and disclosed, not normalized or suppressed. Authored delta whitespace passes. These results qualify the primitive and import, not TLS or PostgreSQL17.6 integration. Complete TLS proof remains UNKNOWN: nested decoder capacities, private configuration/cache layouts and phased pre-call ownership bounds require closure. See vendored provenance for exact source terms and patch manifest.

## Window1 durable checkpoint

Work published draft PR356 at native `703da76a870356f2ab94d6e7af11675dd24f6c04`, tree `07591d3c8668dec06c4190eb4784be04b3351469`. Independent review found no primitive P0/P1; P2 complete-license evidence was repaired additively in both provenance files with VCS-pinned notices/digests, preserving imported bytes/modes. One repair cycle is recorded. Full TLS gate remains OPEN. Independent source review supports a behavior-equivalent private write-discard session store, but no adapter is implemented. Correlated ECH upper3276720 plus span327680/request524288/deframer65536 leaves80 bytes before other required owners; current conservative bounds therefore cannot certify accepted maxima. This is an unresolved proof, not an impossibility claim or permission to change limits. Complete source findings and conditional hook paths are recorded in core provenance.

## Window2 checkpoint

Work comment5560554622 opened window2 at2026-09-06T16:25:30Z after fresh native `1363c9b5b238f4922615eda9b502866c305e83bf` bind. Prior55m14s/4m46s-used/unused window1 history is preserved; no unused time was carried forward. Accepted337 maxima are independently conjunctive, so the historical80/254-byte arithmetic remainder is not a scope or architecture blocker. The actual owner ledger decides capacity, with no new semantic cap.

Implemented and independently reviewed unactivated components: complete pre-state grammar bound3276650, input-derived private unverified chain bound, and bounded synchronous Reader/File data-owner primitives with pre-growth old/new reservations and result custody. Sixteen core library tests and strict vendor/root Clippy pass; vendor tests use its upstream lock, root compilation uses pinned rustls0.23.43. The actual ECH8193 test and separately recorded mixed witness are distinguished in provenance. One strict-Clippy fixture cast repair advances repair count to2; rotations/retries remain0. Full TLS proof remains OPEN at complete phase composition and blocking-loader scheduler/Cell lifetime ownership. No TLS adapter, PostgreSQL decoder or actual hosted SQL/TLS qualification is claimed.

Prospective test lease PR357 is NOT_ACTIVE until protected admission and explicit grant; the shared durability test target remains untouched. Root owns publication and normal merge-up of that allocation. Core provenance contains source formulas, proof limitations and the smallest conditional loader owner/hooks.

## PR and closeout

### Owner continuation checkpoint (window3)

Protected main `b3e637dc43a0a31ff2caf24a6450f7df56b43777` was merged normally into the
preserved task history. The requested TLS gate is precisely **BLOCKED** at the
blocking-loader scheduler allocation owner. Tokio1.53.1 allocates a private generic
task `Cell` inside `spawn_blocking`; the admitted SQLx paths receive only its
`JoinHandle` and cannot reserve the actual layout before allocation or retain a
charge until the enclosing Cell is deallocated on success, error, cancellation,
shutdown and idle retention. Capturing the charge in the closure/result releases it
while the Cell backing still exists. SQLx's runtime dispatcher can also choose
async-global-executor, smol or async-std, making a copied Tokio-only byte constant
both incomplete and a forbidden hidden reservation. Exact source hashes, control
flow and the smallest required amendment are recorded in core provenance. Review
discussion3947483202 P1 is accepted/fixed here: a future protected amendment
cannot be Cell-only. It must also hold custody for operation-attributable
blocking-pool queue/map backing and worker/thread packets through their actual
idle/shutdown retention, or use an equivalent registered loading/runtime owner
where exact per-operation attribution of shared growth is not expressible.

No rustls/runtime/dependency source was changed and no PostgreSQL decoder work began.
Issue351 comment5560895137 activated the protected include-only durability-file
custody lease, superseding the older prospective wording preserved above, but the
module inclusion itself was not performed because the TLS gate is not proven.
Remaining OPEN cells: accounting-aware blocking-task Cell allocation and
blocking-pool retained-backing custody (or equivalent registered owner); complete
TLS phase composition and retained config/cache/session
ownership; real TLS-positive evidence; PostgreSQL accounting and hostile/positive
qualification; final whole-diff review, exact-head CI/MQ and protected readback.
This is an evidence-backed scope insufficiency checkpoint, not WP3 completion, B
readiness, or permission to alter TLS behavior.

One admitted branch/PR; Work retains publication/integration and lease-release control. Preserve immutable admission and all counters across normal merge-up and bounded windows. No force-push/rebase/reset, no-op retrigger or self-approval. Final head belongs in PR/check evidence, not a self-referential metadata commit. Work archives/releases only after terminal protected readback; technical scope insufficiency is not completion.

Window1 stopped at 2026-09-06T16:20:49Z: 3314 productive seconds conservatively charged, 286 unused seconds, no deducted pauses or reset. A further implementation window requires explicit Work continuation.

Window2 stopped at 2026-09-06T17:16:20Z: 3050 productive seconds conservatively charged, 550 unused seconds, zero deducted pauses and zero counter reset. Completed windows:2; repair cycles:2; rotations:0; identical-failure retries:0. Further implementation requires explicit Work continuation. Full TLS and PostgreSQL adapter gates remain OPEN; prospective PR357 remains NOT_ACTIVE.

## Context checkpoint

```yaml
last_progress: proved exact blocking-loader scheduler/Cell scope insufficiency after normal main merge-up
status: blocked_pending_scope_amendment
admission_state: ADMITTED
execution_window_number: 2
execution_windows_completed: 2
worker_rotations: 0
identical_failure_retries: 0
repair_cycles_for_current_gate: 2
owner_action_required: null
blocker: excluded_runtime_owners_lack_blocking_task_and_pool_backing_preallocation_and_lifetime_custody_hooks
next_action: owner decides whether to protect blocking-task plus blocking-pool retained-backing accounting, or an equivalent registered loading/runtime owner; no PostgreSQL work before TLS proof
```

## Window3 protected-amendment checkpoint

Current protected main was merged normally before the amendment checkpoint. RED
commit `f0dddec27ed8151f73d3db80750a89ccf15f3e77` reached SQLx-core compilation
and failed only for the missing sealed pre-spawn owner/API (`E0432`, `E0425`).
The configured root graph actually enables only Tokio 1.53.1. Inspection then
proved SQLx cannot deny before or retain custody through Tokio's private generic
task allocation, blocking `VecDeque`, worker `HashMap`, and worker/thread backing
through idle/shutdown. A closure/result reservation cannot own that enclosing
backing, and public metrics are post-admission observations rather than custody.
The RED fixture was removed and source mutation stopped.

`TLS_BLOCKING_OWNER = BLOCKED_RUNTIME_BACKEND_OWNER`. Smallest next amendment:
an exact Tokio preallocation/admission/final-release hook for task and attributable
pool backing, or an already-funded registered Tokio blocking-pool owner with typed
admission and real lifetime custody. Required dependency paths begin at Tokio
`src/runtime/blocking/pool.rs` and `src/runtime/task/{mod.rs,raw.rs,core.rs}`;
they are not authorized. No PostgreSQL or shared-target work follows.

```yaml
last_progress: protected amendment RED isolated the missing enabled-Tokio owner boundary
status: blocked_pending_runtime_backend_owner
blocker: tokio_1_53_1_has_no_public_preallocation_or_task_pool_custody_hook
next_action: protect an exact Tokio task-plus-pool owner hook or bind an already-funded registered runtime owner; keep TLS/PG OPEN
```

## Window4 Tokio amendment checkpoint

Work application #351 comment `5575674784` activated protected Tokio amendment
`main@e286291173dfadd963fd9fdd2cd71fe6211b40f2`, blob
`1ddc2e7335890244caa1eae18e20494afd26b14a`. All earlier windows, counters, REDs
and blocker checkpoints remain unchanged.

The complete crates.io Tokio 1.53.1 package was vendored at checksum
`202caea871b69668250d242070849eb495be178ed697a3e98aebce5bc81a0bed`, upstream
commit `75fef53d0a8590c2d1dbb63672aa7b7d1ef51155`, and selected by the root path
patch without version or feature downgrade. An initial automatic-test-discovery
setup failure receives no RED credit. Distinct amended RED
`45da01b13b848785ad7fe068c6e100b7cc3eebe5` reached Tokio compilation and failed
only because the protected owned blocking API is absent (`E0432`).

Source mutation stopped before GREEN at exact `SHARED_LEASE_REQUIRED` path
`vendor/tokio-1.53.1/src/task/mod.rs`, symbol `cfg_rt!` re-export list. The
allowed implementation file `src/task/blocking.rs` is private, while SQLx must
name the new fallible owned-spawn API. The smallest amendment is one public
re-export surface for that API; no alternative existing public hook exposes it.
No Tokio semantic source was modified. `TLS_BLOCKING_OWNER = NOT_PROVEN`; owner
task/queue/worker implementation and focused matrix, complete TLS composition,
real TLS-positive evidence, PostgreSQL accounting/PG17.6, independent whole-diff
review, canonical CI/MQ, protected readback and shared-target release remain OPEN.

```yaml
last_progress: vendored exact Tokio package and produced a compilation-valid amended RED
status: blocked_pending_shared_lease
blocker: tokio_task_mod_public_reexport_is_outside_protected_authored_allowlist
next_action: protect vendor/tokio-1.53.1/src/task/mod.rs only for the owned-blocking API re-export, then resume GREEN without expanding any other path
```

## Window5 owned-Tokio intermediate GREEN

Protected re-export authority at `main@feb6db96bd2fc93813cb120b874c61085f6dde45`
was applied under #351 comment `5575953751`. The owned Tokio entry point now
compiles and focused tests cover owner denial, overflow, funded completion,
dropped-handle running custody, idle worker retention, and shutdown release.
Task Cell custody is reserved from its concrete generic layout before allocation
and released after final Cell deallocation; the separate finite owner queue does
not spill into ordinary Tokio work, and an owned-created worker requires funded
explicit stack plus bookkeeping before OS spawn.

This is an intermediate GREEN, not the complete gate. Queue-full denial ordering,
queued abort, deterministic OS-spawn failure, loom concurrency, SQLx adapter and
funded certificate loader, complete TLS composition, actual TLS-positive proof,
and PostgreSQL 17.6 qualification remain OPEN. No shared PG target mutation was
made and ordinary `spawn_blocking` remains the control path.

```yaml
last_progress: implemented and focused-tested the protected Tokio owned task/queue/worker surface
status: implementation_in_progress
tls_blocking_owner: NOT_PROVEN
next_action: close the remaining Tokio pre-admission/lifetime matrix, then adapt the SQLx ledger and prove complete TLS before any PostgreSQL expansion
```
