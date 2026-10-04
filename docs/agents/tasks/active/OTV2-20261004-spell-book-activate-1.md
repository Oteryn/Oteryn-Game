# OTV2-20261004-spell-book-activate-1

```yaml
task_id: OTV2-20261004-spell-book-activate-1
title: "OTV2-20261004-spell-book-activate-1: whole-book sweep and qualification default"
mode: IMPLEMENT
status: implementing
repository: Oteryn/Oteryn-Game
issue: 1622
base_branch: main
branch: claude/spell-book-activate-1-20261004
pr: 0
base_sha: 673f092e
control_plane: claude-code-session-013KJX6mv8LQveCKKXYgAX94
created_at: 2026-10-04
updated_at: 2026-10-04
execution_policy: continuous_progress
owned_paths:
  - apps/game-server/src/gameplay_transport/spell_book_sweep_tests.rs
  - apps/game-server/src/gameplay_transport/qualification.rs  # one cfg(test) module line
  - apps/game-server/src/gameplay_transport/native_combat_cast.rs  # section 1.2 refusal fix only
  - apps/game-server/src/gameplay_transport/stance_cast.rs  # section 1.2 refusal fix only
  - apps/game-server/src/gameplay_transport/familiar_cast_dispatch.rs  # section 1.2 refusal fix only
  - apps/game-server/src/gameplay_transport/native_companion_item_cast.rs  # section 1.2 refusal fix only
  - apps/game-server/src/gameplay_transport/native_world_item_cast.rs  # section 1.2 refusal fix only
  - tools/qualification/node_boot/run.sh
  - tools/qualification/spells/README.md
  - tools/content-schema/native-gameplay/README.md
  - content/abilities/SPELL-IMPORT.md
  - docs/agents/tasks/archive/OTV2-20261004-spell-book-activate-1.md
public_contracts: []
depends_on: []
```

## Outcome

In progress. Sweep counts and results are recorded below when the candidate is frozen.

## Deviations

- The sweep module sits under `gameplay_transport::qualification` because `mod.rs` is leased to
  ATTACK-1b, so the packet filter `gameplay_transport::spell_book_sweep_tests` cannot match; the
  filter used is `spell_book_sweep_tests`.
- `cast_spell_completion` needs Postgres and Platform authority, so the sweep is DB-free over the
  pure engine `prepare_*` functions plus dispatch classification (agreed with the control plane).
- `tools/qualification/node_boot/README.md` lines 17-18 still describe the old r21 default (outside
  owned paths); `wp5_s3b/run.sh` also still defaults to r21.

## Validation

(filled at freeze)
