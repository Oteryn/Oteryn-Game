# OTV2-20260818-tibia-15-25-appearance-spatial-profile-v1 — COMPLETED

```yaml
task_id: OTV2-20260818-tibia-15-25-appearance-spatial-profile-v1
title: Define Tibia 15.25 appearance spatial profile v1
mode: CONTRACT
status: completed
repository: blakinio/Oteryn-v2
pr: 331
base_sha: ade3005cdf9ad6daeb87dc20a6546d5c29ee61da
final_head_sha: 819f14306c9dc26fd5b25da56f460f4cf7c92a8d
merge_sha: f90b99bee9567800a430ca1a703843d647dc01b8
closed_at: 2026-08-18T10:20:00+02:00
cross_repository_coordination_id: OTERYN-GAME-ATLAS-V1-APPEARANCE
public_contracts:
  - oteryn-tibia-15-25-appearance-spatial-v1
```

## Terminal result

PR #331 merged the bounded Game-owned appearance presentation profile for the exact owner-authorized Tibia client archive SHA-256 `01c45146e2fcec3f4087844e0cbc1817fb1d60b310a35ac5d88c07aab6f73d1a`.

That archive directly identified itself as `15.25.bd5a04` and decoded to 42,107 object appearances, 4,927 sprite sheets and 75,623 unique object-referenced sprite IDs.

The accepted profile defines bounded presentation semantics for **that exact digest only**:

- `units_per_tile = 32`;
- south-east/bottom-right visual anchor cell;
- west/north visual coverage for multi-tile decoded sprites;
- effective 2D displacement `(-(shift_x + height), -(shift_y + height))`;
- producer-owned concrete frame/phase/layer/sprite resolution;
- pixel visual coverage explicitly not treated as gameplay occupancy/collision authority.

## Exact-head validation

Final head `819f14306c9dc26fd5b25da56f460f4cf7c92a8d` passed Agent Governance #1620, Architecture Semantic Audit #210, Merge Authority Audit #430 and Merge Gate #472 before squash merge `f90b99bee9567800a430ca1a703843d647dc01b8`.

## Post-merge source correction

After PR #331 merged, the project owner correctly identified that the repository's **latest Atlas build path** does not use the separately uploaded 15.25 archive. `blakinio/Otheryn@e417c5e7c22986bf4acef0495eb47f7b72c97cce` pins a distinct Google Drive bundle under project label `15.32`:

- Drive file id `1Dlo3bS4K1nS3mw4BhPZdlHT7lX5zRAvv`;
- expected ZIP SHA-256 `1a6bad8b7598cd874f534cd4aae2d249fb3d9b4458b3ccfa75754f91bb27870f`.

A one-shot verification in `Oteryn/Oteryn-Atlas` downloaded that exact bundle and proved:

- verification run `32115353176`, job `95643523890`, artifact `9316452985`;
- top-level archive content is `assets/` only; there is no `package.json`, so `15.32` is the repository-pinned project label rather than an internal package-version field;
- catalog SHA-256 `35639e000c4c108665a091cfbdf699d549d995b37670bc08de575ab6cd380d85`;
- appearance SHA-256 `dc4f4c01e3701c77877c67895168e4399837046122d6d17e3e608a12a2fed075`;
- 5,090 catalog entries;
- 43,514 object appearances / frame records;
- 5,084 sprite sheets;
- 111,957 total object sprite refs;
- 79,269 unique object sprite IDs;
- 567 appearances with shift metadata;
- 2,192 appearances with height metadata;
- sprite layouts `32x32`, `32x64`, `64x32`, `64x64`;
- sprite id domain `0..301200`.

The metadata-only durable evidence was merged to `Oteryn/Oteryn-Atlas` as commit `0b56d9a95279f1ec02fddd0dfcf8bd6ffd16b539` at `docs/evidence/DYN-ATLAS-001-15-32-drive-asset-profile.json`.

## DYN-ATLAS routing

The 15.25 contract remains valid as a bounded source-conversion profile for its exact archive. It is **not** the active appearance source for `DYN-ATLAS-001`.

DYN-ATLAS-001 must use a separate Game-owned appearance profile pinned to the verified 15.32 Drive bundle. No field, count, digest or authorization from the 15.25 contract may be silently generalized to the 15.32 source.
