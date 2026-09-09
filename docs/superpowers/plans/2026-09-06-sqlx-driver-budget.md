# SQLx PostgreSQL driver budget351 implementation plan

## TLS certificate destination checkpoint

- [x] Reserve the TLS 1.3 owned certificate-chain outer backing, DER copies, and OCSP copy
  before the first destination allocation and transfer custody into `CommonState`.
- [ ] Apply equivalent reachable TLS 1.2 custody and compressed-certificate second-decode
  accounting.
- [ ] Account transcript vectors and qualified AWS-LC hash contexts, then produce the
  complete-handshake composition witness.

> **2026-09-07 continuation checkpoint:** TLS activation is BLOCKED at the
> excluded blocking-runtime task allocation owner. The admitted SQLx paths cannot
> pre-charge Tokio's private generic `Cell<T, S>` allocation or keep that charge
> through Cell deallocation/cancellation. A Cell-only amendment is insufficient:
> operation-attributable blocking-pool queue/map backing and worker/thread packets
> can survive the Cell through idle retention or shutdown. The protected amendment
> must cover both owner lifetimes, or provide an equivalent registered
> loading/runtime owner when shared growth cannot be attributed per operation;
> alternate runtime branches compound the gap. Exact evidence and the minimal amendment are in sqlx-core
> `OTERYN_PROVENANCE.md`. Stop before PostgreSQL decoder work or test inclusion.
> Issue351 comment5560895137 activated custody of the include-only test path, but
> the inclusion remains unperformed because its TLS prerequisite did not close.

## Current prospective hosted-test qualification amendment

Work162 comment5560691505, protected source `b61f9d8cc1c0a7289ffdaf1bf4e42b851d2c0f9a`, adds only the serialized target inclusion below to351's existing admitted scope. Earlier NOT_ADMITTED wording is historical; actual admission53c6 and window2/native1363 are recorded in351 comment5560554622 and the current task override. Preserve original programme budget, immutable admission and all productive-window/repair evidence. This new lease remains NOT_ACTIVE until reviewed protected integration/readback and Work's explicit grant.

### Exact prospective transfer

After the activation gate, temporarily remove `apps/game-server/tests/durability_postgres.rs` from B329's active write scope and lease it exclusively to the sole351 writer **only** to add this module inclusion:

```rust
#[path = "../../../vendor/sqlx-postgres-0.9.0/tests/oteryn_resource_budget.rs"]
mod oteryn_resource_budget;
```

The included `vendor/sqlx-postgres-0.9.0/tests/oteryn_resource_budget.rs` stays within351's existing vendor subtree. No other change to the shared target is authorized: preserve every existing B test, import, fixture, gate and assertion; no reformatting or test suppression. No workflow, Cargo feature/dependency, production B, Foundation or source scope is added. Driver retains its separately protected two-crate Cargo lease and exclusions.

B keeps every other owned path and its canonical branch/worktree. Work verifies exact overlap before granting the lease and before integration. While active, B must not write the shared target or integrate overlapping target changes;351 may not use the lease for any additional edits. Work serializes ordinary merge-up and reviews the resulting exact delta, retaining prior B material. Return the target to B only after protected351 delivery/integration/readback and Work's explicit release/readmission for this file; no concurrent writer or automatic lease inheritance. Earlier14-path B lists remain historical during the active transfer.

The existing canonical PostgreSQL17.6 target must actually execute the included tests on the pinned root dependency graph. Vendor-only test results and successful compilation do not establish hosted SQL execution. Keep all existing workflows and tests intact. If the service is plaintext, it supplies no TLS-positive evidence:351 must separately qualify actual TLS without security/feature downgrade or treating skipped/unconfigured tests as success. This amendment alone proves no TLS/driver/B acceptance and does not release Server Seam247.

After activation, author the driver-owned vendor tests and add only the named inclusion. Run focused vendor tests and the unchanged canonical root PostgreSQL target on the exact root dependency graph, with independent positives/negatives and visible execution evidence. Keep TLS-positive qualification separate where required. Do not claim root CI automatically runs excluded vendor tests without this inclusion. At protected delivery/readback, Work releases the target back to B; driver/B activation remains its separate gate.

Next action: independently qualify and protect the five-document lease amendment before any shared-target edit.

## Authority, exact scope and sequencing

Issue351 under Work162 implements accepted DUR-FRESH-RESOURCE-ENVELOPE-V1 (#337/#341) and protected registry342. This is the single task plan; no duplicate design approval is required. Source main: d9d1b566acb57b537ff901d9765c32a95110c259. Worker NOT_ADMITTED until independently reviewed allocation is protected and Work binds readback, exclusive Cargo lease and immutable admission.

Prospective paths: `vendor/sqlx-postgres-0.9.0/**`, `vendor/sqlx-core-0.9.0/**`, root `Cargo.toml`, root `Cargo.lock`, `docs/agents/tasks/active/OTV2-20260906-sqlx-driver-budget-351.md` and this plan. Cargo.toml permits only the exact sqlx-postgres/sqlx-core path patches and necessary workspace exclusions; Cargo.lock permits only their consequences. No B/SQL/Foundation/registry/workflow, rustls or other dependency changes. The exact sqlx-core0.9.0 import preserves upstream bytes/licenses except these permitted accounting paths: `src/net/tls/mod.rs`, `src/net/tls/tls_rustls.rs`, `src/net/mod.rs`, new `src/net/resource_budget.rs`, new `src/net/tls/resource_budget_tests.rs`, and `OTERYN_PROVENANCE.md`. Paths are relative to `vendor/sqlx-core-0.9.0/`; its other imported files remain byte-identical upstream. PostgreSQL `src/connection/tls.rs` permits accounting-only budget plumbing. Preserve TLS modes, protocol versions, certificate/hostname verification, selected features and unrelated behavior. Rustls and every other dependency remain excluded; another concrete dependency need requires a protected amendment. Report concrete additional dependency paths for a protected amendment; inability to prove coverage keeps acceptance closed.

One writer, one isolated branch/worktree, proposed `agent/sqlx-driver-budget-351`. Parallel B/338/346 work retains existing custody. Serialize Cargo against247 and Dependabot259/260/261; Work verifies current overlap before admission and integration. Only the explicit root Cargo patch scope transfers from247; all its other scope/history stays held. New driver capability is integrated/protected before the existing B writer activates it on B's owned paths. No source/bootstrap or Server Seam release follows.

## 1. Reproduce and inventory before patching

Verify upstream sqlx-postgres0.9.0 crate checksum `87a2bdd6e83f6b3ea525ca9fee568030508b58355a43d0b2c1674d5f79dcd65e`, VCS `003b698e99e024f3621b8043a2426fde5b741171`, subdirectory `sqlx-postgres`. Preserve upstream manifests and licenses; record exact provenance, import method, file/delta manifest and verification commands in `vendor/sqlx-postgres-0.9.0/OTERYN_PROVENANCE.md`. Verify core checksum `05b44e85bf579a8eeb4ceaa77a3a523baf2bf0e9bac7e40f405d537b5d2d5ccb`, same VCS, subdirectory `sqlx-core`; preserve its exact import/delta manifest and licenses in its own OTERYN_PROVENANCE.md. The two root patches must keep one compatible SQLx type universe and preserve selected features/version. Review all upstream/modified code relevant to the bounded path, not merely the diff.

Reproduce five-byte announced-length reserve, DataRow/RowDescription peer-count reserve and cumulative ParameterStatus/type/table retention. Inventory every controlled allocation across connect/authentication/TLS, read/write, encode/decode, arguments/results, metadata/errors/logging, caches and disposal. Identify allocation capacity/growth, temporary overlap, Arc/Bytes backing ownership and all cross-operation/idle retention. Inspect and amend only the explicitly listed SQLx-core accounting paths to prove bounds; inspect rustls read-only and identify exact additional scope if needed; do not assume an initial8192-byte buffer, disabled statement cache, timeout or shrink_buffers proves them.

## 2. Implement accepted charging without a new policy

Expose a driver reservation capability that the existing B writer can bind to the same executor/slot ledger. No driver constructor may invent an independent budget for B work. Derive allocation admissibility from remaining charged capacity with checked arithmetic and proven allocation overhead. Reserve before buffer/vector/string/cache growth; account capacity rather than length, and release only when backing ownership actually ends or transfers under a held charge. A fixed reservation is permitted only if its finite measured/proved bound is inside the existing budgets; allocating the whole4MiB slot to a fictional driver allowance is forbidden.

The first implementation checkpoint is the complete TLS capacity/lifetime proof: charge bounded immutable configuration, PEM/DER/key expansion, crypto/configuration objects, handshake receive/send buffers, peer certificate chain, cached ticket-chain copies and the incoming pre-eviction copy, cancellation and idle retention. The untouched-TLS reservation is NOT_PROVEN, not impossible. Use the specifically allocated core budget/TLS plumbing without changing TLS policy; do not begin substantial PostgreSQL decoder work until this proof is reviewable.

A private bounded PostgreSQL buffer using public SQLx-core Socket is a candidate, not a guaranteed complete solution. Validate minimum frame lengths and complete count/body structure before allocation; handle every backend message family, including errors/notices/status and authentication. Bound send encoding as well as receive growth. Track shared backing once only with demonstrable retained ownership; deep clones and growth overlap count separately. Bound status/type/table cache growth through the same ledger, preserving SQL semantics. Retained pooled/idle connection allocations cannot disappear from accounting at task completion.

Accepted conjunctive ceilings remain unchanged: queue8 entries/524288 bytes each/4194304 total; active2 slots/4194304 bytes each/8388608 total; total12582912 charged resident bytes in one executor; operation65536, guard8192, durable row131072, SQL32 payload rows/524288 aggregate logical bytes. All other registry dimensions,32 variable columns,64 pending commands/256 domain revisions and original custody/deadline rules remain binding. Logical SQL row/result bytes are not interchangeable with driver wire/capacity charges. No hidden lower semantic limit, truncation, omitted provenance or count/hash shortcut is allowed.

Resource denial yields bounded unavailable behavior; after possibly effectful submission, preserve ambiguity and original operation custody. No detached backend/task, extra executor, freed permit while work remains, transport downgrade or TLS-disable workaround. Reserve completion/error capacity before irreversible COMMIT. The driver does not itself grant owning authorization or clear B checkpoints.

## 3. Qualification and scope gate

Use independent positive controls and one changed invariant per negative. Demonstrate denial before size-controlled allocation for hostile header lengths, overflow/minimum lengths, huge count/tiny body, malformed row ranges, repeated status names/values and cache growth. Exercise receive/send growth, bounded error formatting, retained rows/Bytes clones, partial messages, cancellation at each phase, repeated operations, idle pooling and close/drop. Compare measured peak capacities to the complete ownership ledger; a post-allocation counter check is insufficient.

Prove compatible actual configured PostgreSQL17.6 behavior at accepted maxima and first representable values above, cumulative rows/bytes and simultaneous active slots. Include full64-command/256-revision records, status/error traffic, original-operation reconciliation and cancellation/ambiguity compatibility through already-authorized B qualification after driver integration. Driver-only fixtures are not proof of production B routing; report the later B activation boundary explicitly. Run existing affected workspace tests unchanged as well as vendored tests; vendor exclusion from workspace is not permission to skip its tests. Keep all normal TLS features and prove the dependency path's capacity coverage. If proof needs excluded code, stop only that implementation boundary and return exact path evidence to Work.

Run fmt, strict applicable Clippy, dependency/lock/provenance review, governance and complete author self-review. Require genuinely independent full-change review of parser/resource and supply-chain changes, exact-head canonical CI, normal Merge Queue and protected readback. No suppressed/ignored test, workflow weakening or skipped PostgreSQL result counts as passing. Preserve all failures/repairs and bounded-window history.

## 4. Integration and material escalation boundary

Work serializes root Cargo overlap, normal merge-up and qualification of the resulting exact head, then protected integration/readback and explicit lease release. The B writer subsequently binds this capability to B's actual slot ledger and proves all backend paths under its own allocation. Driver completion alone does not prove B resources or Server Seam readiness.

Missing implementation or insufficient path scope is not a material architecture question. Escalate through Work to the owner-designated architect only with evidence that accepted envelope, semantic guarantees or supported transport policy must change; do not choose a new numeric threshold or policy locally. Historical allocation next action was protection before admission. Work admitted branch `agent/sqlx-driver-budget-351` at `53c6bdf06a2282d893035a995c46052c88f935b4` under comment5560220858. Window1 checkpoint implements exact imports and an owner-supplied, checked reservation/transfer/drop primitive with nine new tests. Complete TLS proof remains OPEN; no TLS guard or substantial PostgreSQL decoder has been activated. Next action is independent review and completion of the source-derived phased TLS reservation proof, preserving all accepted maxima. Provenance records certificate capacity growth, three-copy OCSP overlap, incoming ninth ticket-chain clone, ECH unexpected-message decoding, fragment-span high-water and generic ring parsing bounds. Passing primitive tests and 340 Game library tests are not TLS or PostgreSQL qualification.

## Window2 implementation checkpoint

Continuation5560554622 begins at native1363c9b, preserving immutable53c6 admission and prior counters. The full pre-state decoder grammar is source-bounded at3276650 requested heap bytes; input/span/configuration/crypto/retained/error owners remain separate. Checked phase math and private unverified-chain input tracking are implemented as unactivated primitives. Bounded Reader/File data ownership is tested through EOF and pre-growth denial, but scheduling through the shared Tokio blocking pool is not qualified: queue/map/thread growth and Cell lifetime after result drop need a registered bounded loading owner or exact accounting hooks. No guessed scheduler charge, silent File rejection or unbounded read is acceptable.

A proven finite config-parameterized peak may remain charged until connection/config/state destruction. Actual remaining owner balance determines admission; independent maxima need not simultaneously attain every maximum. No complete TLS adapter or PostgreSQL decoder implementation starts from these partial component results. Next concrete gate is complete phase/loader ownership proof and an exercised owner-funded positive case, followed by protected test-lease admission and actual configured PostgreSQL/TLS qualification.

Window2 stopped at 2026-09-06T17:16:20Z: 3050 productive seconds conservatively charged, 550 unused seconds, zero deducted pauses and zero counter reset. Completed windows:2; repair cycles:2; rotations:0; identical-failure retries:0. Further implementation requires explicit Work continuation. Full TLS and PostgreSQL adapter gates remain OPEN; prospective PR357 remains NOT_ACTIVE.

## 2026-09-07 protected blocking-owner amendment result

Fresh RED commit `f0dddec27ed8151f73d3db80750a89ccf15f3e77` failed at the
intended missing sealed pre-spawn owner/API after reaching SQLx-core compilation.
The configured graph enables Tokio 1.53.1 only. SQLx can wrap closure/result data,
but Tokio exposes no public fallible hook to reserve and retain the private task,
blocking queue/map, and worker/thread backing before allocation through actual
idle/shutdown release. Do not implement a false GREEN in SQLx or use a fake as
backend proof. Status is `BLOCKED_RUNTIME_BACKEND_OWNER`; request the exact Tokio
hook or an already-funded registered runtime owner described in core provenance.
TLS composition, PostgreSQL decoder work and shared-target inclusion remain OPEN.

## 2026-09-07 protected Tokio amendment stop

Application `5575674784` permitted the exact Tokio 1.53.1 package and listed
owner implementation surfaces. The corrected amended RED at
`45da01b13b848785ad7fe068c6e100b7cc3eebe5` reaches Tokio compilation and fails
for the absent owned blocking API. Do not begin GREEN: `src/task/blocking.rs` is
private and the required `src/task/mod.rs` public re-export list is outside the
protected authored allowlist. Request only that exact re-export lease. Preserve
ordinary spawn behavior and all remaining task/queue/worker, TLS and PostgreSQL
work as OPEN; `TLS_BLOCKING_OWNER = NOT_PROVEN`.

## 2026-09-07 protected task-export continuation

The protected `src/task/mod.rs` lease is active and the prior amended RED remains
preserved. An intermediate owned-Tokio GREEN now reserves concrete task Cell,
separate finite owner-queue and explicit-stack worker backing and passes focused
denial/overflow/funded/drop/idle/shutdown controls. Do not advance to PostgreSQL:
queue-full pre-admission, queued abort, OS-spawn failure, loom concurrency, SQLx
ledger adaptation, complete TLS phase composition and real TLS-positive evidence
remain OPEN. `TLS_BLOCKING_OWNER = NOT_PROVEN` and WP3 is not accepted.

## Window6 checkpoint

- [x] Deny a full owned queue before task/Cell reservation or allocation and prove no ordinary-queue spill.
- [x] Retain queued-cancellation custody until queue removal and actual task destruction.
- [x] Deterministically force OS-worker spawn failure and prove exact rollback/release without fallback.
- [x] Provide deterministic multithreaded accounting-race evidence in the allocated test surface; record why the pinned private path is unreachable from the existing Loom registration without an unauthorized source/feature change.
- [ ] Adapt SQLx `BlockingJobOwner` and certificate loading, failing closed for non-Tokio backends.
- [ ] Complete TLS composition and real TLS-positive proof before PostgreSQL17.6 qualification.
- [x] Preserve one `BlockingJobOwner` identity across sequential certificate/key
  loads and prove the owner queue never mistakes the same ledger operation for a
  different owner.
## Window6 SQLx adapter successor

SQLx now binds its existing ResourceBudget to the enabled owned-Tokio API and the
accounted certificate-file loader has funded/denied custody tests with no
fallback. Treat `TLS_BLOCKING_OWNER = PROVEN` narrowly: complete TLS phase,
configuration/session/cache lifetime composition remains the next gate. Do not
activate the shared PostgreSQL target or claim WP3 accepted until complete TLS
and separate real TLS-positive evidence are proven.

## Window7 complete-TLS stop

The owned Tokio/certificate-loader prerequisite stays proven.  The next complete
TLS step is blocked before PostgreSQL work: rustls 0.23.43 private
`DeframerVecBuffer::prepare_read` resizes/shrinks its incoming backing before the
SQLx reader is called, with no public actual-capacity or pre-growth custody hook.
Request the exact prospective
`vendor/rustls-0.23.43/src/msgs/deframer/buffers.rs` owner hook and necessary
public wiring.  It must reserve actual requested backing and old/new overlap from
the same ledger before mutation and retain custody through actual free/transfer.
Do not copy a private capacity schedule, reserve opaque magic bytes, weaken TLS,
or activate the include-only PostgreSQL target.  Configuration/decoder/session
cache/handshake overlap, TLS-positive evidence and PostgreSQL17.6 remain OPEN.

## Window8 protected rustls deframer hook

- [x] Import the checksum-verified complete rustls 0.23.43 published package and preserve all bytes outside the protected authored allowlist.
- [x] Reserve exact prospective deframer capacity before allocation/reader invocation and retain old/new overlap until old backing destruction.
- [x] Prove denial preservation, read-error/WouldBlock retention, drop release, same-ledger SQLx adaptation, unhooked behavior and no-std compilation.
- [ ] Complete configuration, decoder, session/cache and handshake-overlap custody on the same ledger.
- [ ] Run separate actual TLS-positive and configured PostgreSQL17.6 qualification only after complete TLS ownership is proven.
- [ ] Obtain independent exact-head high-risk review, canonical CI/MQ and protected readback before any shared-target release or WP3 acceptance claim.

## Window9 decoded-state stop

- [x] Re-audit phase composition after the deframer owner hook and preserve the
  narrow blocking/deframer proofs.
- [x] Identify the next pre-observation allocation at rustls
  `ConnectionCore::deframe` before `Message::try_from` / `into_owned`.
- [x] Prove existing conservative values leave only 150 bytes in the accepted
  4 MiB slot before nonzero configuration/crypto/session/send/error owners, so
  an opaque pre-call reservation cannot establish the required positive case.
- [ ] Obtain a protected same-ledger decoded-message owner amendment at that
  exact call site, including charged transfer into retained peer-chain/session
  backing.  The current #424 `conn.rs` grant is deframer-wiring-only.
- [ ] Keep TLS-positive and configured PostgreSQL17.6 execution stopped until
  complete TLS ownership is proven; plaintext and CONTROL classifier results
  are not qualification.

## Window10 protected decoded-owner preflight

- [x] Normally merge protected `main@dfc0fd3a9148cb85b7c75e7cad3b15a1fe70d2eb`
  and preserve every prior RED/GREEN checkpoint.
- [x] Read the protected decoded-owner amendment and preflight its exact thirteen
  path/symbol boundaries before decoded-state mutation.
- [x] Stop at exact `SHARED_LEASE_REQUIRED` for
  `vendor/rustls-0.23.43/src/client/hs.rs::emit_client_hello_for_retry`: the
  required ClientHello proof includes its extension box, collected vectors,
  payload clones and owned CA names, but the amendment grants other symbols in
  that file only.
- [ ] After a protected symbol amendment, resume boundary-by-boundary decoded
  custody RED/GREEN without weakening TLS or substituting an aggregate reserve.
- [ ] Keep real TLS-positive and PostgreSQL17.6 qualification stopped until the
  complete TLS ownership matrix is proven.

## Window11 ClientHello symbol-amendment preflight

- [x] Normally merge protected
  `main@e3d8a46871a98a309c73b3febaa41a7e6d2ec408`, retaining the separate
  CONTROL classifier changes without authoring their paths.
- [x] Preflight the protected `emit_client_hello_for_retry` amendment before
  semantic source mutation.
- [x] Stop at exact `SHARED_LEASE_REQUIRED` for
  `vendor/rustls-0.23.43/src/client/hs.rs::ClientHelloInput::new`: it clones
  configured protocol/ALPN backing into `ClientHelloDetails` before the newly
  authorized function is entered, so that function cannot reserve before the
  allocation or retain source/destination overlap.
- [ ] Obtain the narrow constructor-symbol lease and then resume the protected
  decoded-owner RED/GREEN matrix without moving the allocation merely to evade
  its custody boundary.
- [ ] Keep complete TLS, real TLS-positive and exact-candidate PostgreSQL 17.6
  qualification OPEN; #422's control harness supplies no #356 credit.
## Window11 protected ClientHello symbol preflight

- [x] Normally merge protected `main@a2ba218f94e83b36443afcdbd6ec8b748a677efe`
  and read the protected ClientHello symbol amendment.
- [x] Trace construction order before mutating `emit_client_hello_for_retry`:
  SQLx receives the `ClientConnection` only after `new`/`new_with_alpn` and
  `ConnectionCore::for_client` have already emitted the initial ClientHello.
- [x] Stop at exact `SHARED_LEASE_REQUIRED` for
  `vendor/rustls-0.23.43/src/client/client_conn.rs::ClientConnection::{new,new_with_alpn}`
  and `ConnectionCore::for_client`, limited to installing the existing owner
  before `ClientHelloInput::new`/`start_handshake` while preserving ordinary
  constructors.
- [ ] Do not use global/thread-local custody, post-allocation charging, semantic
  deferral, or an opaque aggregate reservation to bypass the missing owner.
- [ ] Resume the ClientHello and decoded-owner RED/GREEN matrix only after that
  exact protected constructor-wiring amendment; TLS-positive and PostgreSQL17.6
  remain stopped until complete TLS ownership is proven.

## Window12 protected constructor-owner application

- [x] Normally merge protected `main@7508a72705ab6cba95a33dd59571eca3e94b91cb`
  containing #429 and read the exact protected amendment.
- [x] Preflight the production SQLx call chain before introducing the rustls API:
  `TlsConfig` and `sqlx-postgres::connection::tls::maybe_upgrade` currently
  carry no accepted `ResourceBudget`, so `tls_rustls::handshake` cannot pass the
  required same owner identity.
- [x] Stop before source mutation at the exact SQLx owner-propagation lease;
  do not create a ledger, global/thread-local registry, or unowned fallback.
- [ ] After exact owner propagation is allocated, prove the #429 ALPN seam RED/GREEN,
  then resume only the already-protected ClientHello/decoded boundaries.

## Window13 #430 prerequisite repair

- [x] Normally merge protected `main@ae103eb6538f3044659aba3e3af8efbe8014707c`
  containing the applied #430 operation-owner amendment.
- [x] Fix accepted P1 `3947483202`: isolate and charge owner queue nodes per
  operation identity so distinct operations can share one Tokio runtime without
  cross-charge, first-owner lock-in, or ordinary-queue fallback.
- [x] Preserve queue-full ordering, cancellation, worker failure, idle/shutdown
  custody, overflow and ordinary unowned controls; prove a distinct-owner case.
- [ ] Implement and prove the #430 caller-supplied Arc propagation. Do not start
  #429 ALPN work until that propagation is GREEN.

## Operation-owner propagation checkpoint

- [x] Preserve the prior missing-call-chain checkpoint as RED evidence.
- [x] Add an explicit owner-aware PostgreSQL establish/stream/SSL selection path.
- [x] Dispatch owner-aware TLS only to rustls and fail closed without fallback.
- [x] Retain the caller's exact budget Arc for the connection lifetime without changing options or pools.
- [ ] Apply protected #429 ALPN/protocol reservation and custody using that propagated owner.
- [ ] Continue #427/#425 only after ALPN is GREEN; stop before unallocated session/key-share/ECH/config/crypto paths.

## Propagation review repair / protected ALPN checkpoint

- [x] Fix review P1 `3956941303` with an owner-aware rustls preconstruction path; no post-construction owner installation remains.
- [x] Fix P1 `3956941320` by composing the same-budget blocking loader for ordinary-equivalent inline/file root, client-certificate and key inputs.
- [x] Fix P1 `3956941331` by removing the unauthorized public connection owner accessor while retaining private stream identity.
- [x] Fix P1 `3956941337` with driver-level establish/stream/SSLRequest tests covering exact Arc identity, caller-drop retention, S/N policy, terminal TLS denial, and the ordinary owner-free control.
- [x] Complete the protected #429 ALPN preconstruction custody seam.
- [ ] Stop at `vendor/rustls-0.23.43/src/client/hs.rs::ClientSessionValue::retrieve`; session-cache custody is explicitly unallocated and precedes continuation through the protected ClientHello emit/decode paths.

## Protected session-retrieval checkpoint

- [x] Preserve TLS1.3 destructive ticket retrieval as a move of opaque custody.
- [x] Add a fail-closed owner-aware TLS1.2 store dispatch and enter the memory cache before its destination clone.
- [x] Reuse the #425 retained-session owner-aware clone so source and destination charges overlap and destination custody releases after backing destruction.
- [x] Prove funded/denied cache retrieval and unsupported custom-store behavior while preserving ordinary retrieval.
- [x] Keep the SQLx owner-aware construction path explicitly TCP-only; do not claim QUIC parameter accounting.
- [ ] Stop before `client::tls13::initial_key_share` / `SupportedKxGroup::start`; key-exchange and crypto allocation remain separately unallocated.
- [ ] Retain complete TLS, TLS-positive, PostgreSQL 17.6, independent review and protected integration as OPEN.

## Client-PEM clone review repair

- [x] Fix review P1 `3957222192` without entering key-exchange scope: parse the
  configured client certificate and key directly from their charged loader
  backings instead of cloning either PEM vector.
- [x] Prove funded certificate/key parsing, zero duplicate PEM backing, denial
  before a second charged input can be created, parse-error custody, final
  release, and ordinary owner-free configured-client behavior.
- [ ] Stop at the already-published `client::tls13::initial_key_share` /
  `SupportedKxGroup::start` shared-lease boundary.

## Window18 post-KX configuration preflight

- [x] Normally merge protected
  `main@b26395edff3dde1ebcc155ab70758520d780884c` without rebase, reset, or
  force-push.
- [x] Preserve the #451 KX/provider-resident checkpoint history and inspect
  the next allocation before claiming complete TLS composition.
- [x] Stop before mutation at
  `vendor/rustls-0.23.43/src/crypto/aws_lc_rs/mod.rs::{default_provider,default_kx_groups}`:
  the exact production path allocates cipher-suite and KX-group vectors before
  SQLx can reserve their actual capacities, and #451 explicitly keeps this file
  read-only.
- [ ] Obtain a narrow provider-configuration owner amendment carrying the same
  ledger and custody through final provider/config destruction; do not use
  post-allocation catch-up or duplicate rustls's private slice/layout semantics.
- [ ] Keep decoded/ClientHello/session/cache composition, actual funded TLS,
  PostgreSQL 17.6, review, CI/MQ, and protected readback open.

## Window19 KX lifecycle repair

- [x] Remove the uncharged second owner-wrapper heap allocation without changing
  any protected KX bound.
- [x] Retain the full KX debit through whole and hybrid-component returned-secret
  consumption, releasing only after synchronous key-schedule consumption.
- [x] Cover exact/max-minus-one admission, returned-secret custody, failure/drop,
  simulated HRR overlap, hybrid component, and ordinary owner-free behavior.
- [ ] Re-run the remaining #451 concurrent provider/thread-churn, actual HRR wire,
  and cancellation matrix before restoring an aggregate KX `PROVEN` verdict.
- [ ] Stop at the unchanged unleased provider-configuration `Vec` owner boundary.

## Window20 canonical Linux layout reconciliation

- [x] Preserve the private layout drift assertion and reconcile the canonical
  graph's distinct ring (208-byte) and AWS-LC (200-byte) `KeyExchange` shapes.
- [x] Keep owner-aware admission restricted to the reviewed AWS-LC shape and
  retain exact/max-minus-one coverage for `554/1625/1705/6264/7881`.
- [x] Keep aggregate KX `NOT_PROVEN`; the remaining #451 lifecycle matrix and
  provider-configuration owner boundary are unaffected by this correction.
- [ ] Stop at the unchanged unleased
  `crypto/aws_lc_rs/mod.rs::{default_provider,default_kx_groups}` boundary.

## Window21 provider configuration

- [x] Merge protected `main@0c69d04a49778e539515fb6848b0ab89268c1fa9` normally.
- [x] Replace the uncharged per-connection AWS-LC provider configuration with one exact-layout, same-root, process-shared owner-aware provider.
- [x] Remove process retention of the caller's owner-wrapper Arc and precharge the remaining per-connection owner Arc allocation.
- [x] Preserve ordinary provider construction and the protected AWS-LC KX bounds/order.
- [ ] Reproduce actual HRR, provider first-use/thread-churn, cancellation, and the remaining decoded/config/session/cache/send/transcript/error matrix before aggregate WP3 GREEN.

## Window22 provider proof repair and PQ profile stop

- [x] Remove avoidable provider-validation `Vec` allocations without enlarging
  #453's protected configuration reservation.
- [x] Move the required racing-first-use proof into the executable SQLx AWS-LC
  harness and distinguish one process debit, per-thread 1,360-byte debits, and
  exactly one provider-configuration debit.
- [x] Prove from the exact SQLx AWS feature closure that
  `rustls/prefer-post-quantum` is absent.
- [x] Correct `provider_configuration_owner` to `NOT_PROVEN`; source-equivalence
  under a hybrid-last profile is not the protected PQ-first proof.
- [ ] Stop before Cargo mutation or manual group reordering at:
  `SHARED_LEASE_REQUIRED = vendor/sqlx-core-0.9.0/Cargo.toml :: _tls-rustls-aws-lc-rs / rustls prefer-post-quantum feature :: protected #451/#453 require the qualified ordinary AWS-LC default provider to be PQ-first, while the exact SQLx AWS profile disables rustls defaults and currently does not enable prefer-post-quantum`.

## Window23 protected PQ-first profile

- [x] Merge protected `main@4f1ce7b4c3092a79ffa42e0b63e786015dedea53` normally.
- [x] Add only `rustls/prefer-post-quantum` to `_tls-rustls-aws-lc-rs` and preserve the root lockfile byte-for-byte.
- [x] Prove the direct feature edge and explicitly assert the ordinary and owner-aware four-group PQ-first sequence.
- [x] Re-run allocation-free validation and the executable racing-first-use accounting proof on the final feature graph.
- [ ] Complete actual HRR, thread-churn, cancellation, complete TLS composition, funded TLS-positive, and PostgreSQL 17.6 qualification before any aggregate WP3 acceptance claim.

## Window24a actual HRR wire proof

- [x] Drive a real owner-aware PQ-first client against an ordinary P-256-only
  TLS 1.3 server and require `FullWithHelloRetryRequest`.
- [x] Observe the same-ledger peak containing the initial 7,881-byte hybrid KX
  plus the 1,625-byte replacement before old-backing release.
- [x] Complete the negotiated handshake and preserve ordinary server,
  certificate, provider, and protocol semantics.
- [ ] Finish initial/post-HRR cancellation and thread-churn controls before
  aggregate KX/provider GREEN.
- [ ] Continue complete TLS custody, funded SQLx TLS-positive, and PostgreSQL
  17.6 qualification.
## Window24b retained provider-thread churn

- [x] Normally merge protected `main@e1750ede386c0ee1001894ab9d91129de5d03fce`.
- [x] Prove retained 1,360-byte per-thread residency, repeat-use idempotence, and same-root churn exhaustion before AWS-LC use.
- [ ] Prove actual wire HRR initial/replacement overlap and cancellation/error/drop ordering.
- [ ] Complete decoded/config/session/cache/send/transcript/error TLS custody.
- [ ] Execute funded AWS-LC TLS-positive and configured PostgreSQL 17.6 positive/hostile qualification.

## Window26 final #451 lifecycle closure

- [x] Exercise initial and post-HRR connection cancellation/drop on the actual
  rustls state path.
- [x] Deny the actual HRR replacement before provider start and unwind the
  initial operation backing without altering retained shared residency.
- [x] Observe replacement release during the successful packet-processing path
  that consumes the owned secret through the TLS key schedule.
- [x] Combine actual lifecycle evidence with invalid-peer error unwind and the
  final-graph provider/KX matrix; mark #451 provider/KX residency `PROVEN`.
- [ ] Continue complete decoded/config/session/cache/send/transcript/error TLS
  accounting before TLS-positive and PostgreSQL qualification.

## Window27 decoded-owner implementation

- [x] Install the existing owner identity in `ConnectionCore` and route normal
  and first-handshake parsing through owner-aware `Reader` construction.
- [x] Propagate the owner through nested readers and reserve exact generic-list
  element capacity before growth, including old/new overlap.
- [ ] Implement the remaining protected payload, message, span, transcript,
  certificate/OCSP, successor, compressed-certificate and retained-session
  ownership boundaries and their complete composition witness.
- [ ] Run funded AWS-LC SQLx TLS-positive and PostgreSQL 17.6 qualification only
  after the complete TLS matrix is proven.

## Window25 actual wire HRR

- [x] Drive a real PQ-first AWS-LC TLS 1.3 client into a P-256-only server and
  assert `FullWithHelloRetryRequest` on the actual rustls state path.
- [x] Prove replacement reservation precedes initial release, exact
  initial-only capacity denies replacement, and pre/post-HRR connection drops
  release the corresponding active exchange with state destruction.
- [ ] Prove the remaining actual key-schedule consumer and cancellation/error
  controls before aggregate KX/provider residency becomes GREEN.
- [ ] Continue decoded/config/session/cache/send/transcript/error custody,
  funded TLS-positive, and PostgreSQL 17.6 qualification.

## Window27 continuation

- Completed the protected #425 handshake-span capacity boundary: pre-allocation
  initial/growth reservations, old/new overlap, high-water retention, and final
  destruction ordering.
- Continue Reader/list/payload/message, transcript, certificate/OCSP,
  successor-state, compressed-certificate, and retained-session custody before
  claiming complete TLS or running TLS-positive/PostgreSQL qualification.

## Window29 decoded bookkeeping repair

- [x] Precharge the exact pinned `ArcInner<DecodedOwner>` allocation and retain
  external custody until the final Arc control block is deallocated.
- [x] Restore checked geometric list growth and destroy partial/mismatched
  prospective backing before debit rollback.
- [ ] Replace successful-list connection aggregation with backing-coupled
  custody that releases on local drop or transfers with retained descendants.

## Window29 decoded payload continuation

- [x] Reserve exact byte-vector capacity before `PayloadU8`/`PayloadU16`
  decoded copies, with denial before allocation and mismatch destruction before
  rollback.
- [x] Roll back charges from partial nested owner-aware message parsing after
  the partial decoded backing has unwound.
- [x] Repair generic-list actual-capacity mismatch cleanup ordering.
- [ ] Attach custody through borrowed payload/message `into_owned` conversion
  and prove parsed plus encoded/source plus destination overlap.
- [ ] Continue handshake AST, transcript, certificate/OCSP, successor-state,
  compressed-certificate, retained-session and complete-handshake composition.
  claiming complete TLS or running TLS-positive/PostgreSQL qualification.

## Decoded-owner review repair checkpoint

- [x] Precharge and externally retain exact `ArcInner<DecodedOwner>` custody.
- [x] Restore amortized geometric decoded-list growth with prospective rollback.
- [ ] Bind each decoded list/payload backing to destruction-time custody; the
      current aggregate remains foundation-only and complete TLS stays open.

## Window30 backing-custody representation boundary

- [x] Preflight a non-allocating custody field after `Message::payload`, so
  backing destruction precedes debit release.
- [x] Confirm that all decoded client parsing can transfer the transaction into
  that field without a second owner or address registry.
- [ ] Obtain the minimum mechanical constructor-factoring lease for excluded
  `client/ech.rs`, `server/tls12.rs`, `server/tls13.rs`, and the existing
  `common_state.rs` literal before retaining the representation change.
- [ ] Resume backing-bound list/payload/message custody and the remaining #425
  matrix; complete TLS and PostgreSQL qualification remain blocked.

## Window31 Message custody continuation

- [x] Apply the protected mechanical ECH/TLS1.2-server/TLS1.3-server constructor amendment.
- [x] Attach successful owner-aware decode debit to a private `Message` custody token ordered after payload backing.
- [x] Preserve ordinary owner-free std/no_std construction and prove move custody without a second debit.
- [ ] Reserve separately for borrowed/deep-owned destinations and split/transfer custody into successor, peer-chain, transcript and retained-session owners.
- [ ] Complete the full TLS composition witness before TLS-positive or PostgreSQL qualification.

## Window32 production Message witness

- [x] Exercise rustls as a production-mode dependency (`cfg(test) == false`) in the
  real AWS-LC wire-handshake harness and require decoded reservation/release events.
- [x] Cover the real owner-aware Message attachment, first-message `into_owned` move,
  and production drop path without promoting complete TLS.
- [ ] Continue per-backing list/payload custody and remove connection-wide aggregate
  rollback as a lifetime authority before retained-state composition.

## Window33 custody safety fence

- [x] Make aggregate decoded release checked so a second release cannot underflow
  accounting or be forwarded to the accepted owner ledger.
- [ ] Replace aggregate checkpoint rollback with explicit local/prospective guards
  as per-backing list and payload custody lands; aggregate accounting remains
  debug state only.

## Window34 payload custody continuation

- [x] Bind successful owner-aware `PayloadU8`/`PayloadU16` capacity debit to the
  byte-vector lifetime with backing-before-token destruction order.
- [x] Exclude exact backing tokens from Message aggregate commit/rollback so an
  error scope releases only still-local custody.
- [ ] Introduce the private charged generic-list backing representation and
  owner-aware destination-copy paths without restoring an uncharged Clone shortcut.
- [ ] Continue retained handshake/transcript/certificate/session composition;
  complete TLS and PostgreSQL qualification remain open.

## Window35 charged HRR cookie repair

- [x] Replace the reachable charged HRR cookie `Clone` panic with an explicit
  fallible owner-aware destination copy.
- [x] Reserve before destination allocation, retain source/destination overlap,
  and prove independent final release plus max-minus-one denial.
- [x] Preserve ordinary owner-free payload `Clone` behavior and audit current
  PayloadU8/PayloadU16 clone sites for equivalent reachable charged copies.
- [ ] Continue generic list backing custody and the remaining retained
  handshake/transcript/certificate/session composition.

## Window36 empty CertificateRequest context repair

- [x] Represent an owner-aware zero-length decoded copy as an allocation-free
  empty Vec without a spurious zero-byte custody token.
- [x] Exercise a decoded TLS 1.3 CertificateRequest with an empty context and
  prove its required clone neither panics nor changes the ledger.
- [x] Preserve the existing non-empty-context rejection and ordinary owner-free
  payload semantics.
- [ ] Continue generic list backing custody and the remaining retained
  handshake/transcript/certificate/session composition.

## Window37 generic backing-bound list representation

- [x] Add a private decoded vector whose exact capacity debit follows backing lifetime.
- [x] Preserve geometric reallocation overlap, move transfer, separately charged
  fallible deep copy, and later-element failure cleanup.
- [ ] Migrate the exact private handshake list fields from aggregate `Vec<T>` custody
  to the backing-bound representation; do not restore infallible charged cloning.
- [ ] Continue payload/message/transcript/certificate/session composition and complete
  TLS/TLS-positive/PostgreSQL qualification.

## Window38 generic charged-copy safety repair

- [x] Remove the arbitrary `T: Clone` charged-copy surface; restrict the generic helper
  to non-allocating `T: Copy` elements.
- [x] Hold prospective destination-vector custody in an armed RAII guard until backing
  construction and final custody commit; unwind drops destination backing first.
- [x] Prove exact max-minus-one denial leaves the source live/charged and leaks no debit,
  while retaining funded overlap and independent final-drop coverage.
- [ ] Census SQLx-client generic lists by MESSAGE_LOCAL/EARLY_DROP/MOVE/DEEP_COPY and
  use field-specific recursive fallible copies for any allocating destination elements.
- [ ] Continue the complete decoded/TLS and PostgreSQL qualification matrix.
- [x] Keep `DecodedVec<T>` available as a plain `Vec<T>` alias in `no_std`, with
  custody compiled only for the owner-aware `std` representation.

## Window39 lifetime census and first production divergence

- [x] Census generic list backing reachable from the SQLx rustls client and classify
  MESSAGE_LOCAL, EARLY_DROP, MOVE_TO_SUCCESSOR/PEER_SESSION, and DEEP_COPY destinations.
- [x] Replace the TLS 1.3 CertificateRequest compatible-signature `Vec` collection with
  a separately precharged backing-bound `DecodedVec<SignatureScheme>` temporary.
- [x] Prove exact funded early-drop release and max-minus-one denial before allocation.
- [ ] Continue certificate/OCSP, peer-chain/session and Message/Payload destination
  ownership; complete TLS and PostgreSQL qualification remain open.

## Window40 retained ticket custody

- [x] Preserve decoded ticket custody across `NonEmpty -> MaybeEmpty` retyping without a second reservation.
- [x] Precharge exact Rust 1.94 `ArcInner<PayloadU16>` backing and retain one debit across Arc clones through final deallocation.
- [x] Carry ticket backing/control custody through TLS 1.2 and TLS 1.3 retained-session values.
- [ ] Precharge retained secret and peer-chain backing/control allocations and continue the remaining complete-TLS matrix.

## Retained session continuation

- [x] Reuse the charged ticket's owner for retained TLS 1.2/TLS 1.3 session construction.
- [x] Precharge and retain secret, deep certificate destination, and certificate Arc-control backing through final destruction.
- [ ] Continue certificate/OCSP message ownership, compressed-certificate overlap, and transcript/hash-context custody.
- [ ] Prove the complete simultaneous-live TLS composition before TLS-positive or PostgreSQL qualification.

## Window43 charged certificate transfer repair

- [x] Preserve the already-charged TLS 1.3 certificate outer vector across
  `ServerCertDetails::into_owned` and `CommonState::peer_certificates` transfer.
- [x] Keep ordinary borrowed/owner-free `CertificateChain::into_owned` semantics unchanged.
- [x] Prove pointer/capacity identity, no second debit, and backing-before-token release.
- [ ] Continue TLS 1.2 and compressed-certificate custody, then transcript/hash contexts.
