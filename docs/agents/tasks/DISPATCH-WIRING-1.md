# DISPATCH-WIRING-1

```yaml
task_id: DISPATCH-WIRING-1
title: "Isolate gameplay transport state and test construction"
mode: IMPLEMENT
status: in_progress
repository: Oteryn/Oteryn-Game
base_branch: main
branch: w6/wiring
issue: null
pr: null
base_sha: 6ec8f7bdc94064ec4b086792334336a3c15abc04
class: "structural refactor; no behaviour change"
migration: no
registry: no
owner: Codex wiring worker (2026-10-22)
created_at: 2026-10-22
updated_at: 2026-10-22
execution_policy: continuous_progress
merge_authority: CP_ONLY
owned_paths:
  - crates/gameplay/dispatch/src/gameplay_transport/ (mechanical state access updates)
  - crates/gameplay/dispatch/README.md
  - apps/game-server/tests/character_authority_postgres.rs (registry ordering only)
  - apps/game-server/tests/integration_rest/spell_lane_key33_pin.rs (mechanical source-path assertion if required)
  - docs/agents/tasks/DISPATCH-WIRING-1.md
slot_only_paths: []
public_contracts: []
depends_on: []
blocks: []
cross_repository_coordination_id: null
external_repositories: []
```

## Layout planned before implementation

- `gameplay_transport/channel_states.rs`: Default Channel state fields in alphabetical
  order and the existing terminal-retirement cleanup, preserving order and lock comments.
- `gameplay_transport/composed_test_owner.rs`: one cfg(test) constructor borrowing
  existing fixtures; tests retain their explicit overrides.
- Existing owner modules: registration and mechanical state access updates only.
- Existing PostgreSQL test registries: alphabetical module registration, no new mounts.
- Dispatch README: mechanic wiring convention and ADR-0022 reference.

## Authority and scope

Integrator-allocated local work only. META 3.1.0 resolved read-only at the bound
immutable revision. No Git mutation, publication, remote write, dependency download,
migration, wire change, spending or gameplay change is authorized or performed.

The packet names four constructors. Workspace search finds three: production,
monk-save Dependencies and kill-reward E2E. The named item-use PostgreSQL test file
is absent. Both existing test constructors now use the shared builder.

## Implementation and deliberately retained cleanup

All fourteen requested fields, plus the tutorial and Newhaven travel states added to
main since, were Default-constructed in all three existing constructors. Their declarations and lock-order comments moved verbatim into
`ChannelSessionStates`; remaining accesses are mechanical. Terminal retirement
now calls `forget_session` with the original sequence: fence holders, chat, NPC,
quests, Wheel, progression, last position. Old cleanup helpers were removed.

- Premium registration/refresher release stays together under the same lock.
- Lost-session removal keeps its connection-generation condition and resume caller.
- The already-gone logout branch still forgets only fences after Premium release.
- The periodic last-position writer still forgets only its own stale tracks.
- Actor spell/attack state is still governed by its existing detach/prune lifecycle.

`ComposedTestOwner::build` is the only test constructor. Kill-reward overrides its
active generation, rooms and clock origin (60 seconds in the past). Quest/Wheel
catalogues remain None and PremiumRefresher retains the harness root in both tests.
The exact SeamRooms source inventory and key-33 source assertion were updated to
follow the moved constructor/access, preserving their checks and test count.

The registries retain exactly the path mounts they held at the toolchain head (monk-save
14, character authority 68 including the mounts main added since this refactor began),
including attributes and visibility, now sorted. No module was reloaded, added or dropped.
Existing file growth from rustfmt's longer state paths is offset only by removing
blank separators; existing files do not grow. No file grows by more than 200 lines.

## Reuse

Reused the monk-save and kill-reward constructor inputs, existing Harness,
Dependencies, fresh admission fixture, qualified room, capability binding,
PremiumRefresher and SpellLane. The new builder borrows those existing fixtures;
no fixture/harness body was copied and no wrapper/re-export was needed.
Reused all sixteen existing state types and their Default implementations, and
FenceHolders/ChatRuntime/NpcConversations/LastPositions cleanup plus the existing
quest/Wheel/progression map removals, with no parallel cleanup implementation.

## Completed local validation

- Rebase repair (2026-10-22): dispatch fmt and clippy --all-targets -D warnings PASS; non-PG lib 852 PASS (14 existing ignored); filtered item_use/rune_use_bridge/monk_save/kill_reward 78 PASS, including 35 PostgreSQL tests with --nocapture and no missing-admin skip markers.
- Baseline inventories: dispatch lib 916 tests; character_authority_postgres 953.
- Resumed fmt --all --check and two-package clippy --all-targets -D warnings: PASS.
  Vendored Tokio has an existing missing-doc warning; no changed-code warnings.
- Previous dispatch parallel run: 901 passed, 1 map-pickup PG deadline failure,
  14 pre-existing ignored. No assertion/deadline changed. Its serial retry's final
  result was not retained across the expired login; do not claim it passed.
- PostgreSQL execution confirmed: monk-save shutdown, cast/Wheel, party and map
  pickup cases created real harness databases and completed. Those harnesses assert
  server_version_num 170006 before applying the real migrator.
- Previous character authority full serial retry: 953 PASS, 0 failed, 0 ignored;
  all 953 names identical to baseline. All nine earlier deadline/unknown-commit
  failures pass. Harness threading alone changed; assertions, production deadlines
  and each case's internal concurrency scenarios remain intact.
- Resumed validation obeys the newer owner PG scope rule: only filtered runs,
  never either full PG suite. The monk-save owner filter covers both builder
  callers, logout/retirement, shutdown, death, map pickup and the existing fixture.
- Resumed focused tests: PG owner 29 PASS; character authority reward 1 PASS;
  session units 32 PASS; bundle inventory 5 PASS; key-33 pins 8 PASS.
- Governance: PASS (22 required documents, 9 lanes); unit tests: 124 PASS.
- Repository policy: PASS (23 files, 68 workflows).
- Workspace searches cover both new structs, their fields and all constructors;
  no enum variant or trait method was added. All moved field docs/types match the
  originals. Independent normalized comparison proves the 34 access-only files
  differ solely in state paths, whitespace/rustfmt call commas and cleanup comments.
- Whole-diff review against /home/mole/oteryn-cache/CODEX-CHECKLIST.md and the
  repository checklist completed. Expanded cleanup matches the old sequence exactly;
  all 916 dispatch test names match the baseline inventory.

No publication, freeze SHA, PR, review trigger or archive transition belongs to this
local worker. The integrator owns those lifecycle steps.

## Exact resumed validation commands and filters

All Rust commands use `/home/mole/oteryn-cache/codex-bin/cargo +1.95.0`.

```text
fmt --all --check
clippy --locked -p oteryn-gameplay-dispatch -p oteryn-game-server --all-targets -- -D warnings
test --locked -p oteryn-gameplay-dispatch --lib -- --list
test --locked -p oteryn-game-server --test character_authority_postgres -- --list
test --locked -p oteryn-gameplay-dispatch --lib gameplay_transport::monk_save::postgres_tests:: -- --test-threads=1 --nocapture
test --locked -p oteryn-gameplay-dispatch --lib gameplay_transport::bundle_gates_tests:: -- --test-threads=1
test --locked -p oteryn-gameplay-dispatch --lib gameplay_transport::tests:: -- --test-threads=1
test --locked -p oteryn-game-server --test character_authority_postgres attack_kill_reward_postgres_cases:: -- --test-threads=1 --nocapture
test --locked -p oteryn-game-server --test integration_rest spell_lane_key33_pin -- --test-threads=1
```

The two `--list` commands enumerate tests without executing PG. No test was added
or removed. Unfinished: full-suite qualification remains CI-only under
the owner PG scope rule; publication/integration remain with the integrator.

## Changed-file line counts (HEAD baseline to local result)

| Path | Before | After |
| --- | ---: | ---: |
| `apps/game-server/tests/character_authority_postgres.rs` | 3451 | 3409 |
| `apps/game-server/tests/integration_rest/spell_lane_key33_pin.rs` | 595 | 595 |
| `crates/gameplay/dispatch/README.md` | 0 | 20 |
| `crates/gameplay/dispatch/src/gameplay_transport/attack.rs` | 2013 | 2013 |
| `crates/gameplay/dispatch/src/gameplay_transport/bundle_gates_tests.rs` | 666 | 666 |
| `crates/gameplay/dispatch/src/gameplay_transport/channel_states.rs` | 0 | 80 |
| `crates/gameplay/dispatch/src/gameplay_transport/chat_intent.rs` | 1635 | 1635 |
| `crates/gameplay/dispatch/src/gameplay_transport/combat_lock_fact.rs` | 145 | 145 |
| `crates/gameplay/dispatch/src/gameplay_transport/composed_test_owner.rs` | 0 | 66 |
| `crates/gameplay/dispatch/src/gameplay_transport/equipment_move.rs` | 344 | 344 |
| `crates/gameplay/dispatch/src/gameplay_transport/familiar_cast.rs` | 2230 | 2230 |
| `crates/gameplay/dispatch/src/gameplay_transport/familiar_cast_dispatch.rs` | 1035 | 1035 |
| `crates/gameplay/dispatch/src/gameplay_transport/familiar_defense.rs` | 310 | 310 |
| `crates/gameplay/dispatch/src/gameplay_transport/item_look.rs` | 661 | 661 |
| `crates/gameplay/dispatch/src/gameplay_transport/item_ref_admission.rs` | 346 | 346 |
| `crates/gameplay/dispatch/src/gameplay_transport/item_use.rs` | 619 | 619 |
| `crates/gameplay/dispatch/src/gameplay_transport/kill_reward.rs` | 1213 | 1213 |
| `crates/gameplay/dispatch/src/gameplay_transport/kill_reward_postgres_e2e_tests.rs` | 400 | 386 |
| `crates/gameplay/dispatch/src/gameplay_transport/last_position.rs` | 1344 | 1344 |
| `crates/gameplay/dispatch/src/gameplay_transport/logout.rs` | 193 | 193 |
| `crates/gameplay/dispatch/src/gameplay_transport/logout_postgres_tests.rs` | 427 | 427 |
| `crates/gameplay/dispatch/src/gameplay_transport/loop_map_pickup.rs` | 339 | 339 |
| `crates/gameplay/dispatch/src/gameplay_transport/loop_map_pickup_tests.rs` | 546 | 546 |
| `crates/gameplay/dispatch/src/gameplay_transport/loop_move.rs` | 518 | 518 |
| `crates/gameplay/dispatch/src/gameplay_transport/mod.rs` | 8167 | 8118 |
| `crates/gameplay/dispatch/src/gameplay_transport/monk_save.rs` | 209 | 209 |
| `crates/gameplay/dispatch/src/gameplay_transport/monk_save_postgres_tests.rs` | 1363 | 1324 |
| `crates/gameplay/dispatch/src/gameplay_transport/monster_ai_cycle.rs` | 1297 | 1297 |
| `crates/gameplay/dispatch/src/gameplay_transport/movement_equipment.rs` | 162 | 162 |
| `crates/gameplay/dispatch/src/gameplay_transport/native_combat_cast.rs` | 3584 | 3584 |
| `crates/gameplay/dispatch/src/gameplay_transport/npc_command.rs` | 1287 | 1287 |
| `crates/gameplay/dispatch/src/gameplay_transport/npc_trade_command.rs` | 253 | 253 |
| `crates/gameplay/dispatch/src/gameplay_transport/parameter_cast.rs` | 1303 | 1303 |
| `crates/gameplay/dispatch/src/gameplay_transport/party_spell_owner.rs` | 484 | 484 |
| `crates/gameplay/dispatch/src/gameplay_transport/resume.rs` | 704 | 704 |
| `crates/gameplay/dispatch/src/gameplay_transport/source_item_cycle.rs` | 233 | 233 |
| `crates/gameplay/dispatch/src/gameplay_transport/spell_periodic.rs` | 458 | 458 |
| `crates/gameplay/dispatch/src/gameplay_transport/spell_premium_coordinator.rs` | 1123 | 1123 |
| `crates/gameplay/dispatch/src/gameplay_transport/spell_timer_callbacks.rs` | 815 | 815 |
| `crates/gameplay/dispatch/src/gameplay_transport/stance_cast.rs` | 343 | 343 |
| `crates/gameplay/dispatch/src/gameplay_transport/training_save.rs` | 303 | 303 |
| `crates/gameplay/dispatch/src/gameplay_transport/vocation_choice.rs` | 838 | 838 |
| `crates/gameplay/dispatch/src/gameplay_transport/world_item_cast.rs` | 1101 | 1101 |
| `docs/agents/tasks/DISPATCH-WIRING-1.md` | 0 | 191 |

## Port to the Rust 1.99.1 / PostgreSQL 18.6 head

The refactor was cherry-picked onto the toolchain head and re-applied to the mechanics
main gained since: `tutorial_sessions` and `newhaven_travel` joined `ChannelSessionStates`
(sorted, forgotten after the last-position track as before), the control-loss
reconciliation, paced movement, tutorial and Newhaven owners follow the moved state paths,
and the two registries were re-sorted with the mounts main added. Validation ran on
`cargo +1.99.1`; PostgreSQL-backed tests were not run (the shared server is 17.6).
