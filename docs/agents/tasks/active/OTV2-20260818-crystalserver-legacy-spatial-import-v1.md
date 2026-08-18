# OTV2-20260818-crystalserver-legacy-spatial-import-v1

```yaml
task_id: OTV2-20260818-crystalserver-legacy-spatial-import-v1
title: Define CrystalServer legacy spatial import profile v1
mode: CONTRACT
status: validating
repository: blakinio/Oteryn-v2
base_branch: main
branch: architecture/OTV2-20260818-crystalserver-legacy-spatial-import-v1
pr: 329
base_sha: 16c665223c1256cc7e4a8a97cf2bc34cd278423c
head_sha: e7a5cdf3c17a9f033f5062adb36f1e648163338e
owner: ChatGPT autonomous execution
created_at: 2026-08-18T09:31:00+02:00
updated_at: 2026-08-18T09:36:00+02:00
execution_budget_minutes: 60
owned_paths:
  - docs/contracts/OTERYN_CRYSTALSERVER_LEGACY_SPATIAL_IMPORT_PROFILE_V1.md
  - docs/agents/evidence/OTV2-20260818-crystalserver-legacy-spatial-import-readiness.md
  - docs/agents/tasks/active/OTV2-20260818-crystalserver-legacy-spatial-import-v1.md
public_contracts:
  - oteryn-crystalserver-legacy-spatial-import-v1
depends_on:
  - docs/contracts/OTERYN_WORLD_SPATIAL_COORDINATE_PROFILE_V1.md
  - docs/contracts/OTERYN_GAME_ATLAS_EXPORT_CONTRACT_V1.md
  - blakinio/Otheryn@e417c5e7c22986bf4acef0495eb47f7b72c97cce
  - opentibiabr/otclient@dd5641492a71e966b96b8a91398b44bb3df67d88
blocks:
  - DYN-ATLAS-001 legacy Thais Z7 coordinate/floor/presentation-order gate
cross_repository_coordination_id: OTERYN-GAME-ATLAS-V1-LEGACY-IMPORT
external_repositories:
  - Oteryn/Oteryn-Atlas
  - blakinio/Otheryn
  - opentibiabr/otclient
```

## Goal

Define one bounded Game-owned importer profile that maps the pinned CrystalServer/Tibia legacy spatial representation used by the historical Otheryn Atlas pipeline into `oteryn-world-spatial-v1` without making OTBM/Tibia semantics canonical.

## Acceptance criteria

- [x] Current Game main, root governance, active tasks and overlapping PRs were refreshed.
- [x] The accepted native spatial profile remains authoritative.
- [x] Legacy source revision/parser/render evidence is pinned.
- [x] Independent OTClient floor-direction evidence is pinned.
- [x] Horizontal x/y mapping is explicit and representation-only.
- [x] Legacy z mapping is explicit and preserves the native vertical-order rule.
- [x] Legacy same-position visible stack order maps to explicit `PresentationOrderKey` values without consumer inference.
- [x] Container descendants are explicitly excluded from static spatial presentation ordering.
- [x] The Thais legacy regression selection has an exact native half-open rectangle/floor mapping.
- [x] Asset anchor/displacement/redistribution decisions remain separate and fail-closed.
- [x] Full three-file diff self-review found no unresolved material semantic/scope finding after strengthening the floor-direction evidence.
- [ ] Exact-head repository governance/architecture checks pass.

## Architecture/evidence basis

### PROVEN

- `OTERYN_WORLD_SPATIAL_COORDINATE_PROFILE_V1` requires explicit versioned legacy conversion profiles and defines native +X east, +Y south, larger `FloorId` = geometrically higher, half-open rectangles, explicit presentation ordering, and no special meaning for floor zero.
- Otheryn legacy evidence pins CrystalServer `zimbadev/crystalserver@5e89bf8329ea406cb4ea8f4a18f32954f13e5418`, `world.otbm` SHA-256 `3bd40d14fefec41f24c4b3ae879e420be1a831ef55b95dcbec721e587a09b034`, and historical Thais `X=32280..32440`, `Y=32155..32305`, `Z=7`.
- Otheryn semantic parser preserves legacy x/y/z and the visible top-level item sequence; renderer uses optional ground followed by top-level tile items and excludes nested container contents from visible map entries.
- Otheryn renderer maps increasing x right/east and increasing y down/south.
- `opentibiabr/otclient@dd5641492a71e966b96b8a91398b44bb3df67d88` independently establishes the legacy Z direction: `SEA_FLOOR=7`, floors `0..7` surface/above, `8..15` underground, and `z < 7` above Z7.

### DERIVED

- `FloorId = -legacy_z` is the smallest order-preserving adapter into native larger-is-higher semantics; legacy Z7 therefore maps to native FloorId -7 without assigning a universal meaning to native floor zero.
- Legacy inclusive Thais bounds map to native half-open `x=[32280,32441)`, `y=[32155,32306)`, `floor=-7`.
- `PresentationOrderKey { plane: 0, order: sequence_index }` preserves the pinned visible legacy draw sequence while keeping Game interaction/collision authority separate.

### UNKNOWN / deliberately deferred

- Asset pixel redistribution rights.
- Canonical Game appearance spatial-unit profile for the pinned Tibia asset set.
- Whether future/non-CrystalServer legacy inputs can use this profile unchanged.
- Physical Game->Atlas serialization/chunking.

## Excluded scope

No OTBM parser/runtime implementation in Game; no Atlas/browser implementation; no permanent serializer/chunk/floor packing decision; no asset redistribution; no production deployment; no legacy IDs promoted to canonical identities; no collision/interaction/movement semantics.

## Validation

### Focused

- live Game/Otheryn/Atlas/OTClient authority preflight: **PASS**;
- changed paths: exactly the three declared owned documentation paths;
- full exact PR patch self-review after OTClient evidence hardening: **PASS**, zero unresolved material findings;
- runtime/component/E2E: **NOT_APPLICABLE** — documentation-only importer semantic contract.

### Exact-head CI

Pending on the checkpoint head produced by this update. All required repository-selected governance/architecture/merge checks must pass on the unchanged exact head before Ready/merge.

## Context checkpoint

```yaml
checkpoint_version: 1
updated_at: 2026-08-18T09:36:00+02:00
head: e7a5cdf3c17a9f033f5062adb36f1e648163338e
branch: architecture/OTV2-20260818-crystalserver-legacy-spatial-import-v1
pr: 329
status: validating
context_routes:
  - architecture
  - agent-governance
  - testing
owned_paths:
  - docs/contracts/OTERYN_CRYSTALSERVER_LEGACY_SPATIAL_IMPORT_PROFILE_V1.md
  - docs/agents/evidence/OTV2-20260818-crystalserver-legacy-spatial-import-readiness.md
  - docs/agents/tasks/active/OTV2-20260818-crystalserver-legacy-spatial-import-v1.md
proven:
  - Native spatial semantics and independent legacy Z-direction evidence are pinned.
  - Legacy parser/render evidence establishes exact visible source sequence and Thais selection.
derived:
  - FloorId = -legacy_z and plane-0/order-index are bounded importer mappings, not canonical legacy inheritance.
unknown:
  - asset rights and appearance spatial-unit profile
conflicts: []
first_failure:
  marker: dyn-atlas-legacy-import-profile-missing
  evidence: DYN-ATLAS-001 required Game-owned conversion authority before using legacy Z7/source order
rejected_hypotheses:
  - use legacy z and item order directly as Atlas authority
  - define legacy z=7 as universal native floor zero
  - copy Tibia assets as part of this contract task
changed_paths:
  - docs/contracts/OTERYN_CRYSTALSERVER_LEGACY_SPATIAL_IMPORT_PROFILE_V1.md
  - docs/agents/evidence/OTV2-20260818-crystalserver-legacy-spatial-import-readiness.md
  - docs/agents/tasks/active/OTV2-20260818-crystalserver-legacy-spatial-import-v1.md
validation:
  - command: full exact PR #329 patch review
    result: PASS
    evidence: three owned docs paths; no unresolved semantic/scope finding after OTClient evidence hardening
  - command: runtime/component/E2E
    result: NOT_APPLICABLE
    evidence: documentation-only semantic importer profile; no executable runtime changed
blockers: []
next_action: observe exact-head repository checks; if all required gates pass with no review findings, mark PR #329 Ready and squash-merge
```

## Source branch closeout

```yaml
source_branch_disposition: auto_delete_after_merge
source_branch_reason: ordinary same-repository contract task
source_branch_evidence: pending
```
