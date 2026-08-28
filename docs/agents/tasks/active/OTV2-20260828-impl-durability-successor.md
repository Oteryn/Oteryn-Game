# OTV2-20260828-impl-durability-successor

```yaml
task_id: OTV2-20260828-impl-durability-successor
title: Recover and complete journal-only Durability on clean history
mode: IMPLEMENT
status: WAITING_ALLOCATION_MERGE
integration_state: BLOCKED_PROVENANCE_RECOVERY_ALLOCATION
repository: Oteryn/Oteryn-Game
base_branch: main
branch: impl/game-durability-journal-recovery-240
issue: 167
recovery_issue: 240
parent_coordinator_issue: 162
historical_pr: 212
pr: null
owner: Oteryn: sol durability lead
allocation_branch: coord/durability-provenance-recovery-240
allocation_pr: null
allocation_merge_sha: null
admission_main_sha: 7c2da078596a7d2e27c3066ff74ac69b8b7f9af6
source_snapshot_pr: 212
source_snapshot_head_at_allocation: fb30fba2a888835dfc7cbde27f940b79d7bfe05d
source_snapshot_mode: read_only_file_content_only_no_commit_inheritance
write_authority: none_until_recovery_allocation_merge
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
```

## Start gate

This task is deliberately non-mutating until the recovery allocation PR created for Issue #240 is merged and its merge SHA is read back from protected `main`.

After that merge, the Work coordinator records the exact successor base from current protected main and creates `impl/game-durability-journal-recovery-240` from that SHA. The worker must not branch from, merge from, cherry-pick from, rebase onto, reset to, or force-update the historical `impl/game-durability-journal` branch.

## Historical evidence boundary

PR #212 and its branch are evidence only. The successor may read file contents from a frozen historical #212 head to avoid discarding legitimate implementation work, but every transferred path must be in the owned-path allowlist above. Historical reviews, CI and test counts are context only and never qualify this successor.

The successor must independently reproduce or close all material live technical findings discovered on #212. At minimum the PostgreSQL regression suite must preserve proofs for:

- committed PREPARE replay after process restart routes to same-attempt reconciliation;
- expired PREPARED replay requires exact incumbent binding before terminalization/release;
- fast-reconnect committed-winner reconstruction rejects missing/zero proof generation;
- durable session actor/scope binding fails closed on disagreement;
- full-range `u64` authority fences and CommandIds persist losslessly;
- typed epoch/transport mirrors match the canonical record;
- new loss epoch requires a valid canonical committed current controller;
- row-lock contention uses trusted post-lock database time for PREPARE/COMMIT deadlines;
- migration cancellation proves a fresh retry rather than continuing the same future;
- RecoveryGrantNonce remains durably single-consumed where the proof class requires it.

## Excluded scope

No Foundation semantics or paths, Server Seam, Cargo/workspace/lockfile, workflow, registry/stable IDs, composition roots, public architecture decisions, production database/config/secrets, live account/session/player data, Platform/Atlas/META or external-repository mutation.

Any need for an unowned path is `SHARED_LEASE_REQUIRED`; any need to change persistence/trust/public contract semantics beyond the accepted #167 contract is `ARCHITECTURE_ESCALATION_REQUIRED`.

## Required validation

1. Reconstruct exact allowed file snapshots on clean successor ancestry.
2. Run the isolated PostgreSQL 17 Durability harness and preserve exact pass/fail evidence.
3. Run Rust 1.94 formatting and strict Clippy for affected targets plus applicable game-server/package/workspace tests.
4. Verify changed paths are exactly within the successor allowlist.
5. Freeze exact final head after implementation/task metadata are complete.
6. Perform mandatory whole-diff self-review.
7. Resolve current protected-main `CODEX_REVIEW_POLICY.json`; when `CODEX_REQUIRED`, the Durability lane lead requests the strict read-only exact-head review under standing authorization.
8. Require zero unresolved P0/P1/P2 findings and required review threads.
9. Require exact-head repository CI on the unchanged final head.
10. Return `READY_FOR_INTEGRATION` only after all gates are proven; no self-merge.

## Context checkpoint

```yaml
status: WAITING_ALLOCATION_MERGE
branch: impl/game-durability-journal-recovery-240
head_sha: null
final_head_sha: null
pr: null
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
blocker: recovery_allocation_pr_not_merged
owner_action_required: null
next_action: after allocation merge, create the successor branch from the allocation-recorded protected-main SHA and reconstruct only allocated file snapshots
```
