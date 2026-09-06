# Durable Fresh Admission Child B Implementation Plan

This exact prospective allocation activates only after protected integration and Work readback. The actual allocation merge becomes immutable worker admission/base before first mutation.

**Goal:** Deliver #329's complete asynchronous PostgreSQL fresh admission, typed publication, exact-operation recovery and affected session lifecycle adapter, preserving one canonical GameSession owner and the accepted final decision L.

**Architecture:** Accepted FND-DUR-FRESH-ADMISSION-V1 (#313/#317), FND-DUR-FRESH-CLAIM-PUBLICATION-V1 (#324/#325), and protected #326/#331 implementation readback required before admission. Reviewed reference: native `14389fe41e8d3053e5143bdeee2acc7dd97eff00`, tree `b1348af0baecc10e2f54eba5766d45b3060e3208`. The four authored blobs match local `925387f70bc4b8ab0ae2bf70058b89e03c8c8792` (tree `9c423133b73099289adcfdfc391af3d34ba5c79e`); the final integration tree additionally includes accepted #330 documents. These are reference evidence, NOT B admission SHAs.

**Authority:** Issue #329 and parent #162 remain GitHub lifecycle authority. Work must integrate #331, reconcile #326 custody and applicable source-readiness architecture, integrate a fresh allocation, then record its actual protected SHA and dispatch one writer. Proposed branch: `agent/durable-fresh-admission-child-b-329`. Actual source readiness remains C/#319; B does not release Server Seam #247. The separate inherited post-grace limitation neither blocks B nor grants new lifecycle policy.

**Stack:** Existing Rust 1.94.0, SQLx/Tokio/serde_json and hosted isolated PostgreSQL 17.6. No dependency changes. Local PostgreSQL is environmentally unavailable; do not bypass PostgreSQL root checks or treat skipped local cases as SQL proof.

## Exact prospective ownership

| Path | Responsibility |
|---|---|
| `apps/game-server/src/durability/fresh_admission.rs` | Async fresh commit/reconcile, complete operation codec, session snapshot and bounded command/completion adapter |
| `apps/game-server/src/durability/admission_authority_guards.rs` | Typed guard persistence/publication, high-water and decision consistency, common key locking, conditional lifecycle effects |
| `apps/game-server/src/durability/admission_journal.rs` | V1 shared locks, fresh-origin continuity, reservations, commit/control-loss/reconcile integration |
| `apps/game-server/src/durability/db.rs` | Shared DB primitives if required, scoped connection/transaction helpers |
| `apps/game-server/src/durability/mod.rs` | Adapter exports and V2 replacement/claim integration |
| `apps/game-server/src/durability/schema.rs` | Forward-ledger compatibility and schema validation |
| `apps/game-server/migrations/0002_fresh_admission_authority.sql` | Forward schema only |
| `apps/game-server/tests/durability_postgres.rs` | Enforced source-identical sealed harness and actual PostgreSQL cases |
| `apps/game-server/tests/support/postgres.rs` | Independently controlled fixtures, barriers, isolated restart/role support |
| `docs/agents/tasks/active/OTV2-20260906-durable-fresh-admission-child-b-329.md` | Bounded worker checkpoint and technical evidence |
| `docs/superpowers/plans/2026-09-06-durable-fresh-admission-child-b.md` | This technical plan, only if explicitly included in Work's allocation |

No Foundation, Cargo, lockfile, workflow, released 0001, listener, production provider/registration, deployment or external repository writes. Concrete missing surfaces require Work amendment before mutation. Steps are serialized because schema, shared lock order and canonical lifecycle mutations are coupled; independent read-only analysis and exact-head review can run alongside the sole writer.

## Protected fixture amendment (window 2)

PR340 protected merge `4f35ec5a56f5e8b0c32db4503d2bd3503b8828ee` / MQ34023932923 and Work comment5558302168 add exactly `apps/game-server/tests/support/authority_matrix.rs` and `apps/game-server/tests/support/authority_recovery.rs` to the eleven paths above. These paths isolate transport/nonce fixture accounts and preserve independent source mutation evidence; they do not expand runtime policy or permit assertion weakening. Immutable B admission, same branch/PR and cumulative counters are retained. The semantic Foundation338 allocation remains separate and disjoint.

## Protected migration-binary composition amendment (window 3)

Allocation344 merge `9ceeb231e2bb92c70eae83369c84f0f3fa6fccb2` / Merge Queue34032269848 adds exactly `apps/game-server/src/bin/oteryn-game-migrate.rs`, fourteen total paths. The permitted change only imports the canonical-library `MigrationExecutor` in place of duplicate source inclusion; migration-only environment/connection/embedded ledger behavior and private seals are unchanged. No additional Foundation/Cargo/lib/registry scope follows.

## Execution windows and custody

The worker has one 60-minute execution window, not a multi-hour grant. The numbered checkpoints below are technical milestones, not seven automatically authorized hours. At the window boundary publish an authorized durable checkpoint and one next action; Work decides continuation/rotation. Preserve the same canonical branch/history and immutable admission. Only one writer owns the worktree at any time.

Rotation does not reset failure retries, repair cycles, unchanged CI observations, frozen candidate identity or review findings. Per-window elapsed time may restart only for a genuinely newly granted window; cumulative gate/failure counters remain attached to their actual candidate/generation. A frozen failure receives verified disposition/repair or remains waiting, never a fresh budget by renaming the worker. No redundant CI runs or metadata-only commits to reset counters. Work owns final integration and native/local tree mapping when required.

## Checkpoint 1 — executable harness and actual hosted SQL RED

- [ ] Compiler-verify the integration target with `extern crate self as oteryn_game_server;` and `#[path = "../src/foundation/mod.rs"] pub mod foundation;`, retaining source-included Durability. Keep a single local Foundation type universe.
- [ ] Implement fixture owner seals only inside the test crate; add no production forging constructor. Existing included Foundation tests use cfg(test); preserve independent source negatives despite additional test-only conveniences becoming visible.
- [ ] Compile locally where available, then obtain the first meaningful SQL RED through the existing configured hosted PostgreSQL17.6 `--test durability_postgres` lane on the authorized branch/PR. Record exact head, run/job/case and intended assertion failure; a compile failure is separate evidence, not SQL RED.
- [ ] Inventory each SQL statement/constraint/trigger/FK that may contend. Capture initial INSERT, V1 epoch/attempt/nonce/reservation paths, mutating reconciliation and V2 predecessor/candidate replacement. Do not change workflows or start a local database workaround.

## Checkpoint 2 — forward schema and lossless decode

- [ ] Add immutable fresh receipt keyed by tagged 33-byte replay encoding, exact 16-byte transport, initial generation 1, typed identity mirrors, semantic version, full `FreshAdmissionOperationV1` and original `authorization_decided_at`.
- [ ] Reuse current session table; relax only truthful absent fresh continuity and enforce required continuity for reconnectable/prepared states. No zero/default reconnect history.
- [ ] Add nonterminal account uniqueness alongside character uniqueness; fail migration on conflicting legacy data. Preserve full u64 with NUMERIC(20,0).
- [ ] Add four typed guard domains, owner source/accepted publication high-water and CAS, persistent tombstones/denials and durable decision/effect consistency. No source bootstrap from migration/session/grant.
- [ ] Extend the existing transport reservation namespace with exactly one truthful fresh/reconnect owner binding. Preserve legacy rows and permanent non-reuse.
- [ ] Strictly decode every historical operation/transition/provenance field and verify typed mirror columns. Historical restoration cannot recreate a live capability. Include lifecycle operation/effect evidence needed for exact replay/reload.
- [ ] Preserve exact migration ledger/checksum/ahead/behind/locking checks and runtime role DDL denial.

## Checkpoint 3 — common pre-L locking and publication

Apply one protocol to fresh, publication, V1/V2, terminal release and mutating reconcile:

1. Determine full footprint: accounts, characters, runtime/trust guards, replay/nonce, predecessor/candidate/incumbent session IDs, old/new transport refs, epochs, attempts and replacement identities.
2. Acquire required relation locks in a documented fixed table order at modes sufficient for planned statements, before the decision. Inventory new 0002 foreign keys and avoid deferred semantic acquisitions at COMMIT.
3. Acquire transaction-scoped exclusive advisory locks for all domain-tagged keys. Use stable encoding; deduplicate and globally sort physical lock IDs. Hash collisions may only over-serialize. Compare complete typed identities independently.
4. Reread footprint under protection. A newly discovered key means rollback/rebuild, not an out-of-order lock appended to an existing transaction.
5. Acquire existing rows in fixed table/PK order, including affected attempts/continuity/reservations/FK prerequisites. Absent-key locks protect future rows. Lock the complete predecessor attempt set before V2 broad updates; character protection covers actor epoch counts.
6. Classify exact immutable replay and all claim/candidate/reservation prerequisites. Only then sample `clock_timestamp()` and invoke Foundation's complete predicate for a new operation.
7. Persist exact authorized effects and COMMIT holding every protection. New semantic contention or transaction retry invalidates the tentative decision; ambiguous COMMIT reconciles the original operation.

- [ ] `ON CONFLICT DO NOTHING`, short lock timeouts and unlocked absence reads are not proof of nonblocking acquisition. Move existing V1/V2 deadline reads after all relevant new acquisitions without redesigning accepted reconnect decisions.
- [ ] Publish via existing sealed request and `validate_locked`; retain source time on exact replay, reject rollback/contradiction/stale CAS, update multiple domains atomically. Ordinary publication never changes claims or owner bindings forbidden by Foundation.
- [ ] Resolve publication completion from independently read accepted rows. Pending/unacknowledged publication is not active authority.

## Checkpoint 4 — fresh COMMIT and original-operation recovery

- [ ] Async API consumes `FreshAdmissionCommitRequestV1`; persist and correlate **`operation()`**, not merely `binding()`.
- [ ] `validate_at_decision(rows, Some(db_now))` supplies already owner-authored successors. SQL must not synthesize source revisions, decisions or source timestamps.
- [ ] Atomically persist successors/high-water, canonical ACTIVE session generation 1, receipt and transport reservation. No fresh durable PREPARE and no success before COMMIT.
- [ ] Restore using `FreshAdmissionCommitReceiptV1::restore(operation, original_decided_at)` and classify exact retry with complete operation equality. Changed transition decisions/effects conflict even if authorization/candidate match.
- [ ] Reconcile receipt plus current session under one fenced snapshot. Proven absence/conflict and uncertain storage outcome remain distinct; ambiguity never substitutes a refreshed operation/candidate/transport.
- [ ] Implement bounded enqueue/completion integration without SQLx/block_on/spawn-and-wait in Foundation logical writer. Current adoption remains a new normalized independently sourced boundary.

## Checkpoint 5 — canonical lifecycle and reconnect integration

- [ ] Fresh-origin ACTIVE row passes truthful fresh-receipt validation; real first control loss supplies actual continuity before reconnect consumers require it.
- [ ] Audit `admission_journal.rs` nullable `control_loss_epoch` decoding (currently read as `String`) and `active_committed_binding_is_valid` (currently requires a committed reconnect attempt). Add explicit validated fresh-origin receipt/current-session branches for first control loss; never manufacture an epoch or reconnect record. Keep missing continuity fail-closed on paths requiring real reconnect history.
- [ ] Claim-preserving control loss/reconnect uses `validate_claim_preserving_session_v1` with independently current rows/session. Preserve source metadata and holders.
- [ ] Replacement consumes `TerminalReplacementClaimTransitionV1` plus existing canonical authorization and exact candidate record. Persist conditional claim effects with the matching predecessor/candidate/attempt/continuity transaction, preserving V1/V2 policy and attempt budgets.
- [ ] Accept #326 replacement/release sealed capabilities as explicit Durability adapter method arguments. Existing public current-snapshot/history constructors support decoding; they do not grant source registration. No extra Foundation path is currently demonstrated necessary.
- [ ] Release consumes `TerminalReleaseClaimTransitionV1::validate_locked`; clear both matching holders with matching canonical terminal effect, preserve lease/source floors and reservations. Stale session/generation cannot release successor claims.
- [ ] Preserve #326's structural freshness repair: terminal release accepts old structurally valid unchanged nested Platform observation; its new Game account/presence successor still needs valid freshness. Do not apply fresh-admission validation to relinquishment or re-age historical evidence.
- [ ] Snapshot reconstruction uses actual current connection/lease/scope/state/transport columns and independent authority where required. Immutable commit/receipt fields supply expected identity, never substituted current facts.
- [ ] Existing terminal-replacement lagging durable scope/current-owner reconciliation remains governed by existing accepted behavior. Report actual incompatible APIs with exact evidence; do not fabricate current facts or new post-grace policy.

## Checkpoint 6 — real PostgreSQL17.6 qualification matrix

Each negative changes one authority invariant with valid independent control facts. Use bounded barriers and observed pg_locks/backend state, not arbitrary sleeps. Every SQL family below must execute in the enforced configured integration target.

| Family | Required cases |
|---|---|
| Atomicity | One receipt/session/claim pair/reservation; injected rollback after every tentative effect; no accepted high-water advancement on abort |
| Operation replay | Exact retry, changed candidate/replay/transport and changed transition decision/source/effects; original L unchanged |
| Incumbents | Same account/different characters, same character/different grants, candidate session collision; one durable winner |
| Transport | fresh/fresh, fresh/reconnect, reconnect/fresh, same-binding replay, pre-existing migrated reservation reload; terminal state never releases namespace |
| Guards | Missing each domain; wrong key/account/character/world/channel; stale lease/scope; each independent runtime/revision/security/trust mutation; wrong owner/purpose |
| Publication | Absent/conflicting bootstrap, stale CAS, equal-revision contradiction, accepted decision reuse, source/time rollback, overflow, tombstone restart; atomic batch failure |
| Concurrency | Publisher-before/admission-before for all guard domains; reversed batch input order; replacement versus predecessor COMMIT; two replacements; delayed PREPARE versus actor budget; release versus new claim |
| Time | Expiry during each pre-L relation/key/row/uniqueness/FK acquisition; future/nonmonotonic evidence; fresh Game release successor expiry |
| Persistence delay | Real backend barrier after valid L/effects before COMMIT, later commit retaining L, independently stale adoption rejection. Do not call this WAL/fsync-stall proof without an actual such experiment |
| Recovery | Lost delivery after actual COMMIT, new process/connection reload, exact receipt plus current snapshot; no second session; ambiguous original operation retained |
| Lifecycle | No fabricated fresh continuity; first control loss and V1/V2 reconnect; unchanged claims; exact replacement; stale release rejection; terminal release then fresh admission; historical replay never reacquires |
| Release repair | Aged valid nested Platform history accepted with fresh owner successor; provenance authority/time/accepted-revision/allowed-state substitution rejected; no history re-aging |
| Storage | All full-u64 fences and strict enum/UUID/ref lengths; malformed operation/lifecycle payload and mirror corruption; restart preserves source/lease/runtime floors |
| Migration/role | Fresh, behind, ahead, checksum, concurrent migration locks, conflicting legacy account incumbents; runtime role cannot execute DDL |

No absent local PostgreSQL run qualifies these cases. The actual hosted head/run/job/test evidence is mandatory; normal library tests and compile-fail docs remain separately required.

## Checkpoint 7 — full qualification and exact-head handoff

- [ ] Focused RED/GREEN and sibling finding-family sweeps complete; all accepted material findings repaired or explicitly rejected with exact evidence, all P2 dispositioned.
- [ ] `cargo +1.94.0 fmt --all --check`; `cargo +1.94.0 run --locked -p oteryn-architecture-check -- workspace .`; task governance validation.
- [ ] `cargo +1.94.0 test --locked -p oteryn-game-server`, doctests, and configured hosted `cargo +1.94.0 test --locked -p oteryn-game-server --test durability_postgres`.
- [ ] Full canonical selected build/strict Clippy/workspace and platform/security/supply-chain checks remain required; package/focused checks never replace the canonical gate or Merge Queue game-gate.
- [ ] Complete material metadata before freeze, whole-diff adversarial self-review and genuinely independent exact-head high-risk review. Read-only reviewers do not receive write custody.
- [ ] Work verifies remote exact head/tree mapping, scope, review findings and checks, integrates normally, reads protected main and archives/releases custody. Upstream movement is reconciled without resets/force pushes; affected validation reruns only for concrete invalidation.

**Single current next action:** Work qualifies this exact allocation, then binds its actual protected merge and dispatches the sole worker.

### Window 4 first-loss safety repair and exact dependency

Published fresh/rollback SQL checkpoints have actual PostgreSQL17.6 evidence304/305 passed respectively (canonical34033443274 and34033942736). Independent review rejected unpublished first-loss tree77bc: raw PREPARE is not owning unexpected-loss authority, and initial stale attempt persistence can poison an unopened epoch. Repair cycle3 removes those optional-context APIs and rejects initial NULL continuity before retained effects. Configured V1/V2 expired-then-distinct-current negative checks unchanged session/receipt/claims and zero attempt/child/continuity rows; actual SQL is pending.

No existing exported sealed real-loss capability was found: the owning facade currently calls synchronous unsealed mark_control_loss. A separately allocated additive Foundation owning-loss source/request must bind the exact current transport/generation/actor/runtime fence and real epoch/grace/protection continuity, with pure locked-current revalidation. PREPARE then consumes established continuity. A public raw record or immutable expected snapshot cannot substitute for that prerequisite. This is a concrete minimal Foundation dependency, not permission to edit outside B's lease. Continue other admitted lifecycle/resource/locking work while Work handles it. All full-plan acceptance above remains mandatory.
