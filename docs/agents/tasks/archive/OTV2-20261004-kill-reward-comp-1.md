# OTV2-20261004-kill-reward-comp-1

```yaml
task_id: OTV2-20261004-kill-reward-comp-1
title: "KILL-REWARD-COMP-1: kill reward composition (Parts A, C1, C2, B)"
mode: IMPLEMENT
status: completed
repository: Oteryn/Oteryn-Game
issue: 1622
base_branch: main
branch: claude/kill-reward-comp-1-20261006-b
pr: 1923
base_sha: eece95e5
owner: claude-code-session-013SfhK4MaNF94CeQgfGhiu6
control_plane: claude-code-session-0114oBVR3osF1auvFMu6ksMH
created_at: 2026-10-04
updated_at: 2026-10-07
execution_policy: continuous_progress
packet: docs/architecture/reviews/OTERYN_GAME_ARCH_KILL_REWARD_LOGOUT_PACKETS_2026-10-04.md
owned_paths:
  - apps/game-server/src/content/creature_reward.rs
  - apps/game-server/src/content/creature_reward_tests.rs
  - apps/game-server/src/content/native_gameplay.rs
  - apps/game-server/src/content/mod.rs
  - apps/game-server/src/combat.rs
  - apps/game-server/src/combat/death_reward.rs
  - apps/game-server/src/gameplay_transport/kill_reward.rs
  - apps/game-server/src/gameplay_transport/kill_reward_tests.rs
  - apps/game-server/src/gameplay_transport/spell_timer_callbacks_tests.rs
  - apps/game-server/src/gameplay_transport/mod.rs
  - apps/game-server/tests/support/combat_death_reward_postgres_cases.rs
  - apps/game-server/tests/support/kill_reward_live_postgres_cases.rs
  - apps/game-server/tests/combat_death_reward_postgres.rs
  - docs/contracts/RESOURCE_LIMITS_REGISTRY.json
  - tools/content-schema/native-gameplay/**
  - tools/qualification/node_boot/**
  - content/spells.manifest.json (loot_tables pin line only, CP D929)
  - docs/agents/tasks/archive/OTV2-20261004-kill-reward-comp-1.md
public_contracts: []
prs: [1848, 1904, 1908, 1923]
```

## Outcome

Parts A (#1848), C1 (#1904) and C2 (#1908) are merged. Part B (#1923) is the last PR. It adds:

- The framed native section `OTERYN_NATIVE_LOOT_TABLES/v1`: layout `MAGIC_V8`, which is V7 plus `sections[11]`. It holds `creature_loot` bindings, loot tables and an `items` admission-facts list.
- The producer and its tests.
- node_boot staging.
- The production pin `tools/content-schema/native-gameplay/loot-tables.json`: 1870 bindings, 1290 tables, 3359 Item facts.
- The fail-closed `CreatureRewardTable` builder. Its rows refuse with `no_loot_binding`, `loot_table_missing`, `corpse_item_missing`, `corpse_item_inadmissible`, `loot_item_inadmissible` or `xp_out_of_range`.
- `reward_table()` wired to the active generation.
- Optional loot: a null binding mints the corpse with an empty table, and `loot_table_ref` is the corpse ref.
- The live PostgreSQL rat case.

## Control plane decisions (D929)

1. Option (b). The live case uses the real rat profile (XP 5, corpse `oteryn:item.tibia.i5964`, capacity 16) with an admitted-only table: gold `i3031`. The production rat row is asserted as a refusal with `loot_item_inadmissible`, because cheese `i3607` is not materializable. Its admission is OTV2-20261007-d3-8-cheese.
2. The loot section carries an `items` admission-facts list. The producer copies it from `content/items/definitions` as the single source, and the builder reads only that list.
3. Option (d). Owned paths are extended to the loot_tables pin line in `content/spells.manifest.json`.

## Notes

- **D3-7 alias:** `i00005801` needs no resolution. The pinned rat profile already names `oteryn:item.tibia.i5964` as its corpse.
- **Compiler-forced edits outside owned paths:** `loot_tables: None` in the struct literals of `movement/source_floor_change.rs`, `monster_combat_lane.rs` (2), `spell/owned_cast_facts.rs` and `foundation/channel_owner_auto_attack_tests.rs`.
- **`combat.rs` allows retained:** the packet removes only the attributes "that the live caller makes unnecessary". The live caller uses part of the re-exports, and `foundation/mod.rs` recompiles `combat.rs` standalone with none of them used. Removing the attributes fails the lib, the lib test and that recompile. The comment and `reason` now state this.
- **Live case placement:** the live case is the `kill_reward_live` module inside `tests/support/combat_death_reward_postgres_cases.rs`. A separate cases file would fail the PG-COVERAGE-1 guard, because `character_authority_postgres.rs`, the CI-run target, is not owned. So `kill_reward_live_postgres_cases.rs` and `tests/combat_death_reward_postgres.rs` are unchanged.
- **`RESOURCE_LIMITS_REGISTRY.json` unchanged:** native section bounds are not registered there.
- `DurableKillSettle::settle` is unchanged (#1907).

## Validation

cargo fmt --all --check: pass
git diff --check: pass
cargo clippy --locked --workspace --all-targets -- -D warnings: pass
cargo test --locked -p oteryn-game-server: pass
python3 -m unittest discover -s tools/content-schema/native-gameplay -p 'test_*.py': OK
python tools/agents/validate_governance.py: pass
python -m unittest discover -s tools/agents/tests: OK

## Closeout

- FREEZE_SHA: reported to the control plane on #1622 at freeze.
- merge commit/result: squash merge of #1923
- review: pending at freeze
