# WHEEL-W1

```yaml
task_id: WHEEL-W1
title: "WHEEL-W1 Wheel of Destiny allocation storage, writer, admission reset, load and stages"
mode: IMPLEMENT
status: implementing
repository: Oteryn/Oteryn-Game
issue: 1622
lane_id: wheel
base_branch: main
branch: claude/wheel-w1
pr: PENDING
base_sha: bac15d8
head_sha: "exact frozen head in the FREEZE_SHA report to the control plane"
final_head_sha: "exact frozen head in the FREEZE_SHA report to the control plane"
owner: claude-code-session-0184G9SKwaLcDGfZ6JnXvERc (oteryn-hard-worker)
control_plane: claude-code-session-013KJX6mv8LQveCKKXYgAX94
created_at: 2026-10-04
updated_at: 2026-10-04
packet: "docs/architecture/reviews/OTERYN_GAME_WHEEL0_WHEEL_OF_DESTINY_DELIVERY_DECISION_2026-09-30.md child W-1 (§4, §8 rows, §13); OTERYN_WHEEL_OF_DESTINY_STATE_CONTRACT_CANDIDATE_V1.md §3.2-§3.4; owner decision D483; control plane decision D484 (W-R runtime carry)"
leases: migration 0070 (control plane lease)
owned_paths:
  - apps/game-server/migrations/0070_character_wheel_allocation.sql
  - apps/game-server/src/durability/character_wheel.rs
  - apps/game-server/src/durability/mod.rs
  - apps/game-server/src/durability/character_revision_sequencer.rs
  - apps/game-server/src/wheel_gem_data.rs
  - apps/game-server/src/gameplay_transport/mod.rs
  - apps/game-server/tests/character_authority_postgres.rs
  - apps/game-server/tests/support/character_wheel_postgres_cases.rs
  - docs/contracts/RESOURCE_LIMITS_REGISTRY.json
  - docs/agents/tasks/archive/WHEEL-W1.md
depends_on:
  - "W-R Wheel data on main (#1697), data-only; its runtime revision piece is carried here (D484)"
  - "CHAR-REV-SEQ-1 merged (#1663)"
public_contracts: []
external_repositories: []
```

## Rows

WHEEL-0 §8 registered in `docs/contracts/RESOURCE_LIMITS_REGISTRY.json` before implementation:
`WHEEL0-RL-01`..`06`, plus the two unnumbered rows as `WHEEL0-RL-07-CHANGE` (allocation change:
0 items, 0 value lines, 1 CharacterRevision, 1 receipt) and `WHEEL0-RL-08-RESET` (Wheel reset:
the same, once per Character per reset revision).
