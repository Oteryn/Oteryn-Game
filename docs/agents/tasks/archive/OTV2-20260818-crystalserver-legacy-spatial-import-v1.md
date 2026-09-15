# OTV2-20260818-crystalserver-legacy-spatial-import-v1 — COMPLETED

```yaml
task_id: OTV2-20260818-crystalserver-legacy-spatial-import-v1
status: completed
repository: blakinio/Oteryn-v2
pr: 329
base_sha: 16c665223c1256cc7e4a8a97cf2bc34cd278423c
final_head_sha: 5cc065b0c1ed36113fb65f97ed327af2fb8ebe37
merge_sha: 227e4e3bd64c3911280c8388e2a833cb210f24fd
cross_repository_coordination_id: OTERYN-GAME-ATLAS-V1-LEGACY-IMPORT
public_contracts:
  - oteryn-crystalserver-legacy-spatial-import-v1
```

## Terminal result

PR #329 squash-merged the Game-owned `OTERYN_CRYSTALSERVER_LEGACY_SPATIAL_IMPORT_PROFILE_V1.md` after exact-head validation. The accepted profile maps the pinned legacy CrystalServer/Tibia representation into native Oteryn spatial semantics without making OTBM authoritative.

For DYN-ATLAS-001 the historical Thais source selection `X=32280..32440`, `Y=32155..32305`, legacy `Z=7` maps to native half-open `x=[32280,32441)`, `y=[32155,32306)`, `FloorId=-7`; visible ground/top-level item order maps to explicit `PresentationOrderKey { plane: 0, order: sequence_index }`. Nested container contents are not visible spatial stack entries.

## Exact-head validation

- Agent Governance #1612: SUCCESS
- Architecture Semantic Audit #202: SUCCESS
- Merge Authority Audit #425: SUCCESS
- Merge Gate #466: SUCCESS, including exact-head scope/governance/CodeQL/aggregate validation
- changed paths: exactly 3 declared documentation paths
- full-diff self-review: PASS, zero unresolved material findings
- review threads: 0
- submitted reviews: 0
- PR comments: 0
- runtime/component/E2E: NOT_APPLICABLE (documentation-only semantic contract)

## Remaining DYN-ATLAS gates

This completed contract does not authorize asset redistribution, appearance spatial metadata, physical Game->Atlas serialization, or a bounded Game-owned semantic export fixture. Those gates remain separately owned and fail closed.

## Source branch closeout

PR #329 source branch was absent after merge, consistent with automatic deletion.
