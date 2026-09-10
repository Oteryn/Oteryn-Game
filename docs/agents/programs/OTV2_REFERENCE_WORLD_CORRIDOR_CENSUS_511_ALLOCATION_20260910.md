# OTV2 Reference World Corridor Census #511 — allocation

Refs #162 #486 #511 #504 #483 #64 #502.

```yaml
classification: COORDINATOR_ALLOCATION
allocation_state: NOT_ACTIVE_CONDITIONAL
repository: Oteryn/Oteryn-Game
programme: 486
control_plane: 162
active_control_plane_profile: OTV2_WORK_DELIVERY_COORDINATOR
admission_main_sha: 1e993fb62cb9912c833f466a021070d9d93d3892
lane_id: R1_WORLD_MAP
worker_task_id: REFERENCE_WORLD_CORRIDOR_CENSUS_511
worker_branch: agent/reference-world-corridor-census-511
runtime_authority: NONE
content_runtime_authority: NONE
registry_mutation_authority: NONE
production_authority: NONE
live_deployment_authority: NONE
```

## Purpose

Allocate exactly one future bounded Phase-A evidence worker to measure the smallest useful migration-source windows around the already-qualified Newhaven and Targuna locator hypotheses. The worker exists only to produce deterministic source-corpus cardinalities and clipping/expansion evidence needed by #504.

This allocation does not select Global target geometry, a permanent world schema, chunk format, `.omap`/`.owb`, production Content maxima, renderer layout or any Evolved behavior.

## Activation gate

This allocation is **NOT ACTIVE** merely because this file or its PR exists.

Material writes by `REFERENCE_WORLD_CORRIDOR_CENSUS_511` are permitted only after all of the following are true:

1. this exact allocation candidate receives producer whole-diff self-review and genuinely independent exact-head control-plane review with `P0=0 / P1=0 / P2=0`;
2. applicable exact-head Agent Governance, Architecture Semantic Audit and Merge Gate are successful;
3. the allocation is integrated through the protected META 3.1 native exact-head Merge Queue path and real `merge_group` aggregate `game-gate` succeeds;
4. protected-main readback proves this exact allocation file is canonical;
5. Work/#162 performs a fresh ownership/overlap/custody readback and explicitly activates this same task/branch.

No direct worker alias, issue comment, branch creation, review result or CI success substitutes for protected allocation plus explicit Work activation.

## Exact future owned paths

After the activation gate, the sole material worker may own only:

```text
tools/reference-world-corridor-census/**
docs/agents/evidence/OTV2-20260910-reference-world-corridor-census.md
docs/agents/tasks/active/OTV2-20260910-reference-world-corridor-census-511.md
```

The new tool directory is optional. If a reproducible one-shot consumer of existing protected APIs is sufficient, omit it rather than creating code for its own sake.

Everything else remains read-only unless a later separately protected allocation amendment says otherwise. In particular, this worker must not mutate:

```text
tools/game-atlas-fullworld-source/**
tools/game-atlas-thais-fixture/**
tools/tibia-worldmap-reconstruction/**
apps/**
docs/contracts/**
docs/architecture/**
docs/agents/programs/**
RESOURCE_LIMITS_REGISTRY.json
.github/workflows/**
Cargo.toml
Cargo.lock
workspace-boundaries.toml
```

Platform, Atlas and every external repository remain read-only. This allocation does not authorize Remote Desktop or any other host access by itself.

## Existing producer boundary — mandatory reuse

Do not create another OTBM parser, legacy intermediate representation, full-world exporter or semantic projection implementation.

Consume the current protected Game-owned producer boundary:

- `tools/game-atlas-fullworld-source/producer.py`
  - `load_runtime(...)` validates the exact accepted map/catalog/assets through the existing qualified producer;
  - `iter_records(...)` streams the pinned source once;
  - tile/town/waypoint discrimination and `native_floor(...)` preserve the accepted spatial transform;
  - `project_tile(...)` / `project_tile_bytes(...)` preserve explicit unresolved appearances fail-closed and reuse qualified deterministic semantic bytes;
- `tools/tibia-worldmap-reconstruction/**` remains the later Phase-B normalized comparison family and is read-only in this Phase-A worker.

Existing workflows do not themselves perform the required bounded Newhaven/Targuna tile/placement census. Do not create a trigger-only/no-op PR or run an unrelated creature census and relabel it as #511 evidence.

## Pinned source and locator inputs

The Phase-A source contract is the existing migration evidence only:

```text
legacy repository: blakinio/Otheryn@e417c5e7c22986bf4acef0495eb47f7b72c97cce
world.otbm SHA-256: 3bd40d14fefec41f24c4b3ae879e420be1a831ef55b95dcbec721e587a09b034
15.32.zip SHA-256: 1a6bad8b7598cd874f534cd4aae2d249fb3d9b4458b3ccfa75754f91bb27870f
```

Catalog/appearance digests must be those accepted by the current protected producer itself. Do not duplicate them into a competing authority table.

Initial source windows:

```text
Newhaven
  semantic record: 3a6d2a5cfd599439efccce35017e12b8
  locator: x=32536, y=32514, floor=-7
  32x32 semantic start shard: f-7-r1016-c1016
  256x256 fullworld source region: fm000007_rxp000127_ryp000127

Targuna
  semantic record: 57d9ef64607b0360d0c01d338f7c70ba
  locator: x=31934, y=31925, floor=-7
  32x32 semantic start shard: f-7-r997-c997
  256x256 fullworld source region: fm000007_rxp000124_ryp000124
```

All coordinates, shards and legacy identities above are `MIGRATION_EVIDENCE / OTS_HYPOTHESIS_ONLY` for Global geometry. They never become `GLOBAL_REFERENCE_TARGET` merely by being measured.

## Required Phase-A method

Treat Newhaven and Targuna as two independent bounded windows.

1. Start from the stated source window for each location.
2. Enumerate source records in deterministic producer order and select only the bounded coordinates/floors needed by that window.
3. Expand by adjacent bounded windows only when an exercised structure/path/service fixture is demonstrably clipped by the current boundary.
4. Record each expansion in deterministic order with the exact clipping/connectivity reason.
5. Never create one giant Newhaven-to-Targuna rectangle. Their later relationship is a typed semantic transition/travel boundary, not a requirement for one contiguous coordinate span.
6. Do not scan or materialize the whole world merely because the producer can stream it if a bounded source/window route is available. If the producer API necessarily performs one source stream internally, keep retained/projected output bounded and record that execution fact honestly.

Ambiguous expansion need must remain explicit rather than silently widening the census.

## Required deterministic outputs

For **each** bounded window report independently:

1. exact Game producer revision and all accepted source identities/digests;
2. exact starting and final source-region/shard/window set plus expansion sequence;
3. exact floors touched;
4. total tile records and non-empty tile records;
5. ordered presentation/placement total and maximum ordered placements per cell;
6. resolved vs unresolved appearance/presentation totals and exact unresolved source identifiers;
7. unique source item/appearance identifiers and unique resolved presentation/sprite identities, migration evidence only;
8. town/waypoint and door/teleport/relocation-like source records encountered, classified only to the strength the existing producer/source supports;
9. candidate composite/multi-cell visual patterns as diagnostics only, with no adjacency-based semantic merge;
10. clipping/connectivity evidence for every added adjacent window;
11. encoded semantic record bytes where the existing producer already defines them;
12. a deterministic machine-readable summary sufficient for #504 to derive lower-bound resource requirements.

Keep these resource dimensions separate: `cells`, `total ordered placements`, `max ordered placements per cell`, `unique identities`, and `encoded bytes`. None may substitute for another.

## Required validation

If a thin consumer is added, use deterministic test-first validation for at least:

- stable results independent of container/hash enumeration;
- exact window inclusion/exclusion at coordinate boundaries;
- floor filtering;
- deterministic expansion ordering;
- explicit unresolved appearance propagation;
- duplicate/counter safety and checked arithmetic;
- source/digest mismatch failure before evidence acceptance;
- no silent dropping of ordered presentations;
- machine-readable summary determinism.

Also run the existing producer self-test/compile checks applicable to the reused API. Do not mutate the producer merely to satisfy the new consumer.

Before worker handoff require full diff review, applicable focused tests and exact evidence/source identity. Before protected integration of any tracked worker result require repository-policy review/CI/Merge Queue appropriate to its final scope.

## Source availability / host boundary

The worker may consume an exact already-authorized source/running environment only through a currently permitted repository/runner mechanism. This allocation does not itself authorize a remote host, credentials, proprietary source redistribution or publication of extracted pixels/assets.

If the exact pinned source/archive/catalog assets needed by `load_runtime(...)` are unavailable on every currently authorized execution surface, return exactly:

```text
SOURCE_CORPUS_REQUIRED
```

with the missing input/execution boundary. Do not guess cardinalities, substitute current Global/Canary/Crystal data, weaken digest checks, or commit proprietary source bytes.

## Phase boundary and downstream use

This allocation is **Phase A migration-corpus census only**.

It does not authorize:

- Phase-B controlled Global target observation/capture;
- target parity classification beyond existing #483 evidence;
- production world/content implementation;
- mutation of `RESOURCE_LIMITS_REGISTRY.json`;
- turning measured lower bounds into hard maxima without a separate evidence-backed resource decision;
- Movement/Combat/runtime activation;
- production deployment;
- Evolved geography/content/balance.

After protected Phase-A evidence, #504 may use the measured cardinalities only as lower-bound/resource-decision inputs. Any production hard maxima remain a separate reviewed registry decision.

## Collision and anti-duplication rule

Immediately before material activation, #162 must re-read open PRs, branches and active tasks. If another canonical #511/corridor census writer exists, this allocation must bind/resume that lineage or remain inactive; it must not create a replacement.

The current pre-allocation readback found no active #511/corridor mutation PR or worker branch.

## Completion handoff

A successful worker return must contain:

```yaml
worker_task_id: REFERENCE_WORLD_CORRIDOR_CENSUS_511
exact_head_sha: <sha>
changed_paths: []
source_identity: <exact pinned identities>
newhaven_window_summary: <machine-readable reference>
targuna_window_summary: <machine-readable reference>
unresolved_source_records: []
focused_validation: []
whole_diff_review: PASS|BLOCKED
phase_a_result: PASS|SOURCE_CORPUS_REQUIRED|BLOCKED_MATERIAL_FINDING
phase_b_target_parity: NOT_PERFORMED
registry_maxima_selected: false
production_authority: NONE
next_action: <one exact coordinator action>
```

`MERGE_AUTHORITY: REPOSITORY_CONTROL_PLANE_ONLY`
