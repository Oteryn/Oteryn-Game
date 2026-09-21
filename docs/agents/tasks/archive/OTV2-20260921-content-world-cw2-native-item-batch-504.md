> Lifecycle closeout: **COMPLETED / ARCHIVED / OWNERSHIP RELEASED**. Delivery PR #719 exact candidate `1d2185dcb00abbbe73e0eb577266510533e79b3a` integrated as `a7e2fd84a21a05bc0e9ea4088031ddafc8e0d20f`; protected current main contains that accepted slice. Any active/checkpoint wording below is historical provenance only.

# OTV2-20260921 — Content World CW2 native Item batch 504

```yaml
task_id: OTV2-20260921-content-world-cw2-native-item-batch-504
title: CW2 native Item binding batch 64
mode: MIGRATE
status: validating
repository: Oteryn/Oteryn-Game
base_branch: main
branch: agent/content-world-cw2-native-item-batch-504
issue: 162
pr: 719
base_sha: 256aa3b152c944cb8451906effe1f0090c5b798d
head_sha: null
owner: Oteryn: content world import
created_at: 2026-09-21T13:00:00Z
updated_at: 2026-09-21T13:35:00Z
owned_paths:
  - apps/game-server/src/content/cw2_b1_import.rs
  - apps/game-server/tests/content_world_cw2_b1_import.rs
  - docs/agents/evidence/OTV2-20260921-content-world-cw2-native-item-batch.json
  - docs/agents/tasks/active/OTV2-20260921-content-world-cw2-native-item-batch-504.md
public_contracts: []
production_authority: NONE
closure: CANDIDATE_ONLY
```

Status: WIP — API-native authoring, pre-freeze hosted validation pending

## Scope and authority

- Task: `Oteryn: content world import`
- Programme issue: #162
- Protected base: `main@256aa3b152c944cb8451906effe1f0090c5b798d`
- Branch: `agent/content-world-cw2-native-item-batch-504`
- META binding: `Oteryn/Oteryn@e102056cc4b9219bc482ceb05afebeb4d62b7bc8`
- Spatial/world placement remains excluded and blocked separately by #483.
- CW3 shared model, CW4 runtime, protocol, persistence, WP3, root Cargo and client paths are excluded.

The owner selected a bounded batch of 64 source items. The batch uses the existing B1 evidence gate, project Item record, typed native binding and deterministic reimport representation. It does not add a parser, catalogue system or shared ItemDefinition surface.

## Pre-mutation custody census

A fresh census of all 29 open pull requests and the target branch was completed after #713 reached protected `main`. No active pull request or live allocation owned or modified any target path. Exact writable paths were allocated as:

1. `apps/game-server/src/content/cw2_b1_import.rs`
2. `apps/game-server/tests/content_world_cw2_b1_import.rs`
3. `docs/agents/evidence/OTV2-20260921-content-world-cw2-native-item-batch.json`
4. `docs/agents/tasks/active/OTV2-20260921-content-world-cw2-native-item-batch-504.md`

## Product

The machine-readable product contains the complete 64-row source-to-native binding map, exact B1 node/profile digests, pinned source provenance, B3 exact-crosswalk occurrence evidence, class counts, unresolved/conflict report, model gaps and CW3 handoff:

- `docs/agents/evidence/OTV2-20260921-content-world-cw2-native-item-batch.json`

Selection:

- 8 currency/stackable/material items;
- 12 weapons;
- 12 armor/equipment items;
- 8 containers;
- 8 consumable/charge-bearing items;
- 16 ordinary physical/decor items.

Every native identity is an explicit Oteryn editorial decision. Numeric IDs, labels, appearance data, source paths and hashes are provenance only and never generate a key. Source values do not become gameplay truth.

## Deterministic closure

- selected source identities: 64
- resolved native binding candidates: 64
- selected unresolved: 0
- selected ambiguous: 0
- selected conflict: 0
- protected B1 overlay projection: 64 resolved / 38,093 unresolved
- selected exact B3 rows ready for a later overlay: 1,930
- B3 rows remaining unresolved: 15,156
- explicit exclusion: `crystal:item:3031` (gold coin), `EXACT_SOURCE_NODE_MISMATCH`

Records sort by native ItemTypeKey, candidates by exact source identity, normalized fields by field path, and reimport states by native identity plus field path. Exact protected B1 byte length and SHA-256 are verified before construction.

## Current ReferenceItemDefinition gaps

The current model cannot carry presentation/appearance, semantic item class, stack maximum, charges, durability, decay/timing, container capacity/policy/nesting, equipment patterns/slot claims/requirements, weapon/use semantics, protection/modifiers/resistances, transfer restrictions, upgrade/imbuement/proficiency, typed weight and unit, interaction families, or independently typed materialization/pickup/destination policies.

Those values remain source observations and explicit losses. CW2 does not extend the shared model.

## CW3 handoff

Minimum proposed semantic deltas:

1. successor bounded multi-item Reference content/artifact profile;
2. typed presentation reference;
3. stack capability with explicit authored maximum;
4. charge capability;
5. container capability;
6. equipment/equip-pattern capability;
7. typed weight plus explicit unit;
8. distinct materialization, pickup and destination policies.

Exact proposed CW3 writable paths:

- `apps/game-server/src/content/reference_playable.rs`
- `apps/game-server/src/content/project.rs`
- `apps/game-server/src/content/reference_artifact.rs`

`apps/game-server/src/content/mod.rs` is conditional only if a new public export is required.

## Validation ledger

Local environment facts:

- deterministic B1/B3 selection checks: PASS (64 unique sources, 64 unique keys, 1,930 exact B3 rows);
- protected input hashes and catalogue counts: PASS;
- branch/path census: PASS;
- `git diff --check`: pending branch readback;
- Rust 1.94 compiler validation: unavailable locally because the runner cannot reach the official toolchain distribution endpoint.

Hosted candidate checks are run on the exact authored branch head before freeze. The final branch head, changed-path readback, check suite and freeze statement are recorded in the pull request.
