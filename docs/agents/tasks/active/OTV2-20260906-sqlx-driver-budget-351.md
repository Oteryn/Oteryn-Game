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

## Window6 owned-Tokio prerequisite checkpoint

The intermediate-GREEN continuation now proves queue-full denial under the pool
lock before `BlockingTask` construction, Cell reservation/allocation, or owned
task admission, with no ordinary-queue fallback. Queued abort retains custody
until owner-queue removal and actual task destruction. A deterministic worker
spawn-failure control proves the never-created worker reservation and rolled-back
task reservation release exactly once while real owner-queue storage remains
charged until runtime destruction. The focused multithreaded race test proves
reservation/release equality across concurrent admissions.

Loom cannot exercise this private path through the pinned crate's existing
`src/runtime/tests/loom_blocking.rs` registration; adding that unlisted source or
a new feature route is not authorized. Deterministic multithreaded evidence was
therefore added in the already allocated focused test surface rather than
inventing a new Loom path. Ordinary `spawn_blocking` controls remain unchanged.

```yaml
last_progress: completed the owned-Tokio queue/cancellation/spawn-failure/concurrency prerequisite matrix
status: implementation_in_progress
tls_blocking_owner: NOT_PROVEN
next_action: adapt SQLx BlockingJobOwner and the certificate loader to the same ResourceBudget, fail closed on every non-Tokio backend, then complete TLS composition before PostgreSQL work
remaining_acceptance_cells: SQLx adapter and funded/denied loader; non-Tokio fail-closed matrix; complete TLS phase/capacity/lifetime composition; actual TLS-positive proof; PostgreSQL17.6 owned driver test; independent whole-diff review; canonical CI/MQ; protected readback and target release
```

## Window6 SQLx enabled-runtime adapter successor

The SQLx adapter now uses the existing `ResourceBudget` for the configured Tokio
owner path with no unowned fallback, and the funded/denied certificate-file
loader retains returned backing on that ledger. The configured root graph enables
Tokio only; other compiled runtime dispatch is deliberately not selected by this
owned entry point.

`TLS_BLOCKING_OWNER = PROVEN` for the enabled Tokio blocking prerequisite and
loader component. This does not prove complete TLS ownership or WP3:
CertificateInput activation, complete configuration/decoder/session/cache/
handshake overlap, actual TLS-positive evidence, PostgreSQL accounting and
configured PostgreSQL 17.6 remain OPEN.

```yaml
last_progress: adapted the enabled Tokio owner and added funded-denied SQLx certificate-loader composition
status: implementation_in_progress
tls_blocking_owner: PROVEN
next_action: complete and test TLS capacity/lifetime composition before any broad PostgreSQL decoder work or shared-target inclusion
```

## Window7 complete-TLS owner boundary

The narrow `TLS_BLOCKING_OWNER` result remains **PROVEN** for the configured
Tokio certificate-loader path.  Complete TLS is now stopped at the next actual
allocation owner: rustls 0.23.43 private
`src/msgs/deframer/buffers.rs::DeframerVecBuffer::{prepare_read,read}` grows and
shrinks its incoming `Vec<u8>` before calling the SQLx-provided reader.  SQLx can
observe returned byte count only after that allocation, and rustls exposes no
public capacity/pre-growth custody hook.  An SQLx wrapper, copied private growth
schedule or whole-handshake reservation cannot prove the required actual
capacity and old/new-overlap custody.

`SHARED_LEASE_REQUIRED = prospective vendor/rustls-0.23.43/src/msgs/deframer/buffers.rs`,
with necessary public owner wiring, solely for a fallible same-ledger pre-growth
reservation retained by the private deframer backing until real free or proved
charged transfer.  Preserve all TLS limits, modes, versions, certificate and
hostname verification, cache/security behavior and ordinary rustls behavior.
No rustls, PostgreSQL or shared-target source was changed.  Complete
configuration/decoder/session-cache/handshake overlap, real TLS-positive proof,
configured PostgreSQL17.6 qualification, independent review, canonical CI/MQ,
protected readback and shared-target release remain OPEN.

```yaml
last_progress: proved the first complete-TLS allocation-owner boundary after the narrow blocking-loader prerequisite
status: blocked_pending_shared_lease
tls_blocking_owner: PROVEN
complete_tls_accounting: NOT_PROVEN
blocker: rustls_private_deframer_grows_backing_before_sqlx_can_reserve_or_observe_capacity
next_action: protect a rustls deframer pre-growth same-ledger owner hook and necessary public wiring; do not start PostgreSQL qualification
```

Follow-up repair preserves the runtime-owner Arc across all certificate loads in
one operation. Tokio's protected owner queue compares owner identity; recreating
the adapter per load would incorrectly reject a funded second certificate/key
load. Sequential owned-job and sequential loader controls now pass. The exact
rustls deframer `SHARED_LEASE_REQUIRED` boundary above remains unchanged.

## Window8 protected rustls deframer owner successor

Protected #424 released the exact rustls 0.23.43 deframer amendment and Work
activated it for this existing worker. The checksum-verified published package is
vendored with only `buffers.rs`, `conn.rs`, `lib.rs` and provenance authored.
The incoming deframer now reserves the complete prospective exact-capacity
backing on the existing owner ledger before allocation and reader invocation,
retains old/new charges through replacement overlap, releases old custody only
after old backing destruction, and retains resized custody across read errors and
WouldBlock until connection drop. Unhooked and no-std behavior remains unchanged.

Focused SQLx tests exercise installation on a real rustls `ClientConnection`,
funded WouldBlock retention/drop release, and denial before reader invocation.
`TLS_BLOCKING_OWNER = PROVEN` remains narrow and the deframer-owner hook is
`PROVEN`. This is not complete TLS or WP3 acceptance.

```yaml
last_progress: implemented and qualified the protected rustls deframer exact-capacity owner hook
status: implementation_in_progress
tls_blocking_owner: PROVEN
rustls_deframer_owner: PROVEN
complete_tls_accounting: NOT_PROVEN
next_action: compose remaining accepted TLS configuration, decoded structures, session/cache and handshake overlap on the same ledger, then obtain separate TLS-positive and configured PostgreSQL17.6 evidence
remaining_acceptance_cells: complete TLS configuration/decoder/session-cache/handshake-overlap custody; actual TLS-positive proof; PostgreSQL17.6 owned driver test; independent whole-diff review; canonical CI/MQ; protected readback and target release
```

## Window9 decoded-message owner stop

The deframer byte-buffer hook and narrow `TLS_BLOCKING_OWNER` remain **PROVEN**.
Complete TLS composition next reaches rustls
`ConnectionCore::deframe` before `Message::try_from` / `into_owned`, where the
private decoded AST and retained handshake backing allocate before SQLx can
observe the branch or reserve its actual capacity.  The protected #424
`conn.rs` authority is limited to deframer-owner wiring and does not grant this
decoded-state owner hook.

The established conservative phase values consume 4,194,154 of the accepted
4,194,304-byte slot before nonzero configuration, crypto, chain, session/cache,
send, transcript and error owners.  Therefore pre-reserving existing bounds
cannot prove a feasible accepted positive case and must not be converted into a
whole-slot magic allowance or semantic shrink.

```yaml
last_progress: proved the next private decoded-message allocation boundary after the protected deframer hook
status: blocked_pending_shared_lease
tls_blocking_owner: PROVEN
rustls_deframer_owner: PROVEN
complete_tls_accounting: NOT_PROVEN
blocker: rustls_private_decoded_message_allocates_before_sqlx_can_reserve_or_transfer_custody
next_action: protect a same-ledger decoded-message owner hook at ConnectionCore::deframe before Message::try_from / into_owned, plus necessary public wiring
remaining_acceptance_cells: decoded AST and retained-chain owner; configuration/crypto/session-cache/send/transcript/error and handshake-overlap custody; actual TLS-positive proof; PostgreSQL17.6 owned driver test; independent whole-diff review; canonical CI/MQ; protected readback and target release
```

## Window10 decoded-owner amendment preflight

The protected decoded-owner amendment is integrated and explicitly applied to
this same worker.  A boundary-by-boundary preflight stopped before source
mutation because its mandatory ClientHello extension/payload qualification
requires authored changes in `client/hs.rs::emit_client_hello_for_retry`, while
the exact `client/hs.rs` lease names only the
`ExpectServerHelloOrHelloRetryRequest` methods and successor-state construction.
The unchanged function allocates the extensions box, collected named-groups
vector, cloned protocol/transport payloads and owned certificate-authority names
before any granted decoded-reader or connection hook can reserve them.

```yaml
last_progress: normally merged protected main and preflighted the protected decoded-owner symbol boundaries
status: blocked_pending_shared_lease
tls_blocking_owner: PROVEN
rustls_deframer_owner: PROVEN
complete_tls_accounting: NOT_PROVEN
blocker: client_hello_allocations_are_outside_the_exact_client_hs_symbol_lease
next_action: authorize client/hs.rs::emit_client_hello_for_retry for same-ledger actual-backing custody, then resume the 13-path decoded-owner RED/GREEN matrix
remaining_acceptance_cells: all protected decoded-owner RED/GREEN boundaries; complete TLS configuration/crypto composition; actual TLS-positive proof; PostgreSQL17.6 owned driver test; independent whole-diff review; canonical CI/MQ; protected readback and target release
```

## Window11 ClientHello symbol-amendment preflight

The ClientHello symbol amendment is protected and applied, and current
protected `main@e3d8a46871a98a309c73b3febaa41a7e6d2ec408` was normally merged.
Preflight found that configured ALPN/protocol backing is cloned in
`ClientHelloInput::new` before control reaches the newly authorized
`emit_client_hello_for_retry`. The mandatory clone-overlap and
denial-before-allocation proof therefore cannot be implemented solely within
the new symbol. No rustls semantic source was changed.

```yaml
last_progress: merged current protected main and preflighted the ClientHello symbol amendment
status: blocked_pending_shared_lease
tls_blocking_owner: PROVEN
rustls_deframer_owner: PROVEN
complete_tls_accounting: NOT_PROVEN
blocker: configured_protocol_clone_allocates_in_unlisted_ClientHelloInput_new_before_emit_client_hello_for_retry
next_action: authorize client/hs.rs::ClientHelloInput::new only for same-ledger protocol-vector clone custody, then resume the protected decoded-owner RED/GREEN matrix
remaining_acceptance_cells: all protected decoded-owner RED/GREEN boundaries; complete TLS configuration/crypto composition; actual TLS-positive proof; exact-head PostgreSQL17.6 owned driver test; independent whole-diff review; canonical CI/MQ; protected readback and target release
```
## Window11 ClientHello owner installation boundary

The protected ClientHello symbol amendment was integrated at
`main@a2ba218f94e83b36443afcdbd6ec8b748a677efe`, explicitly applied to this
worker, and normally merged into the canonical lineage.  Preflight before
semantic mutation proved that `emit_client_hello_for_retry` executes during
`ClientConnection::new`/`new_with_alpn` via `ConnectionCore::for_client`, before
SQLx receives the connection and can call the existing owner setter.  The #425
lease does not include these `client/client_conn.rs` constructor symbols.

```yaml
last_progress: merged protected ClientHello authority and proved the earlier owner-installation boundary before source mutation
status: blocked_pending_shared_lease
tls_blocking_owner: PROVEN
rustls_deframer_owner: PROVEN
complete_tls_accounting: NOT_PROVEN
blocker: ClientHello construction runs before SQLx can install the accepted owner
next_action: authorize an owner-aware ClientConnection constructor / ConnectionCore::for_client wiring that installs the existing owner before ClientHelloInput::new and start_handshake
shared_lease_required: vendor/rustls-0.23.43/src/client/client_conn.rs :: ClientConnection::{new,new_with_alpn} / ConnectionCore::for_client :: pre-ClientHello same-owner installation
remaining_acceptance_cells: ClientHello and all decoded-owner RED/GREEN boundaries; complete TLS configuration/crypto composition; actual TLS-positive proof; PostgreSQL17.6 owned driver test; independent whole-diff review; canonical CI/MQ; protected readback and target release
```

## Window12 ClientHello constructor-owner application

```yaml
last_progress: normally merged protected #429 and proved the production SQLx call has no accepted owner value to pass to the new constructor
status: blocked_pending_shared_lease
tls_blocking_owner: PROVEN
rustls_deframer_owner: PROVEN
complete_tls_accounting: NOT_PROVEN
shared_lease_required: vendor/sqlx-core-0.9.0/src/net/tls/mod.rs :: TlsConfig / vendor/sqlx-postgres-0.9.0/src/connection/tls.rs :: maybe_upgrade :: propagate the already-accepted operation ResourceBudget identity
next_action: allocate the exact operation-owner propagation source and lifecycle, then implement the protected owner-aware ALPN constructor seam
remaining_acceptance_cells: owner-aware ALPN RED/GREEN; protected ClientHello and decoded-owner matrix; session/cache and configuration/crypto ownership; complete TLS; TLS-positive proof; PostgreSQL17.6 qualification; independent review; canonical CI/MQ; protected readback and target release
```

## Window13 operation-owner prerequisite repair

Protected #430 is merged and applied. Before adding the caller propagation path,
the accepted review P1 was reproduced in the existing Tokio owner surface: a
runtime retained one first-owner queue and rejected a later independent owner.
The queue is now isolated and charged per owner through linked nodes, with no
ordinary-queue spill and custody retained through idle worker/runtime shutdown.

```yaml
last_progress: fixed and proved distinct operation owners on one Tokio runtime before SQLx propagation
status: active_owner_propagation
review_p1_3947483202: PROVEN_FIXED
tls_blocking_owner: PROVEN
rustls_deframer_owner: PROVEN
operation_owner_propagation: NOT_PROVEN
next_action: implement #430 caller-supplied Arc propagation RED/GREEN, then and only then resume #429 ALPN ownership
remaining_acceptance_cells: operation-owner propagation; owner-aware ALPN RED/GREEN; protected ClientHello and decoded-owner matrix; session/cache and configuration/crypto ownership; complete TLS; TLS-positive proof; PostgreSQL17.6 qualification; independent review; canonical CI/MQ; protected readback and target release
```

## Window14 operation-owner propagation

```yaml
last_progress: implemented the separate caller-supplied owner path from PgConnection through rustls construction
status: active_alpn_owner
review_p1_3947483202: PROVEN_FIXED
tls_blocking_owner: PROVEN
rustls_deframer_owner: PROVEN
operation_owner_propagation: PROVEN
ordinary_connect_and_pool_owner_free: PROVEN
owner_aware_tls_no_fallback: PROVEN
owner_aware_alpn: NOT_PROVEN
next_action: execute protected #429 ALPN/protocol allocation custody with this same propagated owner
remaining_acceptance_cells: owner-aware ALPN RED/GREEN; protected ClientHello and decoded-owner matrix; session/cache and configuration/crypto ownership; complete TLS; TLS-positive proof; PostgreSQL17.6 qualification; independent review; canonical CI/MQ; protected readback and target release
```

## Window15 propagation review repair and protected ALPN seam

Independent review of `a03d6a2bf28c59d3292ffe1df739b7aa7c35d690` returned four P1s. All four are accepted and fixed in this checkpoint: the rustls owner-aware constructor now enters a separate preconstruction path; the owned SQLx handshake uses the protected blocking certificate loader for inline/file roots, client certificates and keys; the unused public `PgConnection::resource_budget` API is removed; and focused PostgreSQL driver tests exercise the real establish/stream/SSLRequest/TLS chain.

```yaml
status: blocked_pending_shared_lease
review_p1_3956941303: PROVEN_FIXED
review_p1_3956941320: PROVEN_FIXED
review_p1_3956941331: PROVEN_FIXED
review_p1_3956941337: PROVEN_FIXED
operation_owner_propagation: PROVEN
owner_aware_alpn: PROVEN
tls_blocking_owner: PROVEN
rustls_deframer_owner: PROVEN
complete_tls_accounting: NOT_PROVEN
next_action: allocate session-cache retrieval custody before continuing ClientHello emission
shared_lease_required: vendor/rustls-0.23.43/src/client/hs.rs :: ClientSessionValue::retrieve :: owner-aware construction reaches this unallocated retained-session boundary before the protected ClientHello emit path
remaining_acceptance_cells: session-cache retrieval; protected ClientHello emit and decoded-owner cells; key-share/ECH/configuration/crypto ownership; complete TLS; TLS-positive proof; PostgreSQL17.6 final-candidate qualification; whole-diff review; canonical CI/FULL MQ; protected readback and target release
```

## Window16 protected session-retrieval owner

```yaml
status: blocked_pending_shared_lease
session_retrieval_owner: PROVEN
operation_owner_propagation: PROVEN
tls_blocking_owner: PROVEN
rustls_deframer_owner: PROVEN
complete_tls_accounting: NOT_PROVEN
last_progress: added fail-closed owner-aware TLS1.2 store dispatch, pre-clone memory-cache reservation, retained-session RAII clone custody, TCP-only retrieval wiring, and focused overlap/denial/custom-store controls
next_action: obtain exact key-exchange allocation authority before continuing ClientHello emission and the protected decoded-owner continuation
shared_lease_required: vendor/rustls-0.23.43/src/client/tls13.rs :: initial_key_share / SupportedKxGroup::start :: owner-aware ClientHello construction reaches active key-exchange allocation before protected emit/decode continuation
remaining_acceptance_cells: unresolved client-PEM clone review P1; key-share/ECH/configuration/crypto ownership; protected ClientHello emit and decoded-owner cells; complete TLS composition; TLS-positive proof; PostgreSQL17.6 final-candidate qualification; whole-diff review; canonical CI/FULL MQ; protected readback and target release
```

## Window17 client-PEM clone review repair

```yaml
status: blocked_pending_shared_lease
review_p1_3957222192: PROVEN_FIXED
session_retrieval_owner: PROVEN
operation_owner_propagation: PROVEN
tls_blocking_owner: PROVEN
rustls_deframer_owner: PROVEN
complete_tls_accounting: NOT_PROVEN
last_progress: eliminated the unreserved client-certificate and private-key PEM clones by parsing the charged loader backings by reference; both charged sources remain alive through parsing and release only after their backing is destroyed
next_action: obtain exact key-exchange allocation authority before continuing ClientHello emission and the protected decoded-owner continuation
shared_lease_required: vendor/rustls-0.23.43/src/client/tls13.rs :: initial_key_share / SupportedKxGroup::start :: owner-aware ClientHello construction reaches active key-exchange allocation before protected emit/decode continuation
remaining_acceptance_cells: key-share/ECH/configuration/crypto ownership; protected ClientHello emit and decoded-owner cells; complete TLS composition; TLS-positive proof; PostgreSQL17.6 final-candidate qualification; whole-diff review; canonical CI/FULL MQ; protected readback and target release
```

## Window18 post-KX configuration-owner preflight

Protected #451 KX/provider-resident work was initially retained as completed.  After the
normal merge of protected `main@b26395edff3dde1ebcc155ab70758520d780884c`,
the next resource-owned handshake allocation was preflighted before further
semantic mutation.  `aws_lc_rs::default_provider()` allocates the cipher-suite
and non-FIPS default KX-group vectors inside an explicitly read-only #451 path,
before SQLx can observe actual capacity or bind custody to the returned
provider.

```yaml
status: blocked_pending_shared_lease
tls_blocking_owner: PROVEN
rustls_deframer_owner: PROVEN
session_retrieval_owner: PROVEN
operation_owner_propagation: PROVEN
aws_lc_kx_provider_resident: NOT_PROVEN
complete_tls_accounting: NOT_PROVEN
last_progress: normally merged current protected main and proved the earliest post-KX provider-configuration allocation boundary
shared_lease_required: vendor/rustls-0.23.43/src/crypto/aws_lc_rs/mod.rs :: default_provider / default_kx_groups :: same-ledger preallocation and lifetime custody for actual cipher-suite and KX-group Vec capacities
next_action: obtain the exact provider-configuration owner amendment, then resume decoded, ClientHello, session/cache and complete TLS composition on the same ledger
remaining_acceptance_cells: provider configuration vectors; all protected decoded/ClientHello composition; remaining session/cache/error/handshake overlap; funded AWS-LC TLS-positive handshake; PostgreSQL17.6 qualification; independent whole-diff review; canonical CI/FULL MQ; protected readback and target release
```

## Window19 KX full-lifetime custody repair

Coordinator evidence `5597261954` invalidated the earlier KX `PROVEN` label.
The owner-aware start path had allocated a second wrapper `Box`, outside the
protected 554/1625/1705/6264/7881 bounds, and released the full KX debit as
soon as provider completion returned while the returned secret still lived.

The repair returns the owner-aware wrapper inline, stores it directly in the
existing handshake state, and transfers the non-allocating reservation token
to the returned secret.  The TLS 1.3 handler releases that token only after
the synchronous `into_handshake(secret)` call has consumed and destroyed the
secret.  Focused coverage now proves all five exact/max-minus-one starts,
whole and hybrid-component returned-secret retention, simulated HRR
initial/replacement overlap, failure cleanup, and ordinary owner-free control.
The broader #451 matrix has not yet re-established concurrent provider-first
use, thread churn, actual HRR wire handling, and cancellation on this repaired
shape, so the aggregate KX verdict remains `NOT_PROVEN` rather than inheriting
the predecessor claim.

```yaml
status: blocked_pending_shared_lease
kx_wrapper_heap_p1: PROVEN_FIXED
kx_returned_secret_lifetime_p1: PROVEN_FIXED
aws_lc_kx_provider_resident: NOT_PROVEN
complete_tls_accounting: NOT_PROVEN
shared_lease_required: vendor/rustls-0.23.43/src/crypto/aws_lc_rs/mod.rs :: default_provider / default_kx_groups :: same-ledger preallocation and lifetime custody for actual cipher-suite and KX-group Vec capacities
next_action: obtain the provider-configuration owner amendment, then finish the repaired #451 matrix before any aggregate KX GREEN claim
```

## Window20 KX publication/layout readback repair

The canonical remote already contained the inline-custody repair when this
publication recovery began.  Canonical Linux workspace run `34323039456`, job
`102373930802`, then invalidated its claim that the shared classical source has
one layout.  The canonical graph compiles a ring `KeyExchange = 208` and an
AWS-LC `KeyExchange = 200`; ML-KEM `Active = 40` and hybrid
`ActiveHybrid = 96` remain unchanged.  The drift assertion now locks both exact
classical provider shapes, while the owner-aware entry point admits only the
reviewed 200-byte AWS-LC allocation before provider start.
`KxReservation` remains outside those provider structs in
the caller-owned, inline `ResourceOwnedKx`; the focused test also locks that
control value to 40 bytes and therefore detects reintroduction of a boxed owner
wrapper.  The protected `554/1625/1705/6264/7881` AWS-LC bounds therefore remain
valid; the 208-byte ring shape is not admitted by owner-aware KX.

```yaml
status: blocked_pending_shared_lease
kx_wrapper_heap_p1: PROVEN_FIXED
kx_returned_secret_lifetime_p1: PROVEN_FIXED
kx_protected_provider_layouts: PROVEN
kx_exact_target_full_lifetime_bounds: PROVEN_AWS_LC_554_1625_1705_6264_7881
aws_lc_kx_provider_resident: NOT_PROVEN
complete_tls_accounting: NOT_PROVEN
shared_lease_required: vendor/rustls-0.23.43/src/crypto/aws_lc_rs/mod.rs :: default_provider / default_kx_groups :: same-ledger preallocation and lifetime custody for actual cipher-suite and KX-group Vec capacities
next_action: obtain the provider-configuration owner amendment, then finish actual HRR/cancellation and the remaining #451 matrix before any aggregate KX GREEN claim
```

## Window21 provider-configuration shared custody

Protected #453 is applied after the normal merge of `main@0c69d04a49778e539515fb6848b0ab89268c1fa9`. The exact-target owner-aware AWS-LC constructor now establishes process/current-thread residency first, computes the Rust 1.94 two-Vec plus `ArcInner<CryptoProvider>` layout with checked arithmetic, debits the same shared root before allocation, and retains one canonical provider/debit for process lifetime. SQLx consumes that shared Arc directly. Provider registration now stores only a non-allocating initialized flag rather than promoting a caller wrapper Arc to process lifetime, and SQLx separately precharges its per-connection owner-wrapper Arc allocation with destruction-ordered custody.

```yaml
status: active
provider_configuration_owner: PROVEN_FOCUSED
provider_registration_bookkeeping: PROVEN_FIXED
kx_protected_bounds: PROVEN_AWS_LC_554_1625_1705_6264_7881
aws_lc_kx_provider_resident: NOT_PROVEN
complete_tls_accounting: NOT_PROVEN
remaining_acceptance_cells: full #451 HRR/thread-churn/cancellation matrix; decoded/ClientHello/session/cache/send/transcript/error composition; funded TLS-positive; PostgreSQL17.6 positive/hostile qualification; independent review; exact-head CI/FULL MQ; protected readback
next_action: reproduce the remaining #451 matrix on the provider-shared final shape, then continue every already-protected TLS custody cell
```

## Window22 provider proof repair and PQ profile boundary

Independent exact-head review found that provider validation allocated two
temporary vectors outside #453's protected provider-configuration formula and
that the required four-thread racing first-use test had not executed.  The
validation is now allocation-free (`len`/capacity checks plus iterator `zip`),
and the executable SQLx AWS-LC test races four first calls in a fresh process,
asserts one pointer-identical provider, exactly one configuration shared debit,
one process debit, and the distinct 1,360-byte debit for each participating
thread.  The main test thread then registers its own 1,360-byte residency before
the existing per-connection KX bounds run on separate ledgers.

Exact `cargo tree -e features` evidence for the qualified SQLx AWS-LC profile
shows rustls `aws-lc-rs`, `aws_lc_rs`, `std`, and `tls12`, but no
`prefer-post-quantum`.  Consequently the profile's ordinary provider is
hybrid-last, and equality against that provider cannot establish the protected
PQ-first order.  This task does not have authority to change the manifest or
manually reorder the groups.

```yaml
status: blocked_pending_shared_lease
provider_configuration_owner: NOT_PROVEN
provider_validation_allocation_free: PROVEN
provider_racing_first_use: PROVEN
pq_first_profile: NOT_PROVEN
complete_tls_accounting: NOT_PROVEN
shared_lease_required: vendor/sqlx-core-0.9.0/Cargo.toml :: _tls-rustls-aws-lc-rs / rustls prefer-post-quantum feature :: protected #451/#453 require the qualified ordinary AWS-LC default provider to be PQ-first, while the exact SQLx AWS profile disables rustls defaults and currently does not enable prefer-post-quantum
next_action: obtain the exact SQLx AWS profile feature amendment, assert the explicit four-name PQ-first order under that profile, then resume the remaining #451 and WP3 matrices
```

## Window23 protected PQ-first feature closure

Protected #458 is applied after normally merging protected
`main@4f1ce7b4c3092a79ffa42e0b63e786015dedea53`.  The exact SQLx AWS-LC profile
now requests `rustls/prefer-post-quantum` directly and the executable provider
test asserts the explicit ordinary and owner-aware order
`X25519MLKEM768 -> X25519 -> secp256r1 -> secp384r1`, with exactly four
qualified default entries.  No root manifest or lockfile change is required.

```yaml
status: active
pq_first_profile: PROVEN_FOCUSED
provider_configuration_owner: PROVEN_FOCUSED
aws_lc_kx_provider_resident: NOT_PROVEN
complete_tls_accounting: NOT_PROVEN
remaining_acceptance_cells: actual HRR; thread churn; cancellation/error/drop; decoded/config/session/cache/send/transcript custody; TLS-positive; PostgreSQL17.6 positive/hostile qualification; independent review; exact-head CI/FULL MQ; protected readback
next_action: finish the full #451 final-graph matrix, then continue every already-protected TLS and PostgreSQL custody cell
```

## Window24a actual TLS 1.3 HelloRetryRequest proof

The executable SQLx AWS-LC harness now drives a real client/server TLS 1.3
exchange.  The owner-aware PQ-first client initially holds the protected 7,881
byte X25519MLKEM768 reservation; an ordinary P-256-only server emits a real
HelloRetryRequest.  Peak-ledger observation proves the 1,625-byte replacement
is reserved while the initial charge is still held, and the post-transition
ledger proves the initial backing is destroyed before its charge is released.
The handshake completes and reports `FullWithHelloRetryRequest` without
weakening certificate, protocol, provider, group-order, or ordinary server
semantics.

```yaml
status: active
actual_hrr_wire: PROVEN
actual_hrr_kind: FullWithHelloRetryRequest
actual_hrr_overlap: PROVEN_7881_PLUS_1625
aws_lc_kx_provider_resident: NOT_PROVEN
complete_tls_accounting: NOT_PROVEN
remaining_acceptance_cells: initial/post-HRR cancellation ordering; sequential thread churn; complete decoded/config/session/cache/send/transcript/error custody; funded SQLx TLS-positive; PostgreSQL17.6 positive/hostile qualification; independent review; exact-head CI/FULL MQ; protected readback
next_action: finish cancellation and thread-churn controls, then continue the complete TLS ownership matrix
```

## Window24b retained provider-thread churn

Protected `main@e1750ede386c0ee1001894ab9d91129de5d03fce` was merged normally.
The exact AWS-LC SQLx harness now funds three additional new-thread 1,360-byte
registrations on the same root, proves repeated use on each thread is
allocation/debit-free, retains every registration after thread exit, and
rejects the next thread at the residency preflight before AWS-LC use.

```yaml
status: active
provider_thread_churn: PROVEN_FOCUSED
provider_thread_repeat: PROVEN_FOCUSED
provider_thread_exhaustion_pre_use_denial: PROVEN_FOCUSED
aws_lc_kx_provider_resident: NOT_PROVEN
complete_tls_accounting: NOT_PROVEN
remaining_acceptance_cells: actual wire HRR; cancellation/error/drop matrix; decoded/config/session/cache/send/transcript/error custody; funded TLS-positive; PostgreSQL17.6 positive/hostile qualification; independent review; exact-head CI/FULL MQ; protected readback
next_action: execute actual HRR and cancellation lifecycle proof, then continue the protected complete-TLS matrix
```

## Window25 actual wire HRR lifecycle

The executable exact-graph AWS-LC harness now performs a real TLS 1.3
client/server exchange.  The PQ-first owner-aware client offers
X25519MLKEM768 and the P-256-only ordinary server emits a real
HelloRetryRequest.  Event and peak accounting prove that the 1,625-byte P-256
replacement is reserved while the 7,881-byte initial exchange is still held,
and that replacement admission precedes initial release.  Funding the live
initial state plus only 1,624 bytes fails before replacement start, then
releases the initial reservation during actual fatal-state destruction.  The
funded exchange completes with `FullWithHelloRetryRequest`; separate pre-HRR
and post-HRR connection drops prove release of the live initial and replacement
reservations only with their owning connection state destruction.

```yaml
status: active
wire_hrr_initial_replacement_overlap: PROVEN_FOCUSED
wire_hrr_replacement_denial_before_start: PROVEN_FOCUSED
kx_connection_drop_before_and_after_hrr: PROVEN_FOCUSED
aws_lc_kx_provider_resident: NOT_PROVEN
complete_tls_accounting: NOT_PROVEN
remaining_acceptance_cells: actual TLS key-schedule custody assertion; complete decoded/config/session/cache/send/transcript/error accounting; funded SQLx TLS-positive; PostgreSQL17.6 positive/hostile qualification; independent review; exact-head CI/FULL MQ; protected readback
next_action: prove the remaining returned-secret key-schedule and cancellation/error controls, then continue complete TLS custody on the protected paths
```

## Window26 final #451 lifecycle closure

The funded real-wire HRR exchange now observes the replacement reservation's
release during the successful TLS packet-processing sequence that consumes
`ResourceOwnedSecret` through
`KeySchedulePreHandshake::into_handshake(secret)`.  Rustls keeps the
reservation separate until that synchronous call returns.  Combined with the
same final-graph test's initial and post-HRR connection drops, replacement
denial before provider start, 7,881 + 1,625 overlap and successful completion,
and the focused invalid-peer completion-error unwind, every remaining protected
#451 lifecycle cell is executable on the exact AWS-LC graph.

```yaml
status: active
aws_lc_kx_provider_resident: PROVEN
complete_tls_accounting: NOT_PROVEN
kx_initial_owner_drop: PROVEN_ACTUAL_STATE
kx_post_hrr_drop: PROVEN_ACTUAL_STATE
kx_hrr_replacement_denial: PROVEN_BEFORE_START
kx_completion_error: PROVEN
kx_secret_key_schedule_custody: PROVEN_ACTUAL_TLS
remaining_acceptance_cells: complete decoded/config/session/cache/send/transcript/error custody and handshake overlap; funded SQLx TLS-positive; PostgreSQL17.6 positive/hostile qualification; independent review; exact-head CI/FULL MQ; protected readback
next_action: continue complete TLS accounting on the already-protected paths
```

## Window27 decoded-owner span checkpoint

The protected #425 implementation has begun at the earliest independent decoded
backing: owner-aware client construction now creates `HandshakeDeframer` with
its operation owner before the initial `Vec<FragmentSpan>` allocation.  Span
capacity is reserved with checked layout arithmetic before allocation and each
growth replacement holds old plus prospective-new custody until the old vector
is destroyed.  Draining messages retains high-water capacity; final deframer
drop destroys the vector before releasing its charge.  Ordinary/no-std and
unowned QUIC construction retain the upstream path.

```yaml
status: active
aws_lc_kx_provider_resident: PROVEN
handshake_span_capacity_owner: PROVEN_FOCUSED
complete_tls_accounting: NOT_PROVEN
remaining_acceptance_cells: generic reader/list/payload/message ownership; transcript; decoded ClientHello/ServerHello; certificate/OCSP and successor-state custody; compressed certificate overlap; retained-session composition; complete TLS witness; TLS-positive; PostgreSQL17.6; review/CI/MQ/readback
next_action: continue the remaining protected #425 boundaries on this same lineage
```

## Window27b decoded reader/list checkpoint

The span checkpoint was normally reconciled with the path-disjoint decoded
reader work. The same owner is now retained by `ConnectionCore`, supplied to
ordinary and first-message decode entry points, and propagated through nested
`Reader::sub` calls. Generic TLS lists reserve checked prospective element
capacity before each exact-capacity growth and hold old plus new capacity until
the allocator has destroyed the old backing. Focused funded and max-minus-one
controls cover this boundary.

```yaml
status: active
decoded_owner_reader_propagation: PROVEN_FOCUSED
decoded_owner_generic_list_growth: PROVEN_FOCUSED
complete_tls_accounting: NOT_PROVEN
remaining_acceptance_cells: payload/message rollback and ownership; transcript; decoded ClientHello/ServerHello; certificate/OCSP and successor-state custody; compressed certificate overlap; retained-session composition; complete TLS witness; TLS-positive; PostgreSQL17.6; review/CI/MQ/readback
next_action: continue #425 at payload/message ownership and nested failure rollback
```

## Window28b decoded span growth-denial regression

The span-owner test matrix funds a full 16-element vector with one byte less
than the old-plus-32-element replacement overlap. Denial leaves the original
pointer, capacity, contents, and charge unchanged and releases only after the
original backing is dropped. Complete TLS accounting remains `NOT_PROVEN`.

## Window29 decoded bookkeeping and rollback repair

Review P1 `DecodedOwner` bookkeeping is repaired at the construction boundary:
the exact pinned Rust 1.94 `ArcInner<DecodedOwner>` layout is reserved before
`Arc::new`, and external connection custody releases that debit only after the
last decoded-owner Arc/control block has been destroyed. Generic list growth
again follows checked geometric amortization rather than peer-controlled
one-element reallocations. A local rollback guard now destroys partially
decoded list backing before releasing its current capacity debit on a later
element error, and capacity-qualification mismatch explicitly destroys the
prospective backing before rolling back both prospective and replaced debits.

```yaml
status: active
decoded_owner_arc_control_block: PROVEN_FOCUSED
decoded_list_geometric_growth: PROVEN_FOCUSED
decoded_list_later_error_rollback: PROVEN_FOCUSED
decoded_list_success_backing_custody: NOT_PROVEN
complete_tls_accounting: NOT_PROVEN
remaining_acceptance_cells: backing-coupled successful-list custody and transfer; payload/message ownership; transcript; decoded ClientHello/ServerHello; certificate/OCSP and successor-state custody; compressed certificate overlap; retained-session composition; complete TLS witness; TLS-positive; PostgreSQL17.6; review/CI/MQ/readback
next_action: replace successful-list aggregate custody with backing-coupled transfer before continuing dependent payload/message cells

## Window29 decoded byte-payload checkpoint

Length-prefixed `PayloadU8` and `PayloadU16` decoding now inherits the same
connection decoded owner through nested readers. The complete byte-vector
capacity is reserved before copying; the qualified allocation must return that
actual capacity or its backing is destroyed before the prospective debit is
rolled back. Owner-aware message parsing also checkpoints the aggregate before
decode and returns nested parse-error charges only after partial values have
unwound. Generic-list allocation-mismatch cleanup was repaired to destroy the
replacement before releasing both old and prospective capacity charges.

```yaml
status: active
decoded_owner_payload_u8_u16: PROVEN_FOCUSED
decoded_owner_nested_message_rollback: PROVEN_IMPLEMENTED
complete_tls_accounting: NOT_PROVEN
remaining_acceptance_cells: Payload borrowed-to-owned custody; Message into_owned parsed+encoded overlap; handshake AST/DNS/Box ownership; transcript; certificate/OCSP and successor-state custody; compressed-certificate overlap; retained-session composition; complete TLS witness; TLS-positive; PostgreSQL17.6; review/CI/MQ/readback
next_action: continue #425 at Payload into_owned and Message parsed/encoded transfer, then handshake AST ownership
```

## Window28b decoded span growth-denial regression

The span-owner test matrix funds a full 16-element vector with one byte less
than the old-plus-32-element replacement overlap. Denial leaves the original
pointer, capacity, contents, and charge unchanged and releases only after the
original backing is dropped. Complete TLS accounting remains `NOT_PROVEN`.

## Window28 decoded-owner allocation repair

Review of the first decoded-owner checkpoint found two allocation-order defects.
The connection now reserves the exact Rust 1.94 `ArcInner<DecodedOwner>` layout
before `Arc::new`; separate connection custody is declared after the final
decoded-owner Arc and therefore releases only after the control block is
deallocated. Constructor failure explicitly destroys the Arc before releasing
that prospective debit. Generic decoded lists now preserve amortized geometric
growth for both owned and ordinary readers. The owned path reserves the full
prospective replacement capacity before allocation, verifies actual capacity,
keeps old and new charges live through the move, and rolls back the prospective
debit after destroying a mismatched replacement.

The remaining review finding is not closed: list backing still uses the
connection-scoped aggregate charge and therefore lacks per-backing
destruction-time release/transfer. Payload/message and all later #425 custody
remain open, so complete TLS accounting is not proven.

```yaml
status: active
decoded_owner_arc_preallocation: PROVEN_FOCUSED
decoded_owner_geometric_list_growth: PROVEN_FOCUSED
decoded_close_on_backing_drop: NOT_PROVEN
complete_tls_accounting: NOT_PROVEN
remaining_acceptance_cells: per-backing list custody; payload/message rollback; transcript; certificate/OCSP; successor and peer-chain transfer; compressed certificate; retained sessions; complete handshake; TLS-positive; PostgreSQL17.6; review/CI/MQ/readback
next_action: replace aggregate-only list custody with backing-bound ownership, then continue payload/message ownership
```

## Window30 backing-bound custody preflight

A concrete backing-bound implementation attempt established that the current
`Message<'a>` representation cannot carry decoded custody without updating all
of its construction sites. Adding a final custody field (after `payload`, so
payload backing is destroyed first) compiled through the decoded client path
but failed at existing constructors in excluded server handshake and client ECH
surfaces. Keeping the existing bare `Vec<T>` return from `Codec::read` has no
Drop hook, allocator hook, or destination field in which custody can travel;
the prohibited alternatives are an address registry or aggregate connection
retention.

```yaml
status: blocked_pending_shared_lease
decoded_close_on_backing_drop: NOT_PROVEN
complete_tls_accounting: NOT_PROVEN
next_action: "SHARED_LEASE_REQUIRED = vendor/rustls-0.23.43/src/client/ech.rs + src/server/tls12.rs + src/server/tls13.rs + src/common_state.rs :: Message construction sites / minimum custody-preserving constructor factoring :: adding backing-bound Message custody while retaining source compatibility requires initializing the new non-allocating custody field at every existing Message literal; current #425 excludes server handshake and ECH paths"
```

No semantic source from the failed preflight is retained. The earlier
ArcInner precharge, geometric growth, payload denial, and rollback repairs remain
unchanged and regression-only. P1 `3966866700` and umbrella P1 `3954831069`
remain open.

## Window31 Message backing-custody checkpoint

Protected constructor amendment `OTV2-WP3-RUSTLS-MESSAGE-CUSTODY-CONSTRUCTORS-20260909`
is now applied. `Message` carries a private, non-allocating decoded-custody token after
its payload, so payload backing is destroyed before its exact decode debit is released.
Owner-aware parsing snapshots the decoded transaction and attaches its successful debit
to the returned message; moving the message transfers the token without another debit.
All ordinary ECH, TLS1.2-server and TLS1.3-server message construction is routed through
an unowned constructor or explicitly initializes no custody.

```yaml
status: active
message_backing_custody: PROVEN_FOCUSED
constructor_compatibility_ech_server: PROVEN_COMPILE
complete_tls_accounting: NOT_PROVEN
remaining_acceptance_cells: owner-aware deep-copy destination reservation; custody split/transfer into successor, peer-chain and retained session state; transcript and complete handshake composition; TLS-positive; PostgreSQL17.6; review/CI/MQ/readback
next_action: continue #425 ownership conversion and retained-descendant custody on the Message token foundation
```

P1 `3966866700` and umbrella P1 `3954831069` remain open: this checkpoint closes
message-local drop and move custody only; it does not yet prove every deep-copy or
retained-descendant transfer in the complete TLS matrix.

## Window32 production Message custody witness

The exact-graph SQLx AWS-LC wire-handshake harness compiles rustls as a dependency,
so rustls has `cfg(test) == false` and its production-only
`Message::decoded_custody` representation is present. The completed real handshake
now requires a non-KX decoded reservation and its matching release during client
packet processing. This executes the real owner-aware `Message` parse, first-message
`into_owned` transfer, and production drop path rather than constructing
`DecodedCustody` directly in a rustls unit test. The ordinary server remains
owner-free, and the existing underfunded HRR case continues to deny before its
unauthorized reservation.

```yaml
status: active
message_backing_custody: PROVEN_PRODUCTION_FOCUSED
complete_tls_accounting: NOT_PROVEN
remaining_acceptance_cells: per-backing local list/payload release; separately charged deep ownership; retained successor/peer/session custody; transcript and complete handshake composition; TLS-positive SQLx socket; PostgreSQL17.6; review/CI/MQ/readback
next_action: replace aggregate list/payload retention with per-backing custody without connection-wide checkpoint rollback
```

P1 `3966866700` and umbrella P1 `3954831069` remain open. This production witness
validates the Message custody foundation only and does not promote complete TLS.

## Window33 decoded-custody underflow fence

The aggregate `DecodedOwner::charged` counter is retained only as accounting and
debug state while per-backing custody is introduced. Its release path now uses a
checked atomic subtraction and refuses to forward an invalid second release to
the accepted owner ledger. This prevents an intermediate mixed aggregate/backing
implementation from underflowing the counter and, more importantly, from
releasing live custody in the underlying owner. It does not make aggregate
checkpoint rollback a final lifetime authority.

```yaml
status: active
decoded_release_underflow_fence: PROVEN_COMPILE
complete_tls_accounting: NOT_PROVEN
remaining_acceptance_cells: per-backing list/payload custody and every retained-descendant/complete-handshake cell from Window32
next_action: replace aggregate list/payload retention and checkpoint rollback with explicit backing/scope custody
```

P1 `3966866700` and umbrella P1 `3954831069` remain open.

field migration and full retained-descendant matrix are independently verified.

## Window34 payload backing custody

`PayloadU8` and `PayloadU16` now retain an exact, non-allocating custody token after
their byte vector. Owner-aware decoding reserves before copying and binds the debit to
the returned backing; tuple-field drop order destroys the vector before returning its
debit. Ordinary constructors remain uncharged. Deep cloning a charged payload is
explicitly rejected until the destination-owner path can reserve separately.

Per-backing custody is excluded from the enclosing Message aggregate transaction. A
decode checkpoint now records both total and already-custodied bytes, so Message commit
and parse rollback release only still-local reservations and cannot double-release or
steal transferred backing custody.

```yaml
status: active
payload_u8_u16_backing_custody: PROVEN_FOCUSED
complete_tls_accounting: NOT_PROVEN
remaining_acceptance_cells: generic list backing wrapper; borrowed Payload ownership; explicit owner-aware deep copies; retained descendants; transcript and complete-handshake composition; TLS-positive; PostgreSQL17.6; review/CI/MQ/readback
next_action: carry the same backing-bound representation through generic decoded lists and owner-aware deep-copy destinations
```

P1 `3966866700` and umbrella P1 `3954831069` remain open because generic list
backing and the complete retained-descendant matrix are not yet proven.

## Window35 charged HRR cookie copy repair

Exact-head review found that a peer-supplied, owner-aware decoded HRR cookie reached
the ordinary `PayloadU16::clone` assertion in the retry ClientHello path. The charged
protocol path now uses an explicit fallible deep copy: it reserves the destination
capacity before allocation while source custody remains live, verifies the actual
capacity, and attaches independent custody to the destination. Owner-free `Clone`
retains upstream behavior; charged protocol flow no longer panics or creates an
uncharged destination.

Focused controls cover funded source/destination overlap, max-minus-one destination
denial, independent source and destination final release, and the owner-free clone.
The current payload clone census also found the TLS 1.3 ticket sites use `Arc` clones
rather than payload-backing deep copies; the empty handshake client-auth context copy
does not allocate backing. Aggregate decoded/list lifetime and the broader #425 matrix
remain open.

```yaml
status: active
charged_hrr_cookie_copy: PROVEN_FOCUSED
payload_u8_u16_backing_custody: PROVEN_FOCUSED
complete_tls_accounting: NOT_PROVEN
remaining_acceptance_cells: generic list backing; borrowed Payload ownership; retained descendants; transcript and complete-handshake composition; TLS-positive; PostgreSQL17.6; review/CI/MQ/readback
next_action: continue generic list backing custody and every retained descendant cell without using aggregate rollback as lifetime authority
```

P1 `3966866700` and umbrella P1 `3954831069` remain open.

## Window36 empty TLS 1.3 CertificateRequest context repair

Owner-aware zero-length decoded copies now return the allocation-free empty `Vec`
without a custody token. This matches representation truth (`Vec::with_capacity(0)`
has no backing allocation), leaves charged/custodied counters unchanged, and allows
the valid empty TLS 1.3 CertificateRequest handshake context to pass through the
ordinary owner-free `PayloadU8::clone` without panic. The existing client rejection
of non-empty handshake contexts is unchanged.

Focused production-source decoding covers a complete empty-context
CertificateRequest with a signature-algorithms extension, proves clone does not add
a debit, and proves final owner release remains exact.

```yaml
status: active
empty_certificate_request_context: PROVEN_FOCUSED
complete_tls_accounting: NOT_PROVEN
remaining_acceptance_cells: generic list backing; borrowed Payload ownership; retained descendants; transcript and complete-handshake composition; TLS-positive; PostgreSQL17.6; review/CI/MQ/readback
next_action: continue generic list backing custody and every retained descendant cell without aggregate rollback as lifetime authority
```

P1 `3966866700` and umbrella P1 `3954831069` remain open.

## Window37 generic backing-bound list representation

Added the private `DecodedVec<T>` representation required to move generic decoded-list
capacity out of connection-aggregate lifetime authority. Owner-aware decoding reserves a
checked geometric prospective capacity before allocation, verifies actual capacity, and
keeps old and replacement custody live until the old vector is destroyed. The resulting
token is declared after its vector, so local destruction returns the exact capacity debit
before the reader or connection owner drops. Moves are allocation-free; the explicit
fallible deep-copy operation reserves the destination while source custody remains live;
later-element failure drops the partial vector and returns its debit.

Focused tests prove local drop, move, separately charged clone overlap, and later-element
rollback. This checkpoint intentionally does not migrate the remaining private handshake
fields yet, so aggregate `Codec for Vec<T>` and the retained-descendant/complete-handshake
matrix remain open.

```yaml
status: active
generic_backing_bound_representation: PROVEN_FOCUSED
generic_handshake_field_migration: NOT_PROVEN
complete_tls_accounting: NOT_PROVEN
next_action: migrate private decoded handshake list fields to DecodedVec and continue payload/message/retained ownership
```

P1 `3966866700` and umbrella P1 `3954831069` remain open until the private handshake
field migration and full retained-descendant matrix are independently verified.

## Window38 generic charged-copy safety repair

Coordinator evidence `5602859284` found that the initial generic
`DecodedVec<T>::try_clone_with_resource_owner<T: Clone>` charged only the outer vector
while arbitrary element `Clone` implementations could allocate nested destination
backing. It also held the prospective outer debit as an unguarded counter until cloning
finished, which could leak the reservation during safe-Rust unwinding.

The arbitrary charged deep-copy API has been removed. Its replacement is deliberately
limited to `T: Copy`, whose element copy cannot allocate nested backing, and uses an
armed prospective RAII reservation declared before the destination vector. Thus denial
precedes destination allocation, destination backing is destroyed before unwind releases
the prospective debit, and successful construction commits the same debit into the final
backing-bound custody. Allocating element types require field-specific recursive fallible
copies during migration; ordinary owner-free `Clone` remains unchanged.

Focused tests preserve funded source+destination overlap and independent final release,
and add exact max-minus-one denial with source pointer/content/custody unchanged and no
reservation leak.

```yaml
status: active
generic_charged_copy_for_arbitrary_clone: REMOVED_UNSAFE_SURFACE
nonallocating_element_copy: PROVEN_FOCUSED
generic_handshake_field_migration: NOT_PROVEN
complete_tls_accounting: NOT_PROVEN
next_action: perform the SQLx client generic-list lifetime census and migrate or split only divergent-lifetime fields, using field-specific recursive copies for allocating elements
```

P1 `3966866700` and umbrella P1 `3954831069` remain open pending the field census,
retained-descendant composition, and final independent exact-head review.

The field-migration representation is now also source-compatible with `no_std`:
`DecodedVec<T>` aliases ordinary `Vec<T>` without custody when `std` is disabled.
The charged `std` copy restrictions and owner-free `no_std` clone/allocation behavior
therefore remain separate. This is a compatibility repair, not field-migration or
complete-TLS proof.

## Window39 SQLx-client list census and first divergent lifetime

The owner-aware SQLx client reaches the rustls client handshake only (TCP, no ECH).
The source census below is limited to generic TLS lists decoded on that graph. In every
row the source backing debit is the allocator-confirmed `Vec.capacity() * size_of::<T>()`
reserved by `Codec for Vec<T>` on the same `DecodedOwner`; zero capacity has no debit.
`Message::decoded_custody` follows the payload fields in drop order and is sufficient only
for rows whose source backing remains in that payload until message destruction.

| Phase / exact source field | Lifetime | Backing and owner | Exact terminal point |
| --- | --- | --- | --- |
| ServerHello `ServerExtensions` optional lists (`versions`, key share and ordinary extension lists) | MESSAGE_LOCAL | each decoded outer `Vec<T>` capacity on the message `DecodedOwner`; nested payload bytes have their own custody | read by `ExpectServerHello*::handle`; source AST dies with `Message` |
| HelloRetryRequest `HelloRetryRequestExtensions::order` | MESSAGE_LOCAL | `Vec<ExtensionType>` capacity on message custody | ordering checks complete; source dies with HRR `Message` |
| HelloRetryRequest `cookie: PayloadU16` | DEEP_COPY_DESTINATION | source payload capacity has backing-bound custody; retry-cookie destination is separately reserved | source dies with HRR message; charged destination moves into retry ClientHello successor |
| EncryptedExtensions extension lists | MESSAGE_LOCAL | decoded outer capacities on message custody | consumed by `ExpectEncryptedExtensions::handle`; source dies with message |
| TLS 1.3 CertificateRequest `signature_algorithms` | MESSAGE_LOCAL | decoded `Vec<SignatureScheme>` source capacity on message custody | source dies with CertificateRequest message |
| TLS 1.3 CertificateRequest compatible-signature subset | EARLY_DROP / DEEP_COPY_DESTINATION | new exact-capacity `DecodedVec<SignatureScheme>` destination, separately reserved from the same owner | passed by reference to client-auth resolution, then backing dies and releases before successor/message drop |
| TLS 1.3 CertificateRequest authority names and compression algorithms | MESSAGE_LOCAL | outer generic list capacities on message custody; name payloads use decoded payload custody | borrowed during resolver/compressor selection, then die with message |
| TLS 1.2 CertificateRequest `certtypes`, `sigschemes`, `canames` | MESSAGE_LOCAL with later destination copies still open | outer capacities on message custody | consulted by TLS 1.2 client-auth resolution; source dies with message |
| TLS 1.2/TLS 1.3 Certificate outer entry/chain lists | DEEP_COPY_DESTINATION then MOVE_TO_PEER_CHAIN_SESSION | source outer capacity on message custody; owned certificate bytes and retained destination capacities require independent pre-reservation | source message dies after verification; owned destination moves through verifier state into `CommonState::peer_certificates` and session state |
| NewSessionTicket extension/metadata lists | MESSAGE_LOCAL with retained ticket copy open | decoded list capacities on message custody | source dies after ticket processing; separately charged ticket/session destination must survive cache retention |

The first new divergent production path is the TLS 1.3 CertificateRequest compatible
signature list. It no longer uses untracked `collect::<Vec<_>>()`: the message exposes its
same owner to a `T: Copy` filtered-copy operation, reserves the exact destination capacity
before allocation, and binds the debit to `DecodedVec`. The source message remains charged
during construction; the temporary destination releases immediately after client-auth
resolution, before the successor state and connection are dropped. A max-minus-one test
denies before destination allocation and an exact funded test proves early release.

This does not close the certificate/peer/session rows, borrowed Payload conversion,
transcript contexts, or complete-handshake composition.

```yaml
status: active
sqlx_client_generic_list_lifetime_census: PROVEN_SOURCE_DERIVED
certificate_request_compatible_signature_destination: PROVEN_FOCUSED
generic_handshake_field_migration: IN_PROGRESS
complete_tls_accounting: NOT_PROVEN
next_action: continue certificate/peer/session destination custody and owner-aware Message/Payload ownership conversion
```

## Window40 retained ticket custody

The first concrete `MOVE_TO_PEER_CHAIN_SESSION` case is now implemented for TLS session
tickets. `PayloadU16<NonEmpty>` cardinality retyping moves both its vector and existing
decoded custody without allocating or reserving again. TLS 1.2 and TLS 1.3 ticket decode
reserve the exact padded Rust 1.94 `ArcInner<PayloadU16>` layout before `Arc::new`.
A private `TicketPayload` carries that control-block custody through allocation-free Arc
clones and releases it only after the final Arc has deallocated its payload/control block.
The normal TLS 1.3 session path therefore retains both ticket bytes and Arc-control custody
after the source Message is destroyed, without a second debit.

Focused evidence covers funded decode, allocation-free clone/retention, source wrapper drop,
and exact final release. Ordinary owner-free TLS 1.2/TLS 1.3 constructors remain unchanged.
Secret-vector, peer-chain Arc, transcript, certificate/OCSP, compressed-certificate, and
complete-handshake composition remain open.

```yaml
status: active
retained_ticket_backing_and_arc_control: PROVEN_FOCUSED
retained_session_secret_and_peer_chain: NOT_PROVEN
complete_tls_accounting: NOT_PROVEN
next_action: extend the same retained-session model to secret and peer-chain backing, then continue certificate/transcript ownership
```

## Window41 retained secret and peer-chain custody

Retained TLS 1.2 and TLS 1.3 session construction now derives the already-installed owner
from the charged `TicketPayload`. Before copying a PSK/master secret it reserves the exact
destination byte capacity; the source remains live through construction, and the final
`Zeroizing<PayloadU8>` is declared before its custody so zeroization and backing destruction
precede release. The normal TLS 1.3 path no longer clones `CommonState::peer_certificates`
before admission.

The retained certificate destination is an intentional deep copy. It precharges the exact
outer `Vec<CertificateDer>` backing and each DER byte destination before the respective
allocation, then precharges the padded Rust 1.94 `ArcInner<CertificateChain<'static>>`
layout before `Arc::new`. A private wrapper encapsulates every strong Arc handle in the
retained-session graph, shares one control/backing debit across allocation-free clones, and
releases only after final Arc destruction. Thus the connection-owned source chain and the
retained destination coexist under separate custody, and session/cache clones neither
allocate nor double-charge the retained chain.

Focused tests cover funded deep copy, source/destination coexistence, allocation-free Arc
clone, source drop while retained custody survives, max-minus-one denial at the Arc-control
boundary, and exact final release. Complete certificate-message transfer, transcript/hash
contexts, compressed certificates, and complete-handshake composition remain open.

```yaml
status: active
retained_ticket_backing_and_arc_control: PROVEN_FOCUSED
retained_session_secret_and_peer_chain: PROVEN_FOCUSED
complete_tls_accounting: NOT_PROVEN
next_action: continue certificate/OCSP ownership and transcript context custody, then compose complete TLS
```

## Window42 TLS 1.3 certificate destination custody

The normal TLS 1.3 certificate path now obtains the decoded owner before moving the
Message payload.  It computes and reserves the complete destination outer-chain, DER and
OCSP backing before the first destination allocation, then builds the owned chain under
armed prospective custody.  The source Message remains independently charged throughout
the conversion.  Successful custody moves through `ServerCertDetails` into
`CommonState::peer_certificates` without a second reservation; field ordering destroys
the chain and OCSP backing before releasing their combined destination debit.

This is focused production-path progress only.  TLS 1.2 certificate ownership, compressed
certificate decompression/second decode, transcript/hash contexts, and the complete
simultaneous-live handshake composition remain open.

```yaml
status: active
tls13_certificate_destination_custody: PROVEN_FOCUSED
complete_tls_accounting: NOT_PROVEN
next_action: complete equivalent TLS1.2 and compressed-certificate custody, then transcript/hash contexts
```
