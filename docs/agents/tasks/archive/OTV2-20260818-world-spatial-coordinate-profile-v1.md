# OTV2-20260818-world-spatial-coordinate-profile-v1 — COMPLETED

```yaml
task_id: OTV2-20260818-world-spatial-coordinate-profile-v1
title: Freeze canonical World spatial / coordinate profile v1
mode: CONTRACT
status: completed
repository: blakinio/Oteryn-v2
base_branch: main
branch: architecture/OTV2-20260818-world-spatial-coordinate-profile-v1
pr: 327
base_sha: 5577f6fc7c1f7ddef482f0f7b08039047704e36b
final_head_sha: 4789566086b7872c11adee4fe0eda7b4eef46491
merge_sha: 71177560a86a6c1c98b539d92e52981cc3739254
closed_at: 2026-08-18T06:53:11+02:00
cross_repository_coordination_id: OTERYN-GAME-ATLAS-V1-SPATIAL
public_contracts:
  - oteryn-world-spatial-v1
```

## Terminal result

PR #327 was marked Ready only after the exact final head passed the required repository gates, then squash-merged to `main` as `71177560a86a6c1c98b539d92e52981cc3739254`.

The merge establishes `docs/contracts/OTERYN_WORLD_SPATIAL_COORDINATE_PROFILE_V1.md` as the accepted Game-owned spatial/coordinate semantic profile v1 under its own merge-acceptance rule.

This closes the canonical coordinate/floor/order/anchor semantic evidence gap previously recorded by `docs/agents/evidence/OTV2-20260816-game-atlas-physical-profile-readiness.md` without rewriting that historical evidence.

## Accepted semantic result

The accepted profile defines:

- `WorldTilePosition { x: i32, y: i32, floor: i16 }`;
- canonical north-up orientation: +X east/right, +Y south/down;
- finite per-world half-open horizontal bounds using a wider semantic bounds envelope so the full `i32` position domain can be represented safely;
- explicit finite valid `FloorId` set, with larger values geometrically higher/above and no universal special meaning assigned to floor zero;
- half-open rectangle membership;
- explicit unique same-position `PresentationOrderKey { plane: i32, order: u32 }`;
- separate stable placement identity and presentation order;
- explicit anchor-relative multi-tile footprint semantics;
- displacement direction/meaning with precision supplied by a versioned appearance/asset `units_per_tile` profile;
- deterministic spatial canonicalization order independent from rendering order;
- mandatory explicit versioned legacy conversion mapping rather than direct promotion of OTBM/Tibia `z`/stack conventions.

## Decision discipline

The final profile compares three realistic options and selects the explicit native Oteryn profile with versioned legacy adapters over:

1. making Tibia/OTBM coordinates/stack semantics canonical; or
2. leaving axes/floors/order consumer-defined.

Serializer, compression, permanent chunk dimensions, floor packing, concrete world extents/floor list, legacy `Z7` mapping, pathfinding/collision encoding, sprite pixel/displacement precision, asset licensing and live-state transport remain deliberately separate evidence-gated decisions.

## Self-review and repair history

Three material pre-freeze findings were repaired before readiness:

1. half-open bounds using only `i32` endpoints could not represent the exclusive edge after `i32::MAX`;
2. fixed `1/256 tile` displacement unnecessarily froze an asset/render precision;
3. the first proposal omitted the explicit alternatives/trade-offs section required by `ARCHITECTURE_DECISION_DISCIPLINE.md`.

The semantic content head `6192f37aa58e724fbd5be88facd7a277dedf3820` then passed full owned-path self-review with zero unresolved material findings. Final PR head `4789566086b7872c11adee4fe0eda7b4eef46491` added only the validation checkpoint.

## Exact-head validation

```yaml
final_head: 4789566086b7872c11adee4fe0eda7b4eef46491
changed_files: 3
changed_path_scope: PASS
full_diff_self_review: PASS
component_integration: NOT_APPLICABLE_DOCS_ONLY
runtime_e2e: NOT_APPLICABLE_DOCS_ONLY
agent_governance:
  run_id: 32100661763
  run_number: 1605
  result: SUCCESS
merge_gate:
  run_id: 32100661761
  run_number: 461
  result: SUCCESS
architecture_semantic_audit:
  run_id: 32100652762
  run_number: 195
  result: SUCCESS
merge_authority_audit:
  run_id: 32100652713
  run_number: 420
  result: SUCCESS
review_submissions_before_merge: 0
review_threads_before_merge: 0
pr_comments_before_merge: 0
```

An earlier Agent Governance generation failed only because the draft PR body omitted the mandatory `## Summary` heading. PR metadata was repaired without altering semantic content; the final exact head subsequently passed Agent Governance and the aggregate Merge Gate.

## Merge evidence

```yaml
pr: 327
ready_before_merge: true
expected_head_merge_guard: 4789566086b7872c11adee4fe0eda7b4eef46491
merge_method: squash
merge_sha: 71177560a86a6c1c98b539d92e52981cc3739254
main_verified_after_merge: 71177560a86a6c1c98b539d92e52981cc3739254
```

## DYN-ATLAS effect

**PROVEN:** the previously missing canonical Game spatial/coordinate semantic authority is now accepted on protected `main`.

A later Game-owned fixture/conversion/export profile may bind an exact source selection to `coordinate_profile = "oteryn-world-spatial-v1"` without Atlas inventing floor/order semantics.

**UNKNOWN / separate:**

- exact physical `Oteryn-Atlas` implementation repository/authority;
- exact pinned Thais Z7 source selection and conversion mapping;
- asset/sprite rights and provenance for visual fixtures;
- executable Game exporter and Atlas consumer;
- production serializer/compression/container and resource ceilings.

The DYN-ATLAS execution prompt still forbids substituting Oteryn-Platform or legacy Otheryn for a missing authorized Atlas implementation repository.

## Ownership release

The implementation branch/PR is terminal. This archive record replaces the active task record and releases ownership of:

- `docs/contracts/OTERYN_WORLD_SPATIAL_COORDINATE_PROFILE_V1.md`;
- `docs/agents/evidence/OTV2-20260818-game-atlas-coordinate-profile-readiness.md`;
- the task record path.

## Next action

Refresh DYN-ATLAS-001 against live Platform programme authority and the actual available/authorized Atlas implementation repository. If no physical `Oteryn-Atlas` target exists, stop on that exact repository-authority condition rather than implementing the Atlas runtime in Platform or legacy Otheryn by assumption.
