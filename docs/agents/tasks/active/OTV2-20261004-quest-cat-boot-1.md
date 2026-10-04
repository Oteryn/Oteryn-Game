# OTV2-20261004-quest-cat-boot-1

```yaml
task_id: OTV2-20261004-quest-cat-boot-1
title: "QUEST-CAT-BOOT-1 load the quest state catalogue at boot and hand it to gameplay"
mode: IMPLEMENT
status: blocked
repository: Oteryn/Oteryn-Game
issue: 1622
lane_id: quest
base_branch: main
branch: agent/quest-cat-boot-1-20261004
pr: PENDING
base_sha: 673f092e
head_sha: "exact frozen head in the FREEZE_SHA report to the control plane"
owner: claude-code-session-01FCqRT24fsJjjVLs1H3F9nu (worker)
control_plane: claude-code-session-013KJX6mv8LQveCKKXYgAX94
created_at: 2026-10-04
updated_at: 2026-10-04
packet: "docs/architecture/reviews/OTERYN_GAME_ARCH_BATCH_QUEST_WIRING_PACKETS_2026-10-04.md §2.1 QUEST-CAT-BOOT-1"
owned_paths:
  - apps/game-server/src/node/serve.rs
  - apps/game-server/src/gameplay_transport/mod.rs
  - apps/game-server/src/quest/mod.rs
  - apps/game-server/src/quest/loader.rs
  - apps/game-server/src/gameplay_transport/quest_catalogue_boot_tests.rs
  - docs/agents/tasks/active/OTV2-20261004-quest-cat-boot-1.md
  - docs/agents/tasks/archive/OTV2-20261004-quest-cat-boot-1.md
public_contracts: []
external_repositories: []
```

## Blockers

- §1.2 names `content::accepted::REVISIONS[0]` ("oteryn:content/entry-r1") as the catalogue revision,
  but the quest catalogue and Character progression both reject `/` in a revision, so boot would
  always refuse. Recommended: load at `config.readiness.content_revision`. Awaiting control plane.
- `gameplay_transport/qualification.rs:1561` builds `GameplaySeamOwners` under `cfg(test)` and is
  outside owned_paths. Awaiting control plane.
- Optional Postgres acceptance case in `tests/support/quest_state_postgres_cases.rs`. Awaiting control plane.
