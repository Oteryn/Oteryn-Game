# OTV2-20261004-chest-place-bind-1

```yaml
task_id: OTV2-20261004-chest-place-bind-1
title: "CHEST-PLACE-BIND-1: bind RewardClaim placements to world bundle entries"
mode: IMPLEMENT
status: completed
repository: Oteryn/Oteryn-Game
base_branch: main
branch: OTV2-20261004-chest-place-bind-1
issue: 1622
pr: 1806
owner: claude-code worker for control plane session_013KJX6mv8LQveCKKXYgAX94
created_at: 2026-10-05
updated_at: 2026-10-05
execution_policy: continuous_progress
owned_paths:
  - apps/game-server/src/content/world_reward_claims.rs
  - apps/game-server/src/content/mod.rs
  - apps/game-server/src/map/mod.rs
  - apps/game-server/tests/world_reward_claims_*.rs
  - docs/agents/tasks/archive/OTV2-20261004-chest-place-bind-1.md
public_contracts: []
depends_on: [MAP-LOAD-1]
blocks: [WORLD-CONTENT-SERVE-1]
cross_repository_coordination_id: null
external_repositories: []
```

## Scope

ARCH-WORLD-CONTENT-SERVE-1 §1.4-§1.6 and §2.3 on `main` (839432f8): the sparse top-level
`unique` table of `WorldBase` with `TileView::unique`, and the pure binder in
`content/world_reward_claims.rs`. Not called from the node.

## Decisions

- Admitted Item registry is a `&dyn Fn(&TypedDefinitionRef) -> bool` input.
- `achievement_grant` is not encoded by this packet: a record carrying one is left out as `UnencodedField`; `achievement` is always `None`. For the control plane to confirm.
- `unique` table: `(u32, u16, Option<u16>)` per top-level entry with a `unique`, 12 bytes each, so about 12 bytes times the number of such entries in the bundle (a few thousand uniques is tens of KiB). Not measured on the reference bundle: none is on main (`content/world/pins` absent).
- Expected `NO_ENTRY` on the non-production pin (provisional donor chests, no bundle entry): `oteryn:reward-claim.quest.u15_24.targuna.mana_potions_chest` (28827) and `oteryn:reward-claim.quest.u11_80.the_secret_library.small_islands.parchment` (28828). Covered by a fixture test; no real-bundle test.

## Validation

- `cargo fmt --all -- --check`: pass
- `cargo clippy --locked -p oteryn-game-server --all-targets -- -D warnings`: pass
- `cargo test --locked -p oteryn-game-server world_reward_claims`: pass (11 tests)
- `cargo test --locked -p oteryn-game-server map`: pass
- `cargo run --locked -p oteryn-architecture-check -- workspace .`: pass
- `python3 tools/agents/validate_governance.py`: pass
- `python -m unittest discover -s tools/agents/tests`: OK
- `git diff --check`: pass

## Merge

squash merge of #1806
