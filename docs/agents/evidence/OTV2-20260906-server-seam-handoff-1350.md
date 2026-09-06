# Server Seam successor checkpoint — 2026-09-06 13:50 UTC

## Authority and stop condition

User explicitly requested saving work to the repository and a checkpoint for the next agent. All implementation writers stopped. Work162 remains the sole control plane; no material architecture escalation is active. This document does not authorize new implementation scope, reset an admission, or qualify WIP. Checkpoint preflight: issue162 comment5559665236. Historical task records and PR comments remain authoritative evidence; earlier “current” prose is superseded by this checkpoint.

## Remote reconstruction

Repository: Oteryn/Oteryn-Game. Protected main `d9d1b566acb57b537ff901d9765c32a95110c259`, tree `2618a9412b1dea2272c24641b11beb0395ee8e11`, parent `ad7273e3e91a4e4254abb9aa2710c7e0c9754afe`. PR348 owning-loss amendment merged 13:48:22Z after exact CI34036251243 and full Merge Queue34036760848 SUCCESS. PR347 wire allocation merged at that parent, full MQ34035534050 SUCCESS. Registry342/PR345 is integrated and archived by348; all154 entries unchanged. A318/321 and Foundation326/331 remain completed/released.

| Lane | Existing branch / PR | Saved head | Saved tree | State |
|---|---|---|---|---|
| B329 | agent/durable-fresh-admission-child-b-329 /335 | 97eb86f01d22b9d8bc42282a341799369412bba1 | 79c500b1d9054b2adca54335e66b244183749690 | Failing WIP, stopped |
| Foundation338 | agent/post-grace-foundation-successor-338 /343 | ceccd130b50c871236b096723754c6a26f1c11bf | c7993a4258a52a96753dda3d575bfaeb9c86d830 | Green checkpoint, full integration pending |
| Wire346 | agent/native-evidence-wire-346 /349 | 843792e5b94baff4f6db4036bd303ed4e2f49302 | d4a34fcd83676679f8341f65865b28283151193f | Frozen green; not enqueued/integrated |
| Server Seam247 | agent/otv2-gameplay-server-seam-01 / no implementation PR | 9370b254c6ac4f6529e069c1968ae6bfa1e1750e | 3681b01f8a08fc5c9b210b06957834477502b16f | Task3 WAITING_DEPENDENCY |

## First successor action

Fetch current main, these exact branch heads, PR bodies/comments/checks, open allocations and ownership. Reconcile any later remote changes before mutation. Rebind through Work162 to existing branches and admissions; do not reset/rebase/force-push or recreate lanes. Then repair the saved B WIP on its admitted paths. Parallel Foundation work may resume only after its paused grant is freshly rebound. Wire integration is an independent available coordinator action once its exact current evidence is reconstructed. Do not claim Server Seam release from A+B or documentation alone.

## B329 saved WIP and immediate repair

Immutable admission `b8ae4c965cc7f686b89b4d5c0ba2bc04af6e07fd`. Saved commit parent `68609f8ae93e9fb74bff817eaa30e48045eb1b95`, parent tree `b7f239f9278f97db11e5843210f11a62c706415b`. Seven changed files: durability admission_authority_guards.rs, admission_journal.rs, db.rs, fresh_admission.rs, mod.rs, and its existing task/plan. The WIP wires one registered backend/shared pool and custody into nine transaction starts and V2 terminal lookup; constructor signatures are preserved and historical cfg(test) path separated.

Strict Clippy is RED: items_after_test_module in db.rs and dead-code errors because production AdmissionRuntime is unexercised in the source-included PostgreSQL target. No new WIP SQL run or qualification. Fmt, diff and governance26/9 passed. Independent preservation-only review found no obvious P0/P1, scope leak, secrets or destructive migration changes; it is NOT integration approval. Detailed repair steps are in the saved B task/plan. Local diagnostic log, if retained: /workspace/scratch/ec4cc99115b7/b329-runtime-wiring-clippy.log. Repair layout and exercise the production runtime path meaningfully; run strict Clippy and actual configured PostgreSQL qualification before any success claim.

Last green parent68609: actual PostgreSQL17.6 canonical34036943914/Linux101496670199, custody test PASS13:46:38, 131072/131073 projection PASS13:46:44, total309 PASS/0 FAIL13:46:53. An earlier unconfigured run is not SQL evidence. Governance34036943929 and semantic34036943919 SUCCESS. This proves low-level custody only, not shared executor, all-backend routing, two-second termination, accounting or owner acknowledgement.

Custody uses stable generation row0 and slots1/2 in one game_durability_executor_custody relation (15 total). Takeover takes exclusive transaction advisory lock; operations retain shared generation fence through commit. Pending slots cannot be overwritten or silently cleared. Raw first-loss PREPARE currently fails closed before attempts/effects. Owning loss must be separately authorized and durably recorded before PREPARE.

Existing exact14 paths: durability/{fresh_admission.rs,admission_authority_guards.rs,admission_journal.rs,db.rs,mod.rs,schema.rs}; migrations/0002_fresh_admission_authority.sql; tests/durability_postgres.rs; tests/support/{postgres.rs,authority_matrix.rs,authority_recovery.rs}; src/bin/oteryn-game-migrate.rs (all under apps/game-server); docs/agents/tasks/active/OTV2-20260906-durable-fresh-admission-child-b-329.md; docs/superpowers/plans/2026-09-06-durable-fresh-admission-child-b.md. No Foundation, Cargo, lib, vendor, registry, workflow or0001 edits. Unreleased0002 remains editable;0001 immutable.

Prior evidence remains in PR335 and task/plan: resource86c593 canonical34036298616 was307PASS/1FAIL (INT4 octet_length fixture decoded as i64), repaired by explicit bigint in68609; safe first-loss fd729 canonical34035642462 passed306/306; fresh8a807 canonical34033942736 passed305/305;4139 canonical34033443274 passed304/304; guards c426 canonical34031690145 passed303/303; df047 canonical34031174082 passed302/302. Older failing runs and rejected unpublished implementations are retained, not erased. Raw PREPARE must never demote a healthy controller or poison a future attempt epoch.

## Foundation338 pending owning loss

Admission `4f35ec5a56f5e8b0c32db4503d2bd3503b8828ee`; saved head parent `f2ccc7de74ed58bd47c7c71c0d1e62c8a5fce331`. Exact CI34036889769, governance34036889765, semantic34036889845 SUCCESS. Checkpoint adds matrix/doctests; no runtime behavior delta. Full package324 library/20 doctests passed before final P2 test-fixture repair; fresh focused tests and strict Clippy passed after repair. Canonical mutation baseline now uses coherent revision12 at both locked boundaries, preventing revision mismatch from masking the intended predicate. B-integrated compile and actual post-grace SQL qualification remain pending.

Protected348 and issue338 comment5559655141 authorize the seventh path foundation/control_loss_durability_tests.rs. Grant was NOT dispatched or implemented before user stop. Existing six paths: foundation/{admission_recovery_inner.rs,fnd04_verifier.rs,admission_authority_publication.rs,post_grace_recovery_tests.rs} under apps/game-server/src; its existing OTV2-20260906-post-grace-foundation-successor-338 task and 2026-09-06-post-grace-foundation-successor plan. Read-only coordinator addendum: docs/superpowers/plans/2026-09-06-control-loss-foundation-amendment.md.

Implement sealed owning-runtime evidence of real unexpected playable loss, current actor/session/transport/generation/runtime fences and original epoch/grace/protection/rearm. Private authority plus complete inert original operation; bounded final current effect. Distinct durable loss operation BEFORE PREPARE, never a raw flag/socket/restart/receipt granting authority. Clear only exact current controller on same canonical session, preserve claims/lease/protection, and create truthful reconnectable continuity. Cover genuine first fresh loss and subsequent loss after Restored/new owner epoch, replay/ambiguity/stale/current negatives and anti-forgery. No facade/lib/Cargo/SQL/bootstrap/actual-source edits or new numeric thresholds. B consumption follows protected Foundation delivery and authorized SQL integration.

## Wire346 awaiting integration

Immutable admission and parent `ad7273e3e91a4e4254abb9aa2710c7e0c9754afe`. Exact CI34036664393, governance34036664480, semantic34036664432 SUCCESS. Independent/root final review zero findings;291 library/9 wire tests and strict Clippy passed. One P2 malformed-numeric test repair retained. Pure inert DTO codec only, no live-source authority. Fixed parser slots19, raw8192, keys64, strings256, members16, request buffer1024 then String; strict canonical numeric/UUIDv7/base64/UTF-8/surrogates/closed fields; all four native operations and separate V1 bindings/fixed V2.

Exact five paths: apps/game-server/src/admission_evidence.rs; apps/game-server/tests/admission_evidence_wire.rs; apps/game-server/src/lib.rs (ONE export only); docs/agents/tasks/active/OTV2-20260906-native-evidence-wire-346.md; docs/superpowers/plans/2026-09-06-native-evidence-wire.md. Existing247 export lease remains held until normal integration/archive/release. Next: normal merge-up to current main, preserve owned bytes, independent delta/head rebind, new exact CI and full Merge Queue/protected readback. Never assume pre-merge CI qualifies a changed head.

## Resource blocker requiring a separate allocation

Read-only audit of actual SQLx0.9.0: sqlx-postgres connection/stream.rs recv_unchecked accepts peer uint32 frame length, then sqlx-core net/socket/buffered.rs reserves announced remaining bytes before awaiting socket data. Five-byte header can exceed4MiB active budget before timeout or SQL row projection. Error/Notice frames and ParameterStatus storage are also uncapped. DataRow/RowDescription allocate from peer counts; retained Bytes and type/table caches matter. Public connect options expose no preallocation receive cap/socket injection. Initial8192 capacity, statement_cache_capacity(0), shrink_buffers or timeout alone do not prove bounds.

No driver patch or new lane is authorized. A plausible proposal, not an accepted implementation, is vendoring exact sqlx-postgres0.9.0 with provenance/licenses and a private bounded PgStream using public sqlx_core Socket; preallocation ledger, charged retained backing/temporary overlap, count checks and bounded status/type/table caches, including idle connection memory. Would require explicit vendor/Cargo.toml patch/Cargo.lock/provenance/test allocation and serialized B activation. Cargo is held by247; coordinate its lease and existing dependency PRs. No arbitrary new wire limit or fictional4MiB driver allowance. Test hostile length headers before allocation, huge count/tiny body, repeated status, retained clones, caches/cancellation/close and valid maximum actual PostgreSQL cases. Proxy/settings alone do not establish the decoder/cache proof.

Accepted registry bounds remain unchanged: queue8x524288=4MiB; active2x4MiB=8MiB; total12MiB charged (not RSS), one executor/process; row131072; operation65536/guard8192; pending2x131072; queue1000ms/pass2000ms including acquisition/locks. Slots remain occupied through ambiguity/backend/owner acknowledgement. All-row-family/resource ledger and full lifecycle qualification remain open.

## Actual sources and preserved Server Seam

Task3 is still WAITING_DEPENDENCY. No implementation PR for247. Shared lib/main/server Cargo/root Cargo/Cargo.lock/foundation protocol ownership remains247 except the serialized ONE export346. No local journal substitute or new branch/reset. Platform last read-only protected main `3b2ea1c7392187d5d22488673073dc8f8305a374` lacks native UUID issuance, four native evidence operations and recovery V2 producer. Accepted ADR0028 is not producer implementation. Additive UUID/backfill, DTO/routes, Ed25519 issuer, transactional observation revisions, exact replay, shared V1/V2 Account floor and distinct Recovery trust remain separate actual-source obligations.

No Platform/META/Atlas/production/secret/bootstrap/live-account/deployment writes are authorized. Actual PKI/descriptors and Game owning-source composition remain unqualified. Character creation requires Account existence and operation permission; assignment needs scoped owner CAS/nonready fences without self-grant. Read-only rustls0.23.43 finding: Certificate Vec is allocated before custom verifier, so4-cert/4096DER/16384-total caps cannot be proved solely there; builtin65536 handshake and outgoing set_buffer_limit do not prove cumulative receive budget. A future allocated transport experiment is needed, not an impossibility claim.

## Paused windows and custody

Owner stop approximately13:50UTC; administrative preservation followed. Do not count wall time while paused as new productive windows or silently renew/reset budgets. Fresh coordinator reconstruction must determine remaining budget before resume.

- B329: window5 grant5559643400 at13:47–14:47; about3 minutes before stop. Completed windows4, rotation1, repair6. Prior windows and lost unpublished work remain recorded.
- Foundation338: window4 grant5559655141 at13:49–14:49, unstarted. Completed windows3, rotation1, known repair4 plus prior UNKNOWN; no reset.
- Wire346: window1 grant5559516855 at13:27–14:27, frozen after publication; completed0, rotation0, repair1.
- Programme720-minute original budget/history persists; no automatic renewal. Stop does not release paths or authorize another writer.

All six helpers were stopped or performing bounded preservation/review only. Next agent may use helpers on disjoint admitted work after rebind; keep one writer per lane and Work as control plane. Activate supervising architect only for a real material architecture escalation.

## Verification of this checkpoint

Product heads above are saved independently of this documentation PR. This checkpoint is intended for reviewable publication, not immediate main integration. Documentation governance and whitespace checks plus independent handoff review are recorded in its PR. New WIP CI results must be read afresh; last-green parent results never qualify the WIP child. All original PR comments/task histories remain available if transient local worktrees disappear.
