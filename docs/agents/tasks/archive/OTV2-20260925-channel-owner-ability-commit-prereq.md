# OTV2-20260925-channel-owner-ability-commit-prereq

```yaml
task_id: OTV2-20260925-channel-owner-ability-commit-prereq
title: Owner-mediated one-creature Ability commit prerequisite
mode: IMPLEMENT
status: implementing
repository: Oteryn/Oteryn-Game
base_branch: main
branch: agent/otv2-channel-owner-ability-commit-prereq
issue: 162
pr: null
base_sha: bfc8b54548a09c59e062873a5dfa48739c477420
head_sha: null
final_head_sha: null
owner: Sol 6
created_at: 2026-09-25T00:00:00Z
execution_policy: continuous_progress
owned_paths:
  - apps/game-server/src/foundation/runtime_actor_carrier.rs
  - apps/game-server/src/foundation/mod.rs
  - apps/game-server/src/ability/commit.rs
  - apps/game-server/src/ability/mod.rs
  - apps/game-server/src/lib.rs
  - apps/game-server/src/foundation/channel_owner_ability_commit_tests.rs
  - apps/game-server/tests/ability_engine.rs
  - docs/agents/tasks/active/OTV2-20260925-channel-owner-ability-commit-prereq.md
  - docs/agents/evidence/OTV2-20260925-channel-owner-ability-commit-prereq.md
```

Allocation: #162 comments 5828383421, 5828386989 and nine-path test amendment 5828450388. This branch adds a fixed one-creature HP slot in the existing Channel owner carrier. A typed one-target, one-damage, atomic Ability plan reaches that slot through a resolved exact actor and owner-mediated commit. The carrier's one bounded receipt contains the full canonical plan bytes and the resulting HP transition; replay does not apply again. This is unactivated, nonshipping fixture composition. No death occurrence, corpse, production owner capacity/composition, SQL, wire or value authority is introduced.

Validation and exact-head review are coordinator-owned after this writer's authoring freeze. See the matching evidence packet for tests and limits.
