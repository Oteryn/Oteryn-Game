# OTV2-20260828-impl-durability-successor

```yaml
task_id: OTV2-20260828-impl-durability-successor
title: Clean-history Durability recovery successor
mode: IMPLEMENT
status: TDD_RED_PENDING
integration_state: RED_EVIDENCE_PENDING
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
exact_current_head_evidence: immutable PR #243/check evidence after this task-binding commit; a commit cannot contain its own SHA
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

## Mandatory RED gate

The current candidate is intentionally incomplete. Before any production Durability bytes may be added:

1. PR #243 must remain Draft.
2. Its exact current test-only head must be recorded in immutable PR/check evidence.
3. Trusted `Rust / Durability PostgreSQL harness` must execute, not skip.
4. The focused command must **FAIL** because the production Durability module is deliberately absent on clean protected-main ancestry.
5. The failing run/job/head and first relevant compiler/test divergence must be recorded durably on PR #243 or Issue #240.
6. A historical #212 result, a skipped job, or a different failure class cannot satisfy this RED gate.

Frozen RED blobs:

```text
apps/game-server/tests/durability_postgres.rs
  460ad5888d8e870bbeda50a3dc8f64b24a30c1cb
apps/game-server/tests/support/postgres.rs
  bcb243f6c4823a14ec8116b72439c2c79c115d94
```

## GREEN gate — forbidden until RED is proven

Only after the exact RED evidence above is terminal may the lane add these frozen source blobs from `fb30fba2a888835dfc7cbde27f940b79d7bfe05d`:

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

Copy exact blobs/file contents only; do not cherry-pick or merge any #212 commit. Re-run the same PostgreSQL target and require GREEN, preserving exact RED-head -> GREEN-head linkage.

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

## Required validation after GREEN

1. Isolated PostgreSQL 17 Durability harness with exact pass/fail evidence.
2. Rust 1.94 formatting and strict Clippy for affected targets.
3. Applicable game-server/package/workspace tests and exact repository Merge gate.
4. Changed-path verification against the exact successor allowlist.
5. Candidate freeze after implementation/task metadata are complete; exact final head recorded in immutable PR/check evidence rather than a self-referential commit.
6. Mandatory whole-diff self-review.
7. Current protected-main `CODEX_REVIEW_POLICY.json`; when `CODEX_REQUIRED`, the allocated Sol Durability Lead owns the strict read-only exact-head Codex loop.
8. Zero unresolved P0/P1/P2 findings and required review threads.
9. Return `READY_FOR_INTEGRATION` to the Work control plane only after all gates are proven. No self-merge.

## Context checkpoint

```yaml
status: TDD_RED_PENDING
branch: impl/game-durability-journal-recovery-240
pr: 243
head_sha: null
final_head_sha: null
exact_head_evidence: bind the post-task-update exact head in immutable PR #243/check evidence
allocation_pr: 241
allocation_merge_sha: a171410de07c2dab718f52f780d4314bdcc53604
blocker: expected_successor_red_not_yet_proven
owner_action_required: null
next_action: observe the trusted PostgreSQL 17.6 Durability harness on the exact test-only PR #243 head and preserve the expected missing-production-module RED before any implementation blob restore
```
