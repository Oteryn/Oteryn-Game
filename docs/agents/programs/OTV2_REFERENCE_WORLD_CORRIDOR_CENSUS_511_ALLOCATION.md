# OTV2 Reference World Corridor Census #511 Allocation

```yaml
allocation_id: REFERENCE_WORLD_CORRIDOR_CENSUS_511
repository: Oteryn/Oteryn-Game
coordinator_issue: 162
governing_issue: 511
lane_id: R1_WORLD_MAP
active_control_plane_profile: OTV2_WORK_DELIVERY_COORDINATOR
allocation_state: PROSPECTIVE_NOT_ACTIVE
admission_main_sha: 1e993fb62cb9912c833f466a021070d9d93d3892
worker_branch: agent/reference-world-corridor-census-511
worker_task: docs/agents/tasks/active/OTV2-20260910-reference-world-corridor-census-511.md
implementation_authority: PHASE_A_MIGRATION_CORPUS_CENSUS_ONLY
runtime_authority: NONE
registry_mutation_authority: NONE
production_authority: NONE
external_repository_write_authority: NONE
```

## Allocation purpose

This is the bounded #162 allocation package for the first #511 Phase-A migration-corpus census. It activates no worker merely by existing on an unmerged branch. The worker becomes writable only after this allocation is integrated through protected `main`, protected-main readback confirms the exact allocation, and the active Work Delivery Coordinator performs a fresh live admission check.

This allocation consumes the prepared #511 packet in Issue comment `5609420714` and the later execution-surface confirmation in comment `5613807995`. GitHub LIVE state remains lifecycle authority.

The allocation is deliberately evidence-only. It measures two bounded source windows using existing Game tooling. It does not establish Global Reference geometry, select a permanent world format, choose production resource maxima, or activate runtime/content/client behavior.

## Proven admission facts

- **PROVEN:** protected Game admission main is `1e993fb62cb9912c833f466a021070d9d93d3892`, the protected Merge Queue result of #515.
- **PROVEN:** #509/#515 is terminally integrated; its former graphics branch is no longer an active writer.
- **PROVEN:** Issue #511 is open and has no active `reference-world-corridor` branch or implementation PR at allocation preflight.
- **PROVEN:** `OTV2_WORK_DELIVERY_COORDINATOR` is the unique mutating control-plane profile for #162 under the current protected coordinator task.
- **PROVEN:** the existing protected `tools/game-atlas-fullworld-source/producer.py` API is sufficient for Phase A; no OTBM parser/importer clone is required.
- **PROVEN:** no existing GitHub workflow directly executes the required bounded Newhaven/Targuna tile/placement census.
- **PROVEN:** #502 remains an evidence/owner-decision gate and does not grant renderer/registry mutation. #515 measurements are workload observations, not production `RENDER-RL-*` maxima.

## Source boundary

Use only the existing pinned migration/import source contract already enforced by Game tooling:

- legacy migration source: `blakinio/Otheryn@e417c5e7c22986bf4acef0495eb47f7b72c97cce` — read-only migration evidence / `OTS_HYPOTHESIS_ONLY`;
- `world.otbm` SHA-256: `3bd40d14fefec41f24c4b3ae879e420be1a831ef55b95dcbec721e587a09b034`;
- `15.32.zip` SHA-256: `1a6bad8b7598cd874f534cd4aae2d249fb3d9b4458b3ccfa75754f91bb27870f`;
- catalog/appearance identities: exactly those accepted by the protected `game-atlas-fullworld-source` producer at execution time; do not duplicate a second authority table.

No proprietary pixel archive, decoded sprite sheet, source map payload or screenshot may be committed.

## Exact starting windows

### Newhaven

```text
semantic record: 3a6d2a5cfd599439efccce35017e12b8
migration locator: (32536,32514), native floor -7
32x32 semantic start shard: f-7-r1016-c1016
256x256 fullworld source region: fm000007_rxp000127_ryp000127
```

### Targuna

```text
semantic record: 57d9ef64607b0360d0c01d338f7c70ba
migration locator: (31934,31925), native floor -7
32x32 semantic start shard: f-7-r997-c997
256x256 fullworld source region: fm000007_rxp000124_ryp000124
```

These locators are migration-corpus evidence only. They are not promoted to `GLOBAL_REFERENCE_TARGET` coordinates by this allocation.

## Worker owned paths

The worker receives exclusive write authority only for:

```text
tools/reference-world-corridor-census/**
docs/agents/evidence/OTV2-20260910-reference-world-corridor-census.md
docs/agents/tasks/active/OTV2-20260910-reference-world-corridor-census-511.md
```

The `tools/reference-world-corridor-census/**` path is optional. Prefer a reproducible one-shot consumer of existing producer APIs when that is sufficient. Create tracked helper code only when deterministic reproduction/tests materially require it.

No other path is implicitly writable.

## Read-only dependencies

The worker may read but must not mutate without a separately protected #162 amendment:

```text
tools/game-atlas-fullworld-source/**
tools/game-atlas-thais-fixture/**
tools/tibia-worldmap-reconstruction/**
apps/**
crates/**
docs/contracts/**
docs/architecture/**
docs/agents/programs/**
RESOURCE_LIMITS_REGISTRY.json
Cargo.toml
Cargo.lock
.github/workflows/**
```

Atlas, Platform, META and legacy repositories remain read-only unless separately and exactly owner-authorized. This allocation grants no cross-repository write.

## Required Phase-A outputs

Produce an independent machine-readable and human-readable result for **each** Newhaven and Targuna bounded window. Keep the two regions separate.

1. Exact source/digest/producer revision used.
2. Exact selected source regions/shards and deterministic expansion order.
3. Floors touched.
4. Total tile records and non-empty tile records.
5. Ordered presentation/placement count total and maximum placements per cell.
6. Resolved versus unresolved appearance/presentation counts plus exact unresolved identifiers.
7. Unique source item/appearance identifiers and unique resolved presentation/sprite identities, labelled migration evidence only.
8. Door-, teleport-, relocation-, waypoint- and town-like records encountered, classified structurally without inventing gameplay semantics.
9. Candidate composite/multi-cell visual patterns as diagnostics only.
10. Exact clipping/connectivity reason for every adjacent shard/window expansion.
11. Encoded semantic record bytes where already defined by the existing producer.
12. A deterministic machine-readable summary sufficient for #504 to derive lower-bound resource requirements.

Always report cells, total placements and maximum placements per cell as distinct dimensions.

## Expansion policy

Start from the two exact known windows. Expand only when a semantically required structure/path/service fixture is clipped by the current boundary. Record every expansion decision and its reason.

Never scan or encode one giant Newhaven-to-Targuna rectangle. Their geographic gap is not a resource requirement. A later accepted semantic transition/relocation joins the two bounded regions.

If the evidence does not justify an expansion, do not widen the window for aesthetic completeness.

## Failure behavior

- Pinned source unavailable: `SOURCE_CORPUS_REQUIRED`; do not guess counts.
- Digest/source identity mismatch: fail closed before accepting measurements.
- Critical unresolved mapping/appearance: record it explicitly; do not substitute Crystal/Canary/current-Global data.
- Count/byte arithmetic overflow: fail closed using checked arithmetic.
- Ambiguous clipping/expansion necessity: report the ambiguity; do not silently widen toward whole-world scanning.
- Proprietary source/pixel bytes: never commit them.
- Missing Phase-B Global target observation: keep Phase A classified as migration evidence; do not infer parity.

## Acceptance

Before the worker may return `READY_FOR_INTEGRATION` for its Phase-A evidence slice:

- [ ] exact pinned source identities are verified before census acceptance;
- [ ] Newhaven and Targuna are measured independently from the exact starting windows;
- [ ] expansions, if any, are deterministic and justified by clipping/connectivity evidence;
- [ ] no silent tile/item/stack/presentation loss is observed in the selected source records;
- [ ] unresolved identifiers are explicit and machine-readable;
- [ ] cells, placements, max-per-cell, floors and transition-like records are reported independently;
- [ ] output is compatible with downstream #504 lower-bound resource derivation;
- [ ] no OTS/migration fact is promoted to Global target truth;
- [ ] no permanent `.omap/.owb`, chunk, resource-page or GPU layout is selected;
- [ ] no production/resource-registry/runtime/client/server path changes;
- [ ] focused deterministic tests exist if tracked helper code is added;
- [ ] complete changed-file/diff self-review reports zero open material findings;
- [ ] exact-head repository checks required by changed paths are successful;
- [ ] unresolved review threads/requested changes are zero before any integration request.

Runtime E2E for this Phase-A census is `NOT_APPLICABLE`: the allocation produces migration-corpus measurements and evidence only; it activates no runtime/user journey.

## Relationship to #502 and #504

#511 supplies measured world-corpus lower bounds to #504. It may also provide additional Reference-shaped resource observations useful to #502, but it cannot solve #502's unresolved first-supported hardware/graphics product floor and cannot serialize production `RENDER-RL-*` maxima.

Do not copy prototype counts into production limits by convenience.

## Activation and integration discipline

```text
allocation branch/PR
  -> exact-head docs/governance validation
  -> mandatory self-review
  -> protected META 3.1 Merge Queue integration after separate exact-candidate owner authorization
  -> protected-main readback
  -> fresh #162 admission / overlap check
  -> create or reuse exactly worker branch agent/reference-world-corridor-census-511
  -> execute Phase A only
```

An unmerged allocation PR is not write authority. `kontynuuj`, an alias invocation, task existence or branch existence does not grant merge authority or worker activation by itself.

No direct merge, generic auto-merge, bypass, no-op/retrigger commit, workflow widening, registry mutation, production activation or external repository write is authorized.
