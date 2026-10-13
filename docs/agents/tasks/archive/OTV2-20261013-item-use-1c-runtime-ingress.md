# OTV2-20261013-item-use-1c-runtime-ingress

```yaml
task_id: OTV2-20261013-item-use-1c-runtime-ingress
title: "ITEM-USE-1c: Channel-owner item use (cooldowns, requirements, effects) and capability 15 ingress"
mode: IMPLEMENT
status: completed
repository: Oteryn/Oteryn-Game
base_branch: main
branch: claude/use-item-1-20261013
issue: 1622
pr: null
base_sha: ab7d51ae
head_sha: "exact frozen head in the control plane FREEZE_SHA entry"
final_head_sha: null
final_head_frozen_at: null
owner: ITEM-USE-1c worker under the ITEM-USE-1 lead
created_at: 2026-10-13T00:00:00+00:00
updated_at: 2026-10-13T00:00:00+00:00
execution_policy: continuous_progress
owned_paths:
  - apps/game-server/src/gameplay_transport/item_use.rs
  - apps/game-server/src/gameplay_transport/mod.rs
  - apps/game-server/src/gameplay_transport/actor_spell.rs
  - apps/game-server/src/gameplay_transport/connection.rs
  - apps/game-server/src/gameplay_transport/capabilities_tests.rs
  - apps/game-server/src/spell/native_actor_states.rs
  - apps/game-server/tests/support/item_use_postgres_cases.rs
  - docs/contracts/RESOURCE_LIMITS_REGISTRY.json
  - docs/agents/tasks/archive/OTV2-20261013-item-use-1c-runtime-ingress.md
public_contracts:
  - docs/architecture/reviews/OTERYN_GAME_ITEM_USE0_USING_ITEMS_DECISION_2026-09-30.md
depends_on:
  - OTV2-20261013-item-use-1ab-contract-store
blocks: []
cross_repository_coordination_id: null
external_repositories: []
```

## Outcome

Successor of #2039 (1a+1b). Decision ITEM-USE-0 §2, §4-§6. No wire, persistence, authority or
capability-offering change: capability 15 stays `offered: false`, `use_with` stays rejected and a
use by definition still finds nothing to use.

- **Runtime (`gameplay_transport/item_use.rs`).** Per-actor gate in Channel runtime state
  (`ChannelSpellStates::item_uses`): one use in flight, `ITEMUSE0-RL-01` (5 per second), cooldowns
  food 200 ms and potions 1,000 ms shared (both `PARITY_PENDING`) started at PREPARE and kept after
  a failed commit; a refusal is `EXHAUSTED`. Requirements (level, magic level, vocation) from
  content before any reservation (`REQUIREMENT_NOT_MET`); a value content does not know is not
  enforced, a conflicting one fails closed. Food is dry-run through the FoodRegeneration cap
  (`FULL`, nothing burned). The flask output is a compatible stack, a fresh entry or `Missing`.
- **Commit and effect.** `DurabilityRoot::commit_item_use` under the session's item fence, awaited
  for at most `ITEMUSE0-RL-04` (2,000 ms). Only a `Written` outcome applies the effect: food through
  `eat_food`, a potion's Health/Mana restore as a deterministic draw keyed by the CommandRef through
  the owner credit path (`HealingOrigin::ItemUse`, outside Sustain). An ambiguous commit answers
  `REJECTED`, holds the in-flight slot until RL-04 and leaves the reserved items unspendable for
  the GameSession.
- **Ingress.** `connection.rs` routes a `USE` of a main backpack direct entry to `use_item` only
  under capability 15; every other handle keeps the corpse path.
- **Rows.** `ITEMUSE0-RL-01`, `-RL-02`, `-RL-02-DB-P99` and `-RL-04` registered; RL-02 and its p99
  measured by `item_use_commit_throughput_is_measured` (local PostgreSQL 17.6, 99 sequential
  commits, three runs: 120-133 commits/s, p99 18.7-55.3 ms; worst values registered).

## High-risk authority/recovery qualification

`NOT_APPLICABLE` to new authority: the slice adds a caller of the accepted fenced writer
`commit_item_use` (#2039) under the session's existing item fence and changes no fence, lease,
recovery evidence or controller. Replay, stale plan and fence refusal stay the store's (covered by
the item-use Postgres cases).

## Acceptance criteria

- [x] Cooldowns, in-flight slot, RL-01 and RL-04 boundaries: unit tests in `item_use.rs`.
- [x] Requirements and vocation mapping: unit test in `item_use.rs`.
- [x] Sustain never scales `HealingOrigin::ItemUse`: `native_actor_states` unit test.
- [x] Ingress only under capability 15: `a_backpack_entry_is_used_only_under_capability_15`.
- [x] RL-02 measured: `item_use_commit_throughput_is_measured`.

## Excluded scope

Capability 15 offering, `use_with`, use by definition, flask overflow to the Ground (ITEM-MOVE-2b),
official cooldown values and food requirements (`PARITY_PENDING`).

## Validation

### Focused

- `cargo fmt --all --check`: pass
- `cargo clippy --locked -p oteryn-game-server --all-targets --quiet -- -D warnings`: pass
- `cargo test --locked -p oteryn-game-server --lib`: pass (2943 passed, 29 ignored)
- `python tools/agents/validate_governance.py`: pass
- `python -m unittest discover -s tools/agents/tests`: OK
- `git diff --check`: pass

### Component/integration

- `cargo test --locked -p oteryn-game-server --test item_use_postgres`: pass on PostgreSQL 17.6
- `cargo test --locked -p oteryn-game-server --test character_authority_postgres -- item_use`: pass on PostgreSQL 17.6

### E2E

- `NOT_APPLICABLE`: capability 15 is not offered, so no client reaches the path until its offering task.

### Exact-head CI

- final head: pending (lead)

## Self-review

- exact head: the single authoring commit of this record
- method/reviewer: implementing worker
- material findings: none open; deviations reported to the lead
- verdict: ready for independent review

## Independent review

- required: YES (runtime gameplay mutation through a fenced durable writer)
- exact head: pending (lead)

## PR and closeout

- merge commit/result: pending (lead)
- ownership release: on merge
