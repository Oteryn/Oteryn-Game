# OTV2-20260818-tibia-15-25-appearance-spatial-profile-v1

```yaml
task_id: OTV2-20260818-tibia-15-25-appearance-spatial-profile-v1
title: Define Tibia 15.25 appearance spatial profile v1
mode: CONTRACT
status: validating
repository: blakinio/Oteryn-v2
base_branch: main
branch: architecture/OTV2-20260818-tibia-15-25-appearance-spatial-profile-v1
pr: 331
base_sha: ade3005cdf9ad6daeb87dc20a6546d5c29ee61da
reviewed_content_head: 320eaaa283864558921c388b6237f1fd7aafbc3c
owner: ChatGPT autonomous execution
created_at: 2026-08-18T09:51:00+02:00
updated_at: 2026-08-18T10:01:00+02:00
execution_budget_minutes: 60
owned_paths:
  - docs/contracts/OTERYN_TIBIA_15_25_APPEARANCE_SPATIAL_PROFILE_V1.md
  - docs/agents/evidence/OTV2-20260818-tibia-15-25-appearance-spatial-readiness.md
  - docs/agents/tasks/active/OTV2-20260818-tibia-15-25-appearance-spatial-profile-v1.md
public_contracts:
  - oteryn-tibia-15-25-appearance-spatial-v1
depends_on:
  - docs/contracts/OTERYN_WORLD_SPATIAL_COORDINATE_PROFILE_V1.md
  - docs/contracts/OTERYN_CRYSTALSERVER_LEGACY_SPATIAL_IMPORT_PROFILE_V1.md
  - docs/contracts/OTERYN_GAME_ATLAS_EXPORT_CONTRACT_V1.md
  - blakinio/Otheryn@e417c5e7c22986bf4acef0495eb47f7b72c97cce
blocks:
  - DYN-ATLAS-001 appearance visual-anchor/coverage/displacement gate
cross_repository_coordination_id: OTERYN-GAME-ATLAS-V1-APPEARANCE
external_repositories:
  - Oteryn/Oteryn-Atlas
  - blakinio/Otheryn
```

## Goal

Define the smallest Game-owned appearance spatial profile required for `DYN-ATLAS-001` using the exact owner-authorized Tibia client package, without promoting Tibia asset files or the legacy renderer into canonical World authority.

## Acceptance criteria

- [x] Exact asset archive digest and package metadata verified directly.
- [x] Package version identified from `package.json` as `15.25.bd5a04`, not 15.32.
- [x] Catalog/appearance digests and deterministic object/sprite counts recorded.
- [x] Pinned Otheryn decoder/renderer behavior used only as source-conversion evidence.
- [x] `units_per_tile`, visual anchor/coverage and effective displacement mapping explicit.
- [x] Pixel dimensions are explicitly forbidden from becoming gameplay occupancy/collision semantics.
- [x] Concrete frame/phase/layer/sprite resolution remains producer-owned and explicit to Atlas.
- [x] Asset IDs remain provenance/source IDs rather than canonical Game entity identity.
- [x] Owner rights attestation is scoped to the exact archive digest and labelled as owner-supplied evidence.
- [x] Permanent GPU packing, serializer/chunk format and generalized animation contract remain deferred.
- [x] Full owned-path semantic self-review completed; one terminology/authority finding was repaired.
- [ ] Exact-head repository governance/architecture/merge checks pass.

## Evidence classification

### PROVEN

- Archive SHA-256: `01c45146e2fcec3f4087844e0cbc1817fb1d60b310a35ac5d88c07aab6f73d1a`.
- Root package metadata: `15.25.bd5a04`.
- Direct decoding: 42,107 object appearances, 4,927 sprite sheets, 75,623 unique referenced object sprite IDs.
- These counts reconcile with pinned legacy Otheryn 15.25 evidence.
- Pinned renderer uses a 32-pixel tile scale and subtracts multi-tile visual excess, source shift and source height from the projected draw origin.

### DERIVED

- `units_per_tile=32` is the bounded native presentation-unit mapping.
- Owning tile is the south-east/bottom-right visual anchor cell; decoded visual coverage extends west/north.
- Effective native 2D displacement is `(-(shift_x + height), -(shift_y + height))`.
- Atlas receives producer-resolved concrete primitives instead of reproducing legacy source heuristics.

### UNKNOWN / deliberately deferred

- generalized animation semantics beyond the bounded static proof;
- permanent Game -> Atlas physical serializer/chunk profile;
- production CDN/GPU packing.

## Self-review repair

The first contract draft called pixel-derived sprite coverage a `footprint`. Full semantic self-review found this could be misread as gameplay occupancy/collision authority. That was a material terminology/authority risk.

Reviewed content head `320eaaa283864558921c388b6237f1fd7aafbc3c` repairs it by defining **presentation visual coverage** only and explicitly forbidding inference of collision, movement blocking, item ownership or interaction area from decoded sprite dimensions.

No unresolved material semantic/scope finding remains after that repair.

## Rights boundary

Owner attestation authorizes decoding, derivation, rendering, deduplication and publication of the exact archive digest for public Oteryn Atlas use. This task does not claim independent third-party copyright verification and does not extend authorization to another digest.

## Validation

- direct archive/package/catalog/appearance verification: **PASS**;
- exact three-path semantic/full-diff self-review: **PASS after repair**, zero unresolved findings;
- runtime/component/E2E: **NOT_APPLICABLE** — documentation-only Game semantic contract; executable fixture/Atlas tasks own runtime evidence.

## Context checkpoint

```yaml
checkpoint_version: 1
updated_at: 2026-08-18T10:01:00+02:00
head: 320eaaa283864558921c388b6237f1fd7aafbc3c
branch: architecture/OTV2-20260818-tibia-15-25-appearance-spatial-profile-v1
pr: 331
status: validating
context_routes:
  - architecture
  - agent-governance
  - testing
owned_paths:
  - docs/contracts/OTERYN_TIBIA_15_25_APPEARANCE_SPATIAL_PROFILE_V1.md
  - docs/agents/evidence/OTV2-20260818-tibia-15-25-appearance-spatial-readiness.md
  - docs/agents/tasks/active/OTV2-20260818-tibia-15-25-appearance-spatial-profile-v1.md
proven:
  - exact asset archive is package 15.25.bd5a04 with reconciled legacy Atlas counts
  - source renderer evidence establishes bounded 32-pixel presentation projection
derived:
  - source profile maps visual anchor/coverage/displacement into explicit native presentation semantics
unknown:
  - bounded Game Thais semantic export artifact remains the next implementation dependency
conflicts: []
first_failure:
  marker: pixel-footprint-authority-ambiguity
  evidence: first draft could conflate decoded visual extent with gameplay occupancy; repaired before exact-head readiness
rejected_hypotheses:
  - treat archive as 15.32
  - infer gameplay occupancy/collision from sprite pixels
  - let Atlas reimplement legacy asset heuristics as authority
  - generalize owner attestation to unrelated archives
changed_paths:
  - docs/contracts/OTERYN_TIBIA_15_25_APPEARANCE_SPATIAL_PROFILE_V1.md
  - docs/agents/evidence/OTV2-20260818-tibia-15-25-appearance-spatial-readiness.md
  - docs/agents/tasks/active/OTV2-20260818-tibia-15-25-appearance-spatial-profile-v1.md
validation:
  - command: direct asset archive/package/catalog/appearance verification
    result: PASS
    evidence: exact digests, version and deterministic appearance/sprite counts recorded
  - command: full PR #331 semantic diff review
    result: PASS
    evidence: visual-coverage terminology repaired; zero unresolved material findings
  - command: runtime/component/E2E
    result: NOT_APPLICABLE
    evidence: documentation-only Game semantic contract
blockers: []
next_action: require exact-head Agent Governance, Architecture Semantic Audit, Merge Authority Audit and Merge Gate; if green with no review findings, mark Ready and squash-merge
```

## Source branch closeout

```yaml
source_branch_disposition: auto_delete_after_merge
source_branch_reason: ordinary same-repository contract task
source_branch_evidence: pending
```
