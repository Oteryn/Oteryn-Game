# OTV2-20261004-attack-wire-1

```yaml
task_id: OTV2-20261004-attack-wire-1
title: ATTACK-WIRE-1 Capability 17 ATTACK_V1, commands 11 ATTACK_TARGET_INTENT and 12 FIGHT_MODES_INTENT, domain 10 ACTOR_COMBAT_STATE
mode: IMPLEMENT
status: completed
repository: Oteryn/Oteryn-Game
base_branch: main
branch: claude/attack-wire-1-20261004
issue: 1622
pr: 1758
decision: ATTACK-0 §3; ARCH-CORE-LOOP-PACKETS-2 §0.1 and §2.1 (leases); wire lane D486
builds_on: QUEST-LOG-WIRE-1 (#1741) pattern; VIS-2 EntityRefV1 (capability 6)
migration_lease: none
owned_paths:
  - docs/contracts/PROTOCOL_OTERYN_V1_REGISTRY.json        # capability 17, commands 11 and 12, domain 10
  - docs/contracts/RESOURCE_LIMITS_REGISTRY.json           # own ATTACK0-RL-01 and RL-02 rows only
  - docs/contracts/protocol-oteryn/v1/attack_v1.proto      # new
  - crates/protocol-oteryn/src/{lib,attack,attack_tests}.rs
  - apps/game-server/src/gameplay_transport/{capabilities,capabilities_tests}.rs   # GATED row (17, [11,12], [10]) and negative checks only; granted by the #1622 control plane
  - docs/agents/tasks/archive/OTV2-20261004-attack-wire-1.md
```

## Outcome

- Capability 17 `ATTACK_V1`: `requires: [6]`, `offered: false`, offer gate ATTACK-1b. It is
  added to `REGISTERED_CAPABILITY_IDS_V1`, so a client may name it, but the server never selects
  it before ATTACK-1b.
- Command 11 `ATTACK_TARGET_INTENT` (`AttackTargetIntentV1 {target}`, at most 31 bytes): an
  absent target stops attacking; a present one needs a 16-byte identity and a non-zero
  generation, otherwise Malformed.
- Command 12 `FIGHT_MODES_INTENT` (`FightModesIntentV1 {fight_mode, chase, secure}`, at most 6
  bytes): both enums are closed and required, and zero or unknown values are Malformed.
- Both commands answer `AttackIntentResultV1` (at most 2 bytes): OK, TARGET_NOT_VISIBLE,
  TARGET_NOT_A_CREATURE, PROTECTION_ZONE, REENTRY_PROTECTED, REJECTED.
- Domain 10 `ACTOR_COMBAT_STATE`, snapshot type 1 and delta type 1: `ActorCombatStateV1 {target
  or absent, fight_mode, chase, secure, in_fight}`, at most 39 bytes.
- The codec is strict: empty, repeated and unknown fields, a wrong wire type, an explicit false
  bool and an over-bound payload all fail closed, and encoding refuses a zero generation.
  `attack_tests.rs` binds both registries and the proto to the constants.

## Does `chase` carry CHASE now?

Yes. `ChaseMode` has STAND and CHASE, and the server accepts both. Until CHASE-1 the runtime
treats CHASE as STAND (RANGED-0 §8). The client therefore never needs a new wire version for
chase.

## Resource rows (ATTACK0-RL-01, ATTACK0-RL-02)

Both are 25 commands per second per GameSession (range 25..25, CAPACITY_EXCEEDED). The evidence
is Canary: `config.lua.dist` sets `maxPacketsPerSecond = 25` for every client packet of one
connection, and `ProtocolGame::parseAttack` (0xA1) and `ProtocolGame::parseFightModes` (0xA0)
have no tighter per-action limit. So no Canary-faithful client sends more. ATTACK-1b enforces the
window in dispatch and adds the boundary tests.

## Owned-path grant

Registering capability 17 makes the game-server test
`the_gated_commands_and_domains_are_the_registry_ones` require a GATED row. The #1622 control
plane granted `gameplay_transport/{capabilities,capabilities_tests}.rs` for that row and its
negative checks only (BLOCKER option a).

## Excluded

- Dispatch, the runtime auto-attack, domain emission and the client: ATTACK-1b and later.
- `apps/game-server/src/gameplay_transport/mod.rs` (held by DEATH-2b #1747).

## Validation

- `cargo fmt --all -- --check`: pass
- `cargo clippy -p oteryn-protocol-oteryn --all-targets -- -D warnings`: pass
- `cargo clippy -p oteryn-game-server --all-targets -- -D warnings`: pass
- `cargo test --locked -p oteryn-protocol-oteryn`: pass
- `cargo test --locked -p oteryn-session`: pass
- `cargo test --locked -p oteryn-game-server`: pass
- `python tools/agents/validate_governance.py`: pass
- `python -m unittest discover -s tools/agents/tests`: pass
- `git diff --check`: pass
