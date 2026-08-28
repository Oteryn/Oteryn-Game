# OTV2-20260828-impl-durability-successor

```yaml
task_id: OTV2-20260828-impl-durability-successor
title: Clean-history Durability recovery successor
mode: IMPLEMENT
status: QUALIFICATION_PENDING
integration_state: EXACT_HEAD_QUALIFICATION_PENDING
repository: Oteryn/Oteryn-Game
base_branch: main
branch: impl/game-durability-journal-recovery-240
issue: 167
recovery_issue: 240
parent_coordinator_issue: 162
historical_pr: 212
pr: 243
owner: Oteryn: sol durability lead
allocation_pr: 241
allocation_merge_sha: a171410de07c2dab718f52f780d4314bdcc53604
admission_main_sha: a171410de07c2dab718f52f780d4314bdcc53604
initial_red_tree_commit: 896c26901bd1564577a716df934ba289b6e6c188
source_snapshot_pr: 212
source_snapshot_head: fb30fba2a888835dfc7cbde27f940b79d7bfe05d
source_snapshot_mode: read_only_file_content_only_no_commit_inheritance
exact_current_head_evidence: initial GREEN head 20741682ab71fc50da53c82f827ae144712baec0 is immutably evidenced by PR #243 comment 5453327626 and trusted run/job 33176930382/98867863053; post-qualification P1 repair GREEN head 783ec1dfa8af6791e5ad8d06f13f32a6e62b985a is evidenced by Rust workspace run 33182999525 and PostgreSQL job 98888721229; this task-finalization commit still requires fresh exact-head PR #243 review and CI because a commit cannot contain its own SHA
write_authority: exact_owned_paths_from_recovery_allocation_241
shared_paths: none
external_repositories: []
owned_paths:
  - apps/game-server/build.rs
  - apps/game-server/migrations/0001_admission_reconnect_journal.sql
  - apps/game-server/src/bin/oteryn-game-migrate.rs
  - apps/game-server/src/durability/admission_journal.rs
  - apps/game-server/src/durability/db.rs
  - apps/game-server/src/durability/mod.rs
  - apps/game-server/src/durability/schema.rs
  - apps/game-server/tests/durability_postgres.rs
  - apps/game-server/tests/support/postgres.rs
  - docs/agents/tasks/active/OTV2-20260828-impl-durability-successor.md
public_contracts:
  - DUR-RECONNECT-AUTHORITY-V1
  - DUR-RECONNECT-TRANSPORT-REF-UNIQUENESS-V1
  - Foundation V1 reconnect durability boundary
```

## Outcome

Reconstruct and qualify the journal-only PostgreSQL reconnect Durability candidate on clean ancestry after the PR #212 provenance incident. PR #212 remains immutable historical evidence and is never merged, rebased, rewound, force-pushed or treated as qualification evidence for this successor.

## Proven recovery authority

- `PROVEN`: recovery allocation PR #241 merged as protected `main@a171410de07c2dab718f52f780d4314bdcc53604`.
- `PROVEN`: successor branch `impl/game-durability-journal-recovery-240` descends directly from that protected-main allocation merge through clean test-only commit `896c26901bd1564577a716df934ba289b6e6c188`.
- `PROVEN`: Draft successor PR is #243.
- `PROVEN`: only automatically admitted historical source is exact #212 snapshot `fb30fba2a888835dfc7cbde27f940b79d7bfe05d`; later #212 heads require a new durable control-plane source-admission decision.
- `PROVEN`: current RED generation contains only the frozen regression-harness blobs plus this worker-owned task packet. No production Durability/migration/build blob is present.
- `PROVEN`: trusted `.github/workflows/rust.yml` provides PostgreSQL 17.6 job `Rust / Durability PostgreSQL harness` and executes `cargo +1.94.0 test --locked -p oteryn-game-server --test durability_postgres` for this path set.

## Proven RED -> GREEN chain

The successor-owned RED and GREEN gates are terminally proven on clean ancestry:

1. `TDD_RED_PROVEN`: exact head `07a724db929dc1aa46556177b81a2b36f91238a2`; trusted run/job `33176654398` / `98866989164`; durable PR #243 comment `5453275715`. PostgreSQL 17.6 was healthy, the exact locked focused command executed, and it failed with the expected missing `apps/game-server/src/durability/mod.rs` compiler error.
2. `TDD_GREEN_PROVEN`: exact head `20741682ab71fc50da53c82f827ae144712baec0`; trusted run/job `33176930382` / `98867863053`; durable PR #243 comment `5453327626`. PostgreSQL 17.6 was healthy and `cargo +1.94.0 test --locked -p oteryn-game-server --test durability_postgres` passed `44/44` (including the process-replacement child proof).
3. Between those heads, only the allocation-approved frozen implementation/migration/build blobs were restored from admitted read-only file source `fb30fba2a888835dfc7cbde27f940b79d7bfe05d`; no #212 commit was inherited or cherry-picked.
4. Post-qualification review found and the lane repaired two P1 control-plane gaps: exact-epoch protection continuity now compares `context_game_session_id`, and the durable reconnect-session row is the actor-wide `UNIQUE (character_id)` session/epoch anchor. Their respective test-only RED heads were `0129e9c4576894fd5b3d9184ea21afea97c1d204` and `48dac03fecce273836776ae434e4b0a913c6be18`; the latter's full PostgreSQL harness failed only because a later epoch on a distinct session was accepted.
5. The P1 repair GREEN is `783ec1dfa8af6791e5ad8d06f13f32a6e62b985a` (Rust workspace run `33182999525`, PostgreSQL job `98888721229`). This task-only finalization commit intentionally creates a new candidate head. It is `QUALIFICATION_PENDING` until fresh exact-head whole-diff self-review, required strict read-only Codex review, zero unresolved P0/P1/P2 findings/required threads, and required repository CI are durable on PR #243. PR #243 remains Draft and must not be merged by this lane.
6. Fresh review found one P1 and one P2 on task-finalization head `80b94d79ddd0ceac7f33939951449f816ba9f5c2`: a committed COMMIT replay did not revalidate its exact transport-reservation/current winner, and a committed authority anchor accepted absent or malformed compatibility/security evidence. Test-only RED head `7f792ba98d04818cc2e73904a5444b99e922dab6` is proven by Rust run `33184342450` / PostgreSQL job `98893334040`: the full harness failed exactly at both focused regressions. The minimal repair GREEN head `4e8760edb792cdc241f430420ac93759b155007a` is proven by Rust run `33184628524` / PostgreSQL job `98894312224` PASS. This packet update intentionally creates a further candidate head and remains `QUALIFICATION_PENDING`.

Frozen RED blobs:

```text
apps/game-server/tests/durability_postgres.rs
  460ad5888d8e870bbeda50a3dc8f64b24a30c1cb
apps/game-server/tests/support/postgres.rs
  bcb243f6c4823a14ec8116b72439c2c79c115d94
```

## Historical GREEN restoration authority

After the RED evidence above became terminal, the lane restored these frozen source blobs from `fb30fba2a888835dfc7cbde27f940b79d7bfe05d`:

```text
apps/game-server/build.rs
  3a8149ef075f6896a7435c716cb8a4de5d94606b
apps/game-server/migrations/0001_admission_reconnect_journal.sql
  52aa1931550df3be6ab97d8b5a6814559f4ae494
apps/game-server/src/bin/oteryn-game-migrate.rs
  80e72fcdeeb70359986a5f93fe287362c0d205a1
apps/game-server/src/durability/admission_journal.rs
  336fbf4ed5f2cd740ab954261b924011030c272d
apps/game-server/src/durability/db.rs
  48746007625646dee9d8a44972005cacb2a97c73
apps/game-server/src/durability/mod.rs
  f37fd5e1d8ae50e8b71391a85da73369ac25fcb5
apps/game-server/src/durability/schema.rs
  8c92e301bd420a386f8684025ba429903b1b6e91
```

The restoration copied exact file contents only; no #212 commit was cherry-picked or merged. The exact RED-head -> GREEN-head linkage and focused GREEN proof are recorded above.

## Required successor-owned technical proofs

The GREEN successor must independently preserve the material reconnect regressions previously discovered on #212, including:

- committed PREPARE replay after process restart routes to same-attempt reconciliation;
- expired PREPARED replay requires exact incumbent binding;
- fast-reconnect committed-winner reconstruction rejects missing/zero proof generation;
- durable actor/scope binding fails closed on disagreement;
- full-range `u64` authority fences and CommandIds persist losslessly;
- typed epoch/transport mirrors match the canonical record;
- a new loss epoch requires a valid canonical committed current controller;
- PREPARE/COMMIT deadline freshness uses trusted post-lock database time;
- migration interruption proves cancellation plus a distinct fresh retry;
- RecoveryGrantNonce remains durably single-consumed where required.

## Excluded scope

No Foundation path/semantic mutation, Server Seam, Cargo/workspace/lockfile, workflow, registry/stable ID, composition root, public architecture decision, production database/config/secrets, live account/session/player data, Platform/Atlas/META or external-repository mutation.

An unowned path is `SHARED_LEASE_REQUIRED`. A required change to persistence/trust/public-contract semantics beyond accepted #167 authority is `ARCHITECTURE_ESCALATION_REQUIRED`.

## Required exact-head qualification

1. The RED/GREEN PostgreSQL 17 Durability harness chain is proven above on immutable pre-finalization heads.
2. Run task-finalization local governance/repository validations and changed-path verification on this update.
3. Freeze and bind the resulting exact final head in immutable PR #243 evidence rather than a self-referential commit.
4. Complete mandatory whole-diff self-review.
5. Current protected-main `CODEX_REVIEW_POLICY.json` routes this durability/reconnect candidate to `CODEX_REQUIRED`; the allocated Sol Durability Lead owns the strict read-only exact-head Codex loop.
6. Observe fresh required repository CI on the exact final head, with zero unresolved P0/P1/P2 findings and required review threads.
7. Return `READY_FOR_INTEGRATION` to the Work control plane only after every gate is proven. No self-merge.

## Context checkpoint

```yaml
status: QUALIFICATION_PENDING
branch: impl/game-durability-journal-recovery-240
pr: 243
head_sha: 783ec1dfa8af6791e5ad8d06f13f32a6e62b985a
final_head_sha: null
red_head_sha: 07a724db929dc1aa46556177b81a2b36f91238a2
red_evidence: PR #243 comment 5453275715; trusted run/job 33176654398/98866989164; expected missing src/durability/mod.rs failure
green_head_sha: 20741682ab71fc50da53c82f827ae144712baec0
green_evidence: PR #243 comment 5453327626; trusted run/job 33176930382/98867863053; durability_postgres 44/44 PASS
post_qualification_p1_red_heads: 0129e9c4576894fd5b3d9184ea21afea97c1d204 (same-epoch cross-session) and 48dac03fecce273836776ae434e4b0a913c6be18 (later-epoch cross-session)
post_qualification_p1_green_head: 783ec1dfa8af6791e5ad8d06f13f32a6e62b985a
post_qualification_p1_green_evidence: Rust workspace run 33182999525 and PostgreSQL job 98888721229 PASS
post_qualification_p1_p2_red_head: 7f792ba98d04818cc2e73904a5444b99e922dab6
post_qualification_p1_p2_red_evidence: Rust workspace run 33184342450 and PostgreSQL job 98893334040; exact focused replay-reservation and compatibility-anchor assertions failed
post_qualification_p1_p2_green_head: 4e8760edb792cdc241f430420ac93759b155007a
post_qualification_p1_p2_green_evidence: Rust workspace run 33184628524 and PostgreSQL job 98894312224 PASS
exact_head_evidence: bind the post-task-update exact final head in fresh immutable PR #243 self-review, Codex review, and required CI evidence
allocation_pr: 241
allocation_merge_sha: a171410de07c2dab718f52f780d4314bdcc53604
blocker: fresh_exact_head_self_review_codex_required_review_and_required_repository_ci_pending_after_truthful_post_p1_p2_task_finalization
owner_action_required: null
next_action: publish this task-only post-P1/P2 qualification update, then complete and durably record fresh exact-head whole-diff self-review, required strict read-only Codex review, and required repository CI before any READY_FOR_INTEGRATION decision
```
