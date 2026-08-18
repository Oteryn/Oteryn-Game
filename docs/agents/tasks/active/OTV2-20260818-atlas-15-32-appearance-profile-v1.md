# OTV2-20260818-atlas-15-32-appearance-profile-v1

```yaml
task_id: OTV2-20260818-atlas-15-32-appearance-profile-v1
title: Define Atlas 15.32 Drive appearance spatial profile v1
mode: CONTRACT
status: validating
repository: blakinio/Oteryn-v2
base_branch: main
branch: architecture/OTV2-20260818-atlas-15-32-appearance-profile-v1
pr: 333
base_sha: 2c77f0c373157312b9979dc01806acd56f4949a4
reviewed_content_head: c7d805941d2c9e2953f25fa95fe35a8880323301
owner: ChatGPT autonomous execution
created_at: 2026-08-18T10:23:00+02:00
updated_at: 2026-08-18T10:28:00+02:00
execution_budget_minutes: 60
owned_paths:
  - docs/contracts/OTERYN_ATLAS_15_32_APPEARANCE_SPATIAL_PROFILE_V1.md
  - docs/agents/evidence/OTV2-20260818-atlas-15-32-appearance-readiness.md
  - docs/agents/tasks/active/OTV2-20260818-atlas-15-32-appearance-profile-v1.md
public_contracts:
  - oteryn-atlas-15-32-appearance-spatial-v1
depends_on:
  - docs/contracts/OTERYN_WORLD_SPATIAL_COORDINATE_PROFILE_V1.md
  - docs/contracts/OTERYN_CRYSTALSERVER_LEGACY_SPATIAL_IMPORT_PROFILE_V1.md
  - docs/contracts/OTERYN_GAME_ATLAS_EXPORT_CONTRACT_V1.md
  - Oteryn/Oteryn-Atlas@0b56d9a95279f1ec02fddd0dfcf8bd6ffd16b539
  - blakinio/Otheryn@e417c5e7c22986bf4acef0495eb47f7b72c97cce
blocks:
  - DYN-ATLAS-001 latest-source appearance visual-anchor/coverage/displacement gate
cross_repository_coordination_id: OTERYN-GAME-ATLAS-V1-APPEARANCE-15-32
external_repositories:
  - Oteryn/Oteryn-Atlas
  - blakinio/Otheryn
```

## Goal

Define the smallest Game-owned appearance presentation profile for the exact repository-selected Google Drive asset bundle `15.32.zip`, replacing the incorrect DYN routing through the separate 15.25 fixture without rewriting or invalidating the bounded 15.25 contract.

## Acceptance criteria

- [x] Current Game main and active-task state refreshed after 15.25 closeout.
- [x] Exact Otheryn latest-build selection pins label `15.32`, Drive file ID and expected ZIP SHA.
- [x] Google Drive metadata for the exact file ID independently identifies the object as `15.32.zip`, size `246811594` bytes.
- [x] Exact Drive ZIP digest independently verified by one-shot Atlas CI.
- [x] Missing internal `package.json` is recorded; no fabricated internal package-version field is claimed.
- [x] Catalog/appearance digests and deterministic source counts recorded from the exact Drive bytes.
- [x] Difference from the separate 15.25 fixture is explicit.
- [x] `units_per_tile`, visual anchor/coverage and displacement conversion are explicit.
- [x] Pixel visual coverage is explicitly forbidden from becoming gameplay occupancy/collision authority.
- [x] Concrete frame/phase/pattern/layer/sprite selection remains Game/producer-owned and explicit to Atlas.
- [x] Atlas is forbidden from reconstructing legacy source heuristics as authority.
- [x] Public pixel rights for the distinct 15.32 digest remain fail-closed until explicitly attested; prior 15.25 digest attestation is not generalized.
- [x] Permanent serializer, chunk dimensions, CDN and GPU packing remain deferred.
- [x] Full three-file semantic/scope self-review finds no unresolved material finding after Drive metadata incorporation.
- [ ] Exact-head repository governance/architecture/merge checks pass on the final checkpoint head.

## Evidence classification

### PROVEN

- `blakinio/Otheryn@e417c5e7c22986bf4acef0495eb47f7b72c97cce` latest-build script selects project label `15.32`, Drive file `1Dlo3bS4K1nS3mw4BhPZdlHT7lX5zRAvv`, ZIP SHA `1a6bad8b7598cd874f534cd4aae2d249fb3d9b4458b3ccfa75754f91bb27870f`.
- Direct Google Drive metadata for that exact file ID reports name `15.32.zip`, MIME `application/x-zip-compressed`, size `246811594` bytes, created `2026-08-17T18:16:59.210Z`, modified `2026-08-17T18:18:00.781Z`.
- `Oteryn/Oteryn-Atlas` run `32115353176` downloaded and verified that exact SHA before decoding.
- The bundle contains `assets/` only; no `package.json` exists.
- Durable Atlas metadata evidence is `Oteryn/Oteryn-Atlas@0b56d9a95279f1ec02fddd0dfcf8bd6ffd16b539`.
- Verified source counts: 43,514 object appearances, 5,084 sprite sheets, 111,957 total object sprite refs, 79,269 unique object sprite IDs, 567 shift-bearing appearances and 2,192 height-bearing appearances.
- Verified catalog SHA is `35639e000c4c108665a091cfbdf699d549d995b37670bc08de575ab6cd380d85`; appearance SHA is `dc4f4c01e3701c77877c67895168e4399837046122d6d17e3e608a12a2fed075`.
- Verified sprite layouts remain `32x32`, `32x64`, `64x32`, `64x64`.

### DERIVED

- `15.32` is a proven source identity label because both repository selection and Drive filename agree; immutable ZIP/catalog/appearance digests remain the byte-compatibility identity.
- Bounded source presentation uses `units_per_tile=32`, south-east/bottom-right visual anchor, west/north visual coverage and explicit `(-(shift_x + height), -(shift_y + height))` displacement.
- Game must resolve source-specific frame/phase/pattern/layer/sprite selection before Atlas receives the fixture/export.

### UNKNOWN / explicit blocker outside this contract

- public redistribution/publication authority for the exact 15.32 ZIP digest remains to be explicitly attested;
- bounded executable Thais semantic export artifact is the next Game implementation dependency;
- generalized animation and production asset distribution remain deferred.

## Excluded scope

- no raw asset binaries committed;
- no OTBM parser/runtime implementation;
- no Atlas browser implementation;
- no serializer/chunk-size freeze;
- no production CDN/deployment;
- no gameplay occupancy/collision inference from pixels;
- no rewriting of the accepted 15.25 source profile.

## Validation

- exact Google Drive object metadata lookup: **PASS**;
- exact Drive ZIP verification + deterministic decode in Atlas run `32115353176`: **PASS**;
- durable metadata-only Atlas evidence: **PASS**;
- full three-file semantic/scope self-review after incorporating Drive metadata: **PASS**;
- runtime/component/E2E: **NOT_APPLICABLE** — documentation-only Game semantic profile; downstream fixture/export and Atlas renderer tasks own executable evidence.

The primary prior error — using the separate 15.25 fixture as DYN's latest asset authority — is explicitly corrected rather than hidden.

## Context checkpoint

```yaml
checkpoint_version: 1
updated_at: 2026-08-18T10:28:00+02:00
reviewed_content_head: c7d805941d2c9e2953f25fa95fe35a8880323301
branch: architecture/OTV2-20260818-atlas-15-32-appearance-profile-v1
pr: 333
status: validating
context_routes:
  - architecture
  - agent-governance
  - testing
owned_paths:
  - docs/contracts/OTERYN_ATLAS_15_32_APPEARANCE_SPATIAL_PROFILE_V1.md
  - docs/agents/evidence/OTV2-20260818-atlas-15-32-appearance-readiness.md
  - docs/agents/tasks/active/OTV2-20260818-atlas-15-32-appearance-profile-v1.md
proven:
  - exact repo-selected / Drive-named 15.32 source and bytes are pinned by file ID plus immutable digests
  - 15.32 source is materially distinct from the separate 15.25 fixture
derived:
  - bounded source presentation conversion is explicit under Game authority
unknown:
  - exact-digest public pixel rights attestation and bounded Thais export artifact
conflicts: []
first_failure:
  marker: wrong-dyn-appearance-source-selection
  evidence: local uploaded 15.25 fixture had been mistaken for latest source; repo build script, Drive metadata and exact Drive probe prove latest source is distinct 15.32.zip
rejected_hypotheses:
  - rewrite 15.25 contract to pretend it describes 15.32
  - claim internal package version 15.32 without package.json
  - generalize prior 15.25 rights attestation to a different digest
  - let Atlas reconstruct source presentation heuristics
changed_paths:
  - docs/contracts/OTERYN_ATLAS_15_32_APPEARANCE_SPATIAL_PROFILE_V1.md
  - docs/agents/evidence/OTV2-20260818-atlas-15-32-appearance-readiness.md
  - docs/agents/tasks/active/OTV2-20260818-atlas-15-32-appearance-profile-v1.md
validation:
  - command: Google Drive files.get exact object metadata
    result: PASS
    evidence: file name 15.32.zip and byte size/timestamps for exact pinned file ID
  - command: Oteryn-Atlas exact Drive verification run 32115353176
    result: PASS
    evidence: exact ZIP SHA, decoded catalog/appearance digests and counts persisted on Atlas main
  - command: full three-path semantic self-review
    result: PASS
    evidence: no unresolved material authority/scope finding
  - command: runtime/component/E2E
    result: NOT_APPLICABLE
    evidence: documentation-only source profile
blockers: []
next_action: require exact-head Agent Governance, Architecture Semantic Audit, Merge Authority Audit and Merge Gate; merge only if green with no review findings
```

## Source branch closeout

```yaml
source_branch_disposition: auto_delete_after_merge
source_branch_reason: ordinary same-repository contract task
source_branch_evidence: pending
```
