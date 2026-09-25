# OTV2-20260925-ops-node-boot-followups

```yaml
task_id: OTV2-20260925-ops-node-boot-followups
title: OPS-NODE-BOOT-01 remaining follow-ups from #832
mode: IMPLEMENT
status: implementing
repository: Oteryn/Oteryn-Game
base_branch: main
branch: claude/zen-dirac-mxruho
issue: 832
pr: null
base_sha: null
head_sha: null
final_head_sha: null
final_head_frozen_at: null
owner: claude
created_at: 2026-09-25T00:00:00Z
updated_at: 2026-09-25T00:00:00Z
execution_policy: continuous_progress
owned_paths:
  - apps/game-server/migrations/0007_character_fresh_admission_definer.sql
  - apps/game-server/src/durability/character_authority.rs
  - apps/game-server/src/character_recovery_fence.rs
  - apps/game-server/src/node/secure_file.rs
  - apps/game-server/src/node/serve.rs
  - apps/game-server/src/bin/oteryn-game-ops.rs
  - apps/game-server/tests/runtime_scope_assignment_postgres.rs
  - docs/agents/tasks/active/OTV2-20260925-ops-node-boot-followups.md
public_contracts:
  - OPS-NODE-BOOT-01
depends_on:
  - pr:872
blocks: []
cross_repository_coordination_id: null
external_repositories: []
```

- **Governing decision:** `OPS-NODE-BOOT-01` (#830), D1–D3. This PR closes the remaining #832 follow-up checklist.
- **Scope:**
  - **Admission definer.** The control role admits only generation one of an empty Character store, through the definer `game_character_admit_fresh_recovery` (migration 0007). Its direct `INSERT` on recovery admissions is revoked.
  - **Retained-file owner.** `oteryn-game-ops` takes a retained file's owner from the opened descriptor instead of a separate path lookup, and a missing file is distinguished from a refused one.
  - **Fence directory.** The Character recovery fence keeps its validated directory descriptor. It opens, creates, renames and synchronizes every entry relative to that descriptor, and checks the parent of the opened directory.
  - **Shutdown drain.** After readiness is withdrawn, the drain lets an in-flight admission or bootstrap reach its own deadline.
- **Validation:** tests for the control role (direct insert refused, definer admission and replay), the descriptor-owner read, directory pinning and the drain budget.
- **Excluded:** successor recovery admission tooling, which stays an owner-privileged recovery operation. Also excluded: Platform or production changes.
