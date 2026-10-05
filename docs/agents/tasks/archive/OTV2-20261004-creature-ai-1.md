# OTV2-20261004-creature-ai-1

```yaml
task_id: OTV2-20261004-creature-ai-1
title: "CREATURE-AI-1: creature targeting, attacks, defences and the think budget"
mode: IMPLEMENTATION
status: completed
repository: Oteryn/Oteryn-Game
base_branch: main
branch: claude/creature-ai-1-20261004
issue: 1622
pr: 1813
head_sha: "exact frozen head in the #1622 FREEZE_SHA entry"
final_head_sha: "exact frozen head in the #1622 FREEZE_SHA entry"
owner: claude-code-session_01Bs4uYHwtH7pba1QSCSoKCN (worker of control plane session_013KJX6mv8LQveCKKXYgAX94)
created_at: 2026-10-05
updated_at: 2026-10-05
execution_policy: continuous_progress
owned_paths:
  - apps/game-server/src/ai_think.rs
  - apps/game-server/src/ai_think/**
  - apps/game-server/src/ai/mod.rs
  - apps/game-server/src/ai/perception.rs
  - apps/game-server/src/ai/resolution.rs
  - apps/game-server/src/ai/snapshot.rs
  - apps/game-server/src/ai/tests.rs
  - apps/game-server/src/ai/behaviour_profile.rs
  - apps/game-server/src/ai/targeting.rs
  - apps/game-server/src/foundation/runtime_actor_carrier.rs
  - apps/game-server/src/foundation/channel_owner_creature_ai_tests.rs
  - apps/game-server/src/foundation/mod.rs
  - apps/game-server/src/ability/creature_bite.rs
  - apps/game-server/src/ai_monster_melee.rs
  - apps/game-server/src/gameplay_transport/monster_ai_cycle.rs
  - apps/game-server/src/gameplay_transport/mod.rs  # HOLD_STEP line only (CP D709)
  - docs/contracts/RESOURCE_LIMITS_REGISTRY.json
  - docs/agents/tasks/archive/OTV2-20261004-creature-ai-1.md
public_contracts: []
depends_on: [CREATURE-AI-1-PACKET-1, ATTACK-1b]
blocks: [CREATURE-MOVE-1, SPAWN-1a, KILL-REWARD-COMP-1]
cross_repository_coordination_id: null
external_repositories: []
```

## Scope

Packet `docs/architecture/reviews/OTERYN_GAME_CREATURE_AI1_TARGETING_AND_THINK_PACKET_2026-10-04.md`
§0, §1 and §2.1.

## Lease extension

CP decision D709 (option a) extends owned_paths with `ai_monster_melee.rs`,
`gameplay_transport/monster_ai_cycle.rs` and the `HOLD_STEP` line of
`gameplay_transport/mod.rs` only. The module line and 40-byte target resolution of `mod.rs`
(MAP-WIRE-2) and `native_combat_cast.rs` (SPELL-TARGET-1) are not touched.

## Validation

- `python tools/agents/validate_governance.py`: pass
- `python -m unittest discover -s tools/agents/tests`: pass
- `validate_repository_policy`: pass
- `cargo fmt --all --check`: pass
- `cargo clippy --workspace --all-targets --locked -- -D warnings`: pass
- `cargo test --locked -p oteryn-game-server` (24423 passed, 0 failed): pass
- `git diff --check`: pass
