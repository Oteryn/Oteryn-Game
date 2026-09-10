# OTV2-20260910-reference-world-corridor-census-511

```yaml
task_id: OTV2-20260910-reference-world-corridor-census-511
title: Measure bounded Newhaven and Targuna migration-corpus windows
mode: MIGRATE
status: active
repository: Oteryn/Oteryn-Game
issue: 511
base_branch: main
branch: agent/reference-world-corridor-census-511
pr: 525
base_sha: 43ff3341e079f2883b78d01db5cee649290d90be
head_sha: pending post-resume commit
final_head_sha: null
final_head_frozen_at: null
owner: REFERENCE_WORLD_CORRIDOR_CENSUS_511
created_at: 2026-09-10T08:40:00+02:00
updated_at: 2026-09-10T10:45:00+02:00
execution_policy: continuous_progress
owned_paths:
  - tools/reference-world-corridor-census/**
  - docs/agents/evidence/OTV2-20260910-reference-world-corridor-census.md
  - docs/agents/tasks/active/OTV2-20260910-reference-world-corridor-census-511.md
public_contracts: []
depends_on:
  - Oteryn/Oteryn-Game#162
  - Oteryn/Oteryn-Game#511
  - Oteryn/Oteryn-Game#523
blocks:
  - Oteryn/Oteryn-Game#504
cross_repository_coordination_id: null
external_repositories:
  - blakinio/Otheryn@e417c5e7c22986bf4acef0495eb47f7b72c97cce (READ_ONLY_MIGRATION_EVIDENCE)
```

## Protected activation evidence

This task is **ACTIVE_PHASE_A** under the single protected allocation:

- allocation PR #523 exact candidate `e9251de77dd1a7d011e4ad6e3e3323c2d9602fe9`;
- protected allocation/main commit `81ff001609c730b1dca1b28c431199045dec00f3`;
- real merge-group run `34445097627` = SUCCESS;
- merge-group aggregate `game-gate` job `102770140001` = SUCCESS;
- protected allocation path `docs/agents/programs/OTV2_REFERENCE_WORLD_CORRIDOR_CENSUS_511_ALLOCATION_20260910.md` read back from protected main;
- explicit #162 activation comment `5614294048` after fresh overlap/custody reconciliation;
- concurrent prospective allocation PR #522 is CLOSED UNMERGED as superseded and grants no canonical worker authority.

Material work is restricted to the exact owned paths above. Existing producer/reconstruction/runtime/registry/workflow/Cargo/Platform/Atlas paths remain read-only.

## Protected repair and resume evidence

- protected repaired allocation/main: `1fdc37fdb8b8faf2aaab99f17a25c7a6e986f7c5`;
- repaired allocation PR #526 exact head: `254fcd098549d5630357e26f0689ab66b7498aea`;
- real merge-group run `34455053189` = SUCCESS;
- explicit #162 resume comment: `5615644586`;
- preserved pre-repair branch history through `f6605bfb482649dbb381b2dfd76f1ba1184242ae`;
- every material pre-resume result remains `PRE_REPAIR_UNQUALIFIED / NOT_ACCEPTED_EVIDENCE`.

## Outcome

Produce the smallest deterministic Phase-A migration-corpus census needed for the first Reference Newhaven/Targuna corridor evidence. Measure Newhaven and Targuna as **two separate bounded source windows** through the existing protected Game fullworld producer APIs, preserve exact source provenance, and emit machine-readable lower-bound measurements suitable for downstream #504.

The result is migration/source evidence only. It is not a playable world import, not Global target geometry parity, not production resource maxima and not Evolved content.

## Architecture and source authority

- **PROVEN:** #162 / `OTV2_WORK_DELIVERY_COORDINATOR` is the active product control plane.
- **PROVEN:** protected #523 is the sole allocation for this material worker.
- **PROVEN:** `tools/game-atlas-fullworld-source/producer.py` owns exact pinned source validation and ordered semantic tile projection and must be consumed read-only.
- **PROVEN:** `tools/tibia-worldmap-reconstruction/**` remains the later Phase-B comparison family and is read-only in Phase A.
- **PROVEN:** no second OTBM parser/importer/fullworld exporter is required or authorized.
- **PROVEN:** protected `.github/workflows/game-atlas-thais-fixture.yml` provides the repository-approved hosted source-acquisition pattern: pinned legacy checkout, exact map SHA-256, exact Drive file ID download, exact ZIP/catalog/appearance digest verification before projection.
- **PROVEN:** connected Drive source file id `1Dlo3bS4K1nS3mw4BhPZdlHT7lX5zRAvv` was independently streamed during activation preflight; size `246811594` and SHA-256 `1a6bad8b7598cd874f534cd4aae2d249fb3d9b4458b3ccfa75754f91bb27870f` match protected source evidence.
- **UNKNOWN:** exact Global target geometry for Newhaven/Targuna migration locators; remain `OTS_HYPOTHESIS_ONLY` until later #483-governed target observation.
- **UNKNOWN:** production hard maxima derived from census counts; #504 owns later evidence-backed resource disposition.

### Exact pinned migration source

```text
legacy repository:
  blakinio/Otheryn@e417c5e7c22986bf4acef0495eb47f7b72c97cce
world.otbm SHA-256:
  3bd40d14fefec41f24c4b3ae879e420be1a831ef55b95dcbec721e587a09b034
Drive asset file id:
  1Dlo3bS4K1nS3mw4BhPZdlHT7lX5zRAvv
15.32.zip SHA-256:
  1a6bad8b7598cd874f534cd4aae2d249fb3d9b4458b3ccfa75754f91bb27870f
catalog SHA-256:
  35639e000c4c108665a091cfbdf699d549d995b37670bc08de575ab6cd380d85
appearance SHA-256:
  dc4f4c01e3701c77877c67895168e4399837046122d6d17e3e608a12a2fed075
```

All exact digests must be reverified on the worker execution surface before counts are accepted.

### Exact Phase-A starts

```text
Newhaven
  semantic_record=3a6d2a5cfd599439efccce35017e12b8
  locator=(32536,32514)
  native_floor=-7
  semantic_shard=f-7-r1016-c1016
  fullworld_region=fm000007_rxp000127_ryp000127

Targuna
  semantic_record=57d9ef64607b0360d0c01d338f7c70ba
  locator=(31934,31925)
  native_floor=-7
  semantic_shard=f-7-r997-c997
  fullworld_region=fm000007_rxp000124_ryp000124
```

These are migration-source locators, never automatic `GLOBAL_REFERENCE_TARGET` coordinates.

## Required implementation shape

Prefer the smallest reproducible path:

```text
protected game-atlas-fullworld-source producer APIs (read-only)
+ exact source/digests
+ Newhaven and Targuna bounded selectors
-> thin deterministic census consumer only if needed
-> machine-readable summary
-> public-safe evidence document
```

If tracked helper code is needed it must live only under `tools/reference-world-corridor-census/**`, consume the producer API rather than copy it, and have focused deterministic tests. It must not parse OTBM independently, reconstruct appearance semantics, select a new world schema or add a workflow.

## Required measurements

For Newhaven and Targuna independently report:

1. exact source/digest/producer revision;
2. starting and final source-window/shard/region set plus deterministic expansion order;
3. floors touched;
4. total tile records and non-empty tile records;
5. total ordered presentation/placement records and maximum placements per cell;
6. resolved/unresolved appearance/presentation totals and exact unresolved identifiers;
7. unique source item/appearance IDs and unique resolved presentation/sprite identities, migration-only;
8. town/waypoint and door/teleport/relocation-like source observations without invented gameplay semantics;
9. candidate composite/multi-cell visual patterns as diagnostics only;
10. exact clipping/connectivity reason for every expansion;
11. encoded semantic record bytes where the protected producer already defines them;
12. deterministic machine-readable summary sufficient for #504 lower-bound resource decisions.

Never collapse `cells`, `total placements`, `max placements per cell`, `unique identities` or `encoded bytes` into one resource.

## Expansion rule

Start from each exact bounded locator window. Expand only when an exercised structure/path/service fixture is demonstrably clipped by the current boundary and record the exact deterministic reason/order.

Never build one Newhaven-to-Targuna rectangle. The geographic gap is not a resident-world resource requirement.

## Required RED/GREEN validation

If helper code is introduced, first prove failing tests for the missing bounded census behavior, then minimal GREEN. At minimum cover:

- exact inclusive/exclusive window boundaries;
- floor filtering;
- stable results independent of hash/container enumeration;
- deterministic expansion order;
- explicit unresolved appearance propagation;
- no silent loss of ordered presentations;
- checked count/byte arithmetic and overflow rejection;
- exact source/digest mismatch fail-closed;
- deterministic machine-readable summary;
- max/max+1 where a local fixed helper bound exists.

Also run existing fullworld producer compile/self-tests relevant to the reused API. Do not change that producer to make the consumer pass.

## Acceptance criteria

- [ ] Exact pinned map/asset/catalog/appearance identities verified on execution surface before measurements accepted.
- [ ] Existing fullworld producer API reused; no duplicate parser/importer/schema.
- [ ] Newhaven and Targuna measured as separate bounded windows.
- [ ] Tile/non-empty/placement/max-per-cell/floor counts independently reported.
- [ ] Resolved/unresolved identifiers explicit and machine-readable.
- [ ] Every expansion justified by clipping/connectivity evidence.
- [ ] No silent tile/item/stack/presentation loss.
- [ ] Transition-like records remain structural observations only.
- [ ] #504 receives exact lower-bound corpus dimensions without production-maxima claim.
- [ ] No migration/OTS value promoted to Global Reference truth.
- [ ] No proprietary map/source/pixel/screenshot bytes committed.
- [ ] No permanent world/chunk/bundle/GPU/resource layout selected.
- [ ] No registry/runtime/client/server/Cargo/workflow/Platform/Atlas/META write.
- [ ] Complete final diff self-review has zero open material findings.
- [ ] Exact-head repository checks are green.
- [ ] Unresolved review threads/requested changes are zero before integration handoff.

## Excluded scope

No Phase-B Global capture/observation, target parity claim, production Content/world runtime, full-world import, permanent `.omap/.owb` or chunk format, Movement/Combat/NPC runtime, server/client protocol IDs, renderer physical-layout decision, production resource hard maximum, registry mutation, deployment, external repository write or Evolved behavior.

## Failure behavior

- exact pinned source unavailable on the active authorized execution surface: `SOURCE_CORPUS_REQUIRED`;
- legacy checkout, interpreter, module-origin, tracking or loaded-blob mismatch: `LEGACY_PARSER_REVISION_MISMATCH` before evidence acceptance;
- digest mismatch: fail closed before output acceptance;
- unresolved critical mapping/appearance: preserve explicit unresolved evidence; do not substitute another source;
- count/byte arithmetic failure: fail closed;
- proven or ambiguous expansion necessity: `WINDOW_EXPANSION_REQUIRED` with the minimal proposed shard/floor; stop before counting it;
- unallocated path needed: `SHARED_LEASE_REQUIRED` with exact path/reason;
- material architecture decision needed: `ARCHITECTURE_ESCALATION_REQUIRED` for only the affected lane.

## Validation

### Focused

- command/run: `python tools/reference-world-corridor-census/self_test.py`
- result: PASS after recorded missing-module RED; parser provenance/contamination, boundary/floor/order/unresolved/losslessness/stream-framing/overflow/digest negatives all GREEN
- command/run: `python -m py_compile tools/reference-world-corridor-census/census.py tools/reference-world-corridor-census/self_test.py`
- result: PASS

### Component/integration

- command/run: `python -m py_compile tools/game-atlas-fullworld-source/producer.py tools/game-atlas-fullworld-source/self_test.py && python tools/game-atlas-fullworld-source/self_test.py`
- result: PASS
- command/run: two independent exact pinned-corpus census executions followed by `cmp /tmp/census-post-resume-a.json /tmp/census-post-resume-b.json`
- result: PASS; post-resume canonical SHA-256 `9b34dc8ad4662032e9c09201514609810375c3c49c0b7e3026b1878e50889610`

### E2E

- scenario: `NOT_APPLICABLE` — Phase A is migration-corpus evidence only
- result: `NOT_APPLICABLE`

### Exact-head CI

- final head: pending
- trigger source: pull_request
- workflow/run/job: pending
- result: pending

## Self-review

- implementation head: `05c9c94dbc44d7045f840814be8f24894139d62a`; task-record follow-up commit pending
- material findings: 0 after changed-file and whole-diff review
- verdict: PASS

## Independent review

- required: resolve from current risk policy and final changed paths
- exact head: pending or `NOT_APPLICABLE`
- verdict: pending or `NOT_APPLICABLE`

## Phase-A measured result

- Newhaven: one 32x32 shard, 1,024 cells/tile records, 1,199 ordered presentations, max 5/cell, 70 appearance IDs, 84 sprite IDs, 899,694 semantic bytes, zero unresolved.
- Targuna: one 32x32 shard, 1,024 cells/tile records, 1,264 ordered presentations, max 5/cell, 156 appearance IDs, 190 sprite IDs, 939,759 semantic bytes, zero unresolved.
- Newhaven direct-concatenation stream: 899,694 bytes, max record 3,080 bytes, SHA-256 `bebce3eca44dfcf3ab4044fd7a4a66bf6c1f7a80e2b0d375de14c3eb3898faac`.
- Targuna direct-concatenation stream: 939,759 bytes, max record 3,079 bytes, SHA-256 `5e9a498a37570662f57d24ec8ca020c521ccf56091b03e33970aa6e2a1394438`.
- Four-edge occupancy is `EDGE_OCCUPANCY_ONLY`, not clipping or expansion evidence; no `WINDOW_EXPANSION_REQUIRED` finding exists and no adjacent shard was counted.
- Canonical details: `tools/reference-world-corridor-census/phase-a-summary.json` and public-safe evidence document.
- `phase_b_target_parity=NOT_PERFORMED`; `registry_maxima_selected=false`; `production_authority=NONE`.

## Context checkpoint

```yaml
last_progress: post-repair resume reconciliation, parser provenance validation, canonical stream evidence, focused tests, and exact real-corpus double run complete
status: active
branch: agent/reference-world-corridor-census-511
head_sha: pending post-resume commit
pr: 525
final_head_sha: pending exact-head CI
final_head_frozen_at: null
ci_trigger_source: pull_request
ci_check_generation: pending push
ci_checks_for_current_head: 0
ci_run_ids: []
ci_job_ids: []
runner_assignment_state: not_started
owner_action_required: none
blocker: null
phase_a_result: PASS
phase_b_target_parity: NOT_PERFORMED
production_authority: NONE
next_action: commit and push the same branch, update Draft PR #525, and obtain exact-head repository checks/review; do not begin Phase B
```
