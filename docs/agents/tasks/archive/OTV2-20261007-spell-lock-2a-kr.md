# OTV2-20261007-spell-lock-2a-kr

```yaml
task_id: OTV2-20261007-spell-lock-2a-kr
title: SPELL-LOCK-2a lane permit for main's kill-reward settle and corpse-entry take
mode: IMPLEMENT
status: completed
repository: Oteryn/Oteryn-Game
base_branch: claude/spell-lock-2a-20261005
branch: claude/spell-lock-2a-kr-20261007
pr: null
base_sha: f1e68380751713c2d49636e976315626d2433e7e
head_sha: "exact frozen head in the FREEZE_SHA report to the control plane"
final_head_sha: "exact frozen head in the FREEZE_SHA report to the control plane"
final_head_frozen_at: null
owner: oteryn-hard-worker session_01ARgwFxy96wwU3MEiSbVPCd
created_at: 2026-10-07T00:00:00Z
updated_at: 2026-10-07T00:00:00Z
execution_policy: continuous_progress
owned_paths:
  - apps/game-server/src/gameplay_transport/kill_reward.rs  # DurableKillSettle::settle only (D905, owner-approved)
  - apps/game-server/src/gameplay_transport/kill_reward_tests.rs  # only if the settle change needs it (unchanged)
  - apps/game-server/src/gameplay_transport/item_ref_admission.rs  # take_corpse_entry only (owner-approved)
  - docs/agents/tasks/archive/OTV2-20261007-spell-lock-2a-kr.md
public_contracts: []
depends_on: [OTV2-20261005-spell-lock-2a]
blocks: [OTV2-20261005-spell-lock-2a]
cross_repository_coordination_id: null
external_repositories: []
```

## Scope

Stacked on #1907 after its merge of origin/main (f1e68380). Main added two key-33 writer calls
without the `SpellLanePermit` that SPELL-LOCK-2a requires; both stay outside #1907's owned_paths
(D884 declined), so they land here (D905):

- `DurableKillSettle::settle`: the lane is taken before `revision_sequencer.acquire`
  (SPELL-LOCK-2 §1.2 lane-first order, extending ARCH-KILL-REWARD-LOGOUT-1 §1.2); `None` returns
  `SettleVerdict::Retry` with no side effect; `&permit` is the first argument of
  `settle_creature_death_rewards_with_bestiary`. The caller holds no Channel guard.
- `take_corpse_entry`: the lane is taken before the runtime and spell-state guards; `None` returns
  `ItemTransferError::Unavailable`, which the item-move caller already treats as retryable
  disconnect; `&permit` is the first argument of `commit_item_transfer`. The callers hold no
  Channel guard.

No permit is held across a channel await other than the guarded writer's own work.

## Validation

`cargo fmt --check`, `cargo clippy -p oteryn-game-server --all-targets -D warnings` and the
kill-reward and item-move unit tests on the branch head; CI `game-gate` on the PR.
