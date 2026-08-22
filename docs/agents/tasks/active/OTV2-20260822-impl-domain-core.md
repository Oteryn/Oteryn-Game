# OTV2-20260822-impl-domain-core

```yaml
task_id: OTV2-20260822-impl-domain-core
title: Implement Character and Item semantic domain core
mode: IMPLEMENT
status: implementing
repository: Oteryn/Oteryn-Game
base_branch: main
branch: agent/otv2-impl-domain-core-01
pr: null
base_sha: fd39c6aa026e82062a8b29af24811d467c115f19
allocation_merge_sha: 33cec30b8075c73290d7d76e9f59df4701771650
owner: chat-github-20260822-domain-core
created_at: 2026-08-22T18:11:00+02:00
updated_at: 2026-08-22T18:11:00+02:00
execution_budget_minutes: 60
owned_paths:
  - apps/game-server/src/domain/**
  - docs/agents/tasks/active/OTV2-20260822-impl-domain-core.md
public_contracts:
  - docs/architecture/GAME-CHAR-01_STAGE_A_OWNER_BASELINE.md
  - docs/architecture/GAME-CHAR-01_STAGE_B_OWNER_BASELINE.md
  - docs/architecture/GAME-ITEM-01_ITEM_MODEL_AND_EQUIPMENT_CONTRACT.md
  - docs/architecture/SIM-DETERMINISM-01_AUTHORITATIVE_SIMULATION_CONTRACT.md
  - docs/architecture/DUR-03_ITEM_TRANSACTION_AND_ANTI_DUPLICATION_CONTRACT.md
```depends_on:
  - Oteryn-Game#45
  - Oteryn-Game#46
  - OTV2-20260818-impl-simulation
blocks:
  - OTV2-IMPL-DURABILITY
  - OTV2-IMPL-ABILITY
  - OTV2-IMPL-INTERACTION
  - OTV2-IMPL-AI
cross_repository_coordination_id: OTV2-NATIVE-FOUNDATION
external_repositories: []
```

## Outcome

Deliver protocol- and persistence-neutral Character/Item semantic primitives needed by the first VSL, with deterministic legality/transitions and explicit revision/fixture context.

## Source facts

- `PROVEN`: this lane owns only `apps/game-server/src/domain/**` plus its task record until the coordinator advances the shared lease.
- `PROVEN`: DUR owns durable conservation/transaction mechanics; SIM owns deterministic arithmetic/RNG; protocol/client own adapters.
- `UNKNOWN` Reference formulas remain fixture-only or absent and cannot become product values.

## Acceptance criteria

- [ ] TDD-first Character lifecycle/build/progression primitives with invalid transition rejection.
- [ ] TDD-first ItemDefinition/ItemInstance identities plus typed location/custody/equipment/container legality.
- [ ] Exact revision/profile/content context required for interpretation; incompatible context fails closed.
- [ ] Stable typed errors with no wire IDs, DB schema, UI or generic untyped misc state.
- [ ] Structural fixture profile is explicitly versioned/non-Reference and cannot activate as ordinary product policy.
- [ ] Focused tests pass without requiring unmerged sibling output.
## Implementation plan

1. RED: add unit tests inside the domain module for Character lifecycle/revision and Item location/equipment/container legality APIs.
2. GREEN: implement only the semantic types/state transitions required by those tests using existing workspace primitives.
3. Add negative fixture-profile activation tests proving Reference-unknown values cannot masquerade as product policy.
4. Keep shared composition files untouched until coordinator lease; validate the module independently with `rustc`/temporary test inclusion only through allocated paths if needed.
5. After FOUNDATION shared composition merges or lease advances, integrate minimally, run workspace validation, self-review and PR closeout.

## Excluded scope

No PostgreSQL, transactions, wire protocol, UI, entitlement authority, final formulas/rates, channel switching or broad Character/Item product breadth.

## Validation

### Focused
- command/run: `rustc --edition 2024 --test apps/game-server/src/domain/mod.rs -o target/domain-tests.exe && target/domain-tests.exe`
- result: PASS — 5 tests; 0 failed

### Component/integration
- command/run: `cargo test --workspace` after lawful composition integration
- result: module production compile PASS with `rustc --edition 2024 --crate-type lib -D warnings`; workspace integration remains pending shared lease

### E2E
- scenario: `NOT_EVALUATED` until consuming VSL seams are merged
- result: pending

## Context checkpoint

```yaml
last_progress: TDD domain kernel is GREEN standalone: UUIDv7 identities, revision-fenced Character lifecycle, explicit item revision context/injected limits, atomic equip claims and read-only container legality; 5/5 tests and production `-D warnings` compile pass.
status: implementing
branch: agent/otv2-impl-domain-core-01
head_sha: fd39c6aa026e82062a8b29af24811d467c115f19
pr: null
blocker: shared composition lease is intentionally held by FOUNDATION
owner_action_required: null
next_action: commit/push the independent domain kernel; defer game-server composition integration until the coordinator advances the shared lease after FOUNDATION.
```
