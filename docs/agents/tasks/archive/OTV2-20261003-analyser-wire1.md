# OTV2-20261003-analyser-wire1

```yaml
task_id: OTV2-20261003-analyser-wire1
title: ANALYSER-WIRE-1 ANALYSER_V1 wire (capability 10, state domain 15), fact codecs, bounds and pending queue
mode: CONTRACT
status: completed
repository: Oteryn/Oteryn-Game
issue: 1622
base_branch: main
branch: claude/analyser-wire1-20261003
pr: "the one named in the #1622 FREEZE_SHA entry"
base_sha: d75ba6d
head_sha: "exact frozen head in the #1622 FREEZE_SHA entry"
final_head_sha: "exact frozen head in the #1622 FREEZE_SHA entry"
final_head_frozen_at: null
owner: work coordinator task worker (hard, protocol review), #1622
created_at: 2026-10-03T00:00:00Z
updated_at: 2026-10-03T00:00:00Z
execution_policy: continuous_progress
owned_paths:
  - docs/contracts/PROTOCOL_OTERYN_V1_REGISTRY.json
  - docs/contracts/RESOURCE_LIMITS_REGISTRY.json
  - docs/contracts/protocol-oteryn/v1/analyser_v1.proto
  - crates/protocol-oteryn/src/{lib,analyser,analyser_tests}.rs
  - docs/agents/tasks/archive/OTV2-20261003-analyser-wire1.md
public_contracts:
  - docs/contracts/PROTOCOL_OTERYN_V1_REGISTRY.json
  - docs/contracts/RESOURCE_LIMITS_REGISTRY.json
  - docs/contracts/protocol-oteryn/v1/analyser_v1.proto
depends_on: ["ANALYSERS-0 D298 (merged, 1e0134d9)"]
blocks: [ANALYSER-EMIT-1, ANALYSER-CLIENT-1]
cross_repository_coordination_id: null
external_repositories: []
```

## Outcome

ANALYSER-WIRE-1 registers the ANALYSERS-0 §4 and §6 wire, using the numbers the control plane leased in STATE (cap 10, domain 15).

- **Capability 10 `ANALYSER_V1`** (`offered: false` until ANALYSER-EMIT-1), with no command type. **State domain 15 `ACTOR_ANALYSER`:** delta type 1 `ANALYSER_FACTS_DELTA_V1` (`AnalyserFactsV1`, 1..=64 facts) and snapshot type 1 `ANALYSER_SNAPSHOT_V1` (0 bytes; the envelope carries the revision). Capability 10 joins `REGISTERED_CAPABILITY_IDS_V1`, so a selected 10 decodes. The server selects none.
- **`AnalyserFactV1`:** a oneof of `experience {raw, gained}`, `kill_loot {race, corpse_items <= 32, gold}`, `supply_used {item_type, count}`, `impact {kind, value, element}`, `damage_input {value, element, source, race}` and `dropped {count}`. An empty or doubled oneof fails closed.
- **Bounds and rows:**
  - `ANALYSERS0-RL-01`: 64 facts.
  - `ANALYSERS0-RL-02`: 32 items.
  - `ANALYSERS0-RL-03`: 4,096 bytes.
  - `ANALYSERS0-RL-04`: 1,024 pending, the §6 row the decision leaves unnumbered.
  - The largest fact is a measured constant, `MAX_ANALYSER_FACT_BYTES = 401` (a 32-item `kill_loot` at its widest).
- **`AnalyserPendingQueue`:** a pure queue of encoded facts.
  - At 1,024 it drops the oldest and counts it.
  - `take_batch` opens with `dropped {count}`, then adds facts while there are fewer than 64 and the batch stays within 4,096 bytes. The rest wait for the next sync unit, never dropped for size.

## Decisions for protocol review (control-plane answers)

1. `race` is the Bestiary race index, `1..=1024`. 0 means a creature with no Bestiary race. It is allowed only for a `CREATURE` source.
2. `AnalyserElement` is a closed enum of the SPELL-PRESENT-0 §2 content damage types, healing excluded. It is required for damage and absent for healing.
3. `item_type` is a non-zero uint32 content item type id. `count` is `1..=65535`.
4. Experience and gold are uint64, and 0 is allowed. `value` and `dropped.count` are non-zero.
5. RL-03 measurement: 64 worst-case `kill_loot` facts are about 25.9 KiB, so the batch is also byte-bounded at 4,096 and the builder splits by bytes.

## Excluded scope

- ANALYSER-EMIT-1 owns:
  - emission at the commit points (§5);
  - offering capability 10;
  - splitting a loot list of more than 32 items into a second `kill_loot`;
  - the per-session revision counter.
- ANALYSER-CLIENT-1 owns the client windows.

## Validation

- `cargo fmt --all -- --check`: PASS.
- `cargo clippy -p oteryn-protocol-oteryn --all-targets -- -D warnings`: PASS.
- `cargo test -p oteryn-protocol-oteryn`: 108 passed, 7 of them analyser tests (round trip, the RL-03 measurement, the bounds, fail-closed, the queue drop and split, and the registry binding).
- `cargo check --workspace --all-targets`: PASS.
- `python3 tools/agents/validate_governance.py`: PASS.
- `git diff --check`: clean.

## Closeout

- PR: the one named in the #1622 FREEZE_SHA entry. Merge commit/result: its squash merge.
- This record was archived in the PR's final authoring commit (`docs/agents/tasks/archive/README.md`).

```yaml
last_progress: final authoring commit; archived before freeze
status: completed
branch: claude/analyser-wire1-20261003
owner_action_required: null
blocker: null
next_action: ANALYSER-EMIT-1 and ANALYSER-CLIENT-1
```
