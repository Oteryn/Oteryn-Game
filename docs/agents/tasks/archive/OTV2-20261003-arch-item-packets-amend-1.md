# OTV2-20261003-arch-item-packets-amend-1

```yaml
task_id: OTV2-20261003-arch-item-packets-amend-1
title: "ARCH-ITEM-PACKETS-AMEND-1: capability 6 before capability 4, the ground-speed seam, slot-to-Ground host call sites"
mode: CONTRACT
status: completed
repository: Oteryn/Oteryn-Game
base_branch: main
branch: claude/arch-item-packets-amend-1
issue: 1622
pr: 1702
head_sha: "exact frozen head in the #1622 FREEZE_SHA entry"
final_head_sha: "exact frozen head in the #1622 FREEZE_SHA entry"
owner: claude-code-session_01YL1cQaLL3BquJajKivZVhw (Sol Supervising Architect)
created_at: 2026-10-03
updated_at: 2026-10-03
execution_policy: continuous_progress
owned_paths:
  - docs/architecture/reviews/OTERYN_GAME_ARCH_BATCH_ITEM_EQUIP_PACKETS_2026-10-03.md
  - docs/architecture/reviews/OTERYN_GAME_ARCH_BATCH_TIMED_FORGE_PROF_PACKETS_2026-10-03.md
  - docs/architecture/ADR-0021-world-map-runtime-loading.md
  - docs/agents/tasks/archive/OTV2-20261003-arch-item-packets-amend-1.md
public_contracts: []
depends_on: []
blocks: [ITEM-MOVE-1, SPEED-1, BAGS-1, ITEM-MOVE-2b]
cross_repository_coordination_id: null
external_repositories: []
```

## Outcome

- **#1698 P1 4175197224:** capability 4 requires capability 6. The item batch packets NPC-VIS-1
  (kind 5 `Npc`, after NPC-BEHAVIOUR-0 is accepted) and VIS-3 (domain 1 from the VIS-1 interest
  set, offers capability 6, after NPC-VIS-1 and CAP-NEG-1), and ITEM-MOVE-1 waits for VIS-3
  (§1.10, §2.0a, §2.0b, §2.6). NPC-BEHAVIOUR-0 acceptance is now on the loot chain's critical path.
- **#1698 P2 4175197230:** SPEED-1 reads ground speed from a lookup seam (150 on the engineering
  map, injected non-default test); MAP-LOAD-1 supplies the production source with a non-default
  production-path test as a merge condition (§1.11, §2.3, ADR-0021 amendment).
- **#1696 P1 4175041166:** the slot-to-Ground drop call sites, success and rejection, go to
  whichever of TIMED-RT-1b and ITEM-MOVE-2b merges second; ordering 2b after RT-1c is circular
  (item batch §1.7, §2.13; TIMED batch §1.1, §2.3).
- No code, contract or wire change.
