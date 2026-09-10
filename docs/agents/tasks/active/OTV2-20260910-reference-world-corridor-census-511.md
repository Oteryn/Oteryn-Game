# OTV2-20260910-reference-world-corridor-census-511

```yaml
task_id: OTV2-20260910-reference-world-corridor-census-511
title: Measure bounded Newhaven and Targuna migration-corpus windows
mode: MIGRATE
status: waiting
repository: Oteryn/Oteryn-Game
issue: 511
base_branch: main
branch: agent/reference-world-corridor-census-511
pr: null
base_sha: 1e993fb62cb9912c833f466a021070d9d93d3892
head_sha: null
final_head_sha: null
final_head_frozen_at: null
owner: unassigned_until_protected_162_admission
created_at: 2026-09-10T08:06:18+02:00
updated_at: 2026-09-10T08:09:57+02:00
execution_policy: continuous_progress
owned_paths:
  - tools/reference-world-corridor-census/**
  - docs/agents/evidence/OTV2-20260910-reference-world-corridor-census.md
  - docs/agents/tasks/active/OTV2-20260910-reference-world-corridor-census-511.md
public_contracts: []
depends_on:
  - Oteryn/Oteryn-Game#162
  - Oteryn/Oteryn-Game#511
blocks:
  - Oteryn/Oteryn-Game#504
cross_repository_coordination_id: null
external_repositories:
  - blakinio/Otheryn@e417c5e7c22986bf4acef0495eb47f7b72c97cce (READ_ONLY_MIGRATION_EVIDENCE)
```

## Activation gate

`WAITING_PROTECTED_ALLOCATION`.

This task is prospective. It receives no write authority until `docs/agents/programs/OTV2_REFERENCE_WORLD_CORRIDOR_CENSUS_511_ALLOCATION.md` is integrated through protected `main`, the resulting protected-main SHA is read back, and the active #162 Work Delivery Coordinator performs a fresh duplicate/ownership/dependency check and explicitly admits this task.

Do not create the worker branch merely because this task file exists on an allocation PR.

## Outcome

Produce the smallest deterministic Phase-A migration-corpus census needed for the first Reference Newhaven/Targuna corridor evidence. Measure Newhaven and Targuna as two separate bounded source windows through the existing protected Game fullworld producer APIs, preserve exact source provenance, and emit machine-readable lower-bound measurements suitable for downstream #504.

The observable result is a reproducible census with explicit unresolved records and expansion reasons, not a playable world import and not a claim of Global Reference geometry parity.

## Architecture and source of truth

- **PROVEN:** active allocation/control-plane authority remains #162 / `OTV2_WORK_DELIVERY_COORDINATOR`.
- **PROVEN:** Issue #511 defines this as `WORLD_REFERENCE_EVIDENCE_GATE`, Phase A first, with no runtime/registry/production authority.
- **PROVEN:** existing `tools/game-atlas-fullworld-source/producer.py` supplies the source-validation and ordered semantic projection boundary required for the census.
- **PROVEN:** `tools/tibia-worldmap-reconstruction/**` remains the later Phase-B target-comparison family and is read-only for this task.
- **PROVEN:** no second OTBM parser/importer is necessary or authorized.
- **PROVEN:** protected #515/#509 is terminal and the former real-content graphics worker is released.
- **UNKNOWN:** exact Global target geometry for the migration-source Newhaven/Targuna coordinates; keep all such source observations `OTS_HYPOTHESIS_ONLY` until separately qualified under #483.
- **UNKNOWN:** any production hard resource maxima inferred from the resulting counts; #504/#502 own later resource disposition.

### Pinned migration source

```text
legacy source revision:
  blakinio/Otheryn@e417c5e7c22986bf4acef0495eb47f7b72c97cce
world.otbm sha256:
  3bd40d14fefec41f24c4b3ae879e420be1a831ef55b95dcbec721e587a09b034
15.32.zip sha256:
  1a6bad8b7598cd874f534cd4aae2d249fb3d9b4458b3ccfa75754f91bb27870f
catalog/appearance identities:
  consume the exact identities enforced by protected game-atlas-fullworld-source;
  do not duplicate them as a new authority table
```

### Exact Phase-A starts

Newhaven:

```text
semantic_record=3a6d2a5cfd599439efccce35017e12b8
locator=(32536,32514)
native_floor=-7
semantic_shard=f-7-r1016-c1016
fullworld_region=fm000007_rxp000127_ryp000127
```

Targuna:

```text
semantic_record=57d9ef64607b0360d0c01d338f7c70ba
locator=(31934,31925)
native_floor=-7
semantic_shard=f-7-r997-c997
fullworld_region=fm000007_rxp000124_ryp000124
```

These are migration-source locators, not Global target coordinates.

## High-risk authority/recovery qualification

`NOT_APPLICABLE`: this task does not authorize a production mutation, durable authority transition, session/lease replacement, PREPARE/COMMIT, persisted recovery interpretation, runtime activation or live data write. Source validation and evidence classification remain fail-closed but are not authority-bearing production recovery semantics.

## Required implementation shape

Prefer the smallest reproducible execution:

```text
protected game-atlas-fullworld-source producer APIs (read-only)
+ two exact bounded start windows
-> deterministic census consumer
-> machine-readable summary
-> public-safe evidence document
```

If a one-shot invocation of existing APIs is sufficient and reproducible, do not create `tools/reference-world-corridor-census/**`. If tracked orchestration is materially required, keep it a thin consumer with focused deterministic tests; it must not parse OTBM, reconstruct appearance semantics, or copy the producer implementation.

## Measurements

For each region independently report:

1. exact source/digest/producer revision;
2. selected source-region/shard set and deterministic expansion order;
3. floors touched;
4. total tile records and non-empty tile records;
5. total ordered presentation/placement records and maximum placements per cell;
6. resolved/unresolved appearance/presentation counts and exact unresolved identifiers;
7. unique source item/appearance IDs and unique resolved presentation/sprite identities, labelled migration-only;
8. structurally encountered door/teleport/relocation/waypoint/town-like records without invented gameplay meaning;
9. candidate composite/multi-cell visual patterns as diagnostics only;
10. exact clipping/connectivity reason for every expansion;
11. encoded semantic record bytes where the protected producer already defines them;
12. a deterministic machine-readable summary consumable by #504.

Never collapse cells, total placements and max placements per cell into one metric.

## Expansion rule

Start from the exact Newhaven and Targuna windows. Expand only when a required structure/path/service fixture is clipped at the boundary, and record the exact reason and deterministic expansion order.

Do not build one Newhaven-to-Targuna bounding rectangle. The geographic gap is not evidence of a resident-world resource requirement.

## Acceptance criteria

- [ ] Pinned source identities are verified before measurements are accepted.
- [ ] Existing fullworld producer APIs are reused; no duplicate parser/importer/schema is created.
- [ ] Newhaven and Targuna are measured as separate bounded windows.
- [ ] Tile/non-empty/placement/max-per-cell/floor counts are deterministic and independently reported.
- [ ] Resolved/unresolved identifiers are explicit and machine-readable.
- [ ] Every window expansion is justified by clipping/connectivity evidence.
- [ ] No silent tile/item/stack/presentation loss is accepted.
- [ ] Transition-like records are structural observations only, not invented gameplay semantics.
- [ ] Output gives #504 exact lower-bound corpus dimensions without claiming production maxima.
- [ ] No migration/OTS value is promoted to Global Reference target truth.
- [ ] No proprietary map/source/pixel/screenshot payload is committed.
- [ ] No permanent world/chunk/bundle/GPU/resource layout is selected.
- [ ] No `RESOURCE_LIMITS_REGISTRY.json`, runtime, client/server, Cargo, workflow, Platform, Atlas or META write occurs.
- [ ] If helper code exists, focused deterministic tests cover source identity, window selection, max/max+1/count overflow and deterministic ordering where applicable.
- [ ] Complete final diff self-review reports zero open material findings.
- [ ] Exact-head GitHub checks required for the final changed paths are green.
- [ ] Unresolved review threads/requested changes are zero before integration handoff.

## Excluded scope

No Phase-B Global target observation/capture, Reference parity claim, full-world import, permanent `.omap/.owb` or chunk decision, Content/runtime activation, Movement/Combat implementation, NPC/spawn inference, server/client protocol identifiers, renderer physical-layout decision, production resource maximum, registry mutation, production deployment or cross-repository write.

`tools/game-atlas-fullworld-source/**`, `tools/tibia-worldmap-reconstruction/**`, `apps/**`, `crates/**`, root Cargo files, workflows, public contracts/architecture and resource registries are read-only unless a separately protected #162 allocation amendment explicitly says otherwise.

## Failure behavior

- Exact pinned source unavailable: return `SOURCE_CORPUS_REQUIRED`.
- Digest mismatch: fail closed before accepting output.
- Unresolved critical mapping/appearance: retain explicit unresolved evidence; do not substitute Crystal/Canary/current Global.
- Checked arithmetic failure/overflow: fail closed.
- Ambiguous expansion necessity: record the ambiguity; do not widen toward whole-world scanning.
- Required unallocated path: return `SHARED_LEASE_REQUIRED` or exact allocation-amendment request; do not seize it.
- Material architecture/product decision: return `ARCHITECTURE_ESCALATION_REQUIRED` and stop only the affected lane.

## Validation

### Focused

- command/run: pending after protected admission; if no helper code is added, deterministic census reproduction/validation command only
- result: pending

### Component/integration

- command/run: existing producer contract validation plus exact output-schema/count consistency checks
- result: pending

### E2E

- scenario: `NOT_APPLICABLE` — Phase A is a migration-corpus evidence census and activates no runtime/user journey
- result: `NOT_APPLICABLE`

### Exact-head CI

- final head: pending
- trigger source: pull_request
- workflow/run/job: repository-required checks selected by changed paths
- runner assignment: pending
- classification: pending
- result: pending

## Self-review

- exact head: pending
- method/reviewer: implementing worker, mandatory whole-diff challenge
- material findings: pending
- verdict: pending

## Independent review

- required: pending; resolve from current risk policy and final changed paths, do not invent an extra gate
- exact head: pending or `NOT_APPLICABLE`
- method/auditor: pending or `NOT_APPLICABLE`
- material findings: pending or `NOT_APPLICABLE`
- verdict: pending or `NOT_APPLICABLE`

## PR and closeout

- changed-file review: pending
- unresolved review threads: pending
- related/superseded PRs: none at allocation preflight
- protected Merge Queue: requires fresh exact-candidate owner authorization; not granted by this task record
- merge commit/result: pending
- ownership release: pending after protected integration/readback and #504 evidence handoff

## Context checkpoint

```yaml
last_progress: prospective exact #162 allocation package authored from protected main 1e993fb62cb9912c833f466a021070d9d93d3892
status: waiting
branch: agent/reference-world-corridor-census-511
head_sha: null
pr: null
final_head_sha: null
final_head_frozen_at: null
ci_trigger_source: null
ci_check_generation: null
ci_checks_for_current_head: 0
ci_run_ids: []
ci_job_ids: []
runner_assignment_state: not_started
terminal_ci_wait_started_at: null
terminal_ci_checks_for_current_generation: 0
unchanged_state_checks: 0
identical_failure_retries: 0
repair_cycles_for_current_gate: 0
ci_recovery_actions_for_current_head: 0
stall_warnings: 0
owner_action_required: protected allocation integration before worker admission
blocker: WAITING_PROTECTED_ALLOCATION
next_action: qualify and integrate the exact allocation PR, then perform fresh #162 protected-main admission readback before creating the worker branch
```
