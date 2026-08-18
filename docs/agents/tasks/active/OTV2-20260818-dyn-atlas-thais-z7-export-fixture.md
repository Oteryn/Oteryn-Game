# OTV2-20260818-dyn-atlas-thais-z7-export-fixture

```yaml
task_id: OTV2-20260818-dyn-atlas-thais-z7-export-fixture
title: Produce bounded deterministic Thais Z7 semantic Atlas export fixture
mode: IMPLEMENTATION
status: validating
repository: blakinio/Oteryn-v2
base_branch: main
branch: feat/OTV2-20260818-dyn-atlas-thais-z7-export-fixture
pr: 335
base_sha: 457df3772a7aaf648c1a048b2db2caa409fcf974
reviewed_content_head: 687f5affca92f23fa574623add7beb05b83de8c1
owner: ChatGPT autonomous execution
created_at: 2026-08-18T10:35:00+02:00
updated_at: 2026-08-18T11:02:00+02:00
execution_budget_minutes: 120
owned_paths:
  - tools/game-atlas-thais-fixture/**
  - .github/workflows/game-atlas-thais-fixture.yml
  - docs/agents/evidence/OTV2-20260818-dyn-atlas-thais-z7-export-fixture.md
  - docs/agents/tasks/active/OTV2-20260818-dyn-atlas-thais-z7-export-fixture.md
public_contracts: []
depends_on:
  - docs/contracts/OTERYN_GAME_ATLAS_EXPORT_CONTRACT_V1.md
  - docs/contracts/OTERYN_WORLD_SPATIAL_COORDINATE_PROFILE_V1.md
  - docs/contracts/OTERYN_CRYSTALSERVER_LEGACY_SPATIAL_IMPORT_PROFILE_V1.md
  - docs/contracts/OTERYN_ATLAS_15_32_APPEARANCE_SPATIAL_PROFILE_V1.md
  - Oteryn/Oteryn-Atlas@0b56d9a95279f1ec02fddd0dfcf8bd6ffd16b539
  - blakinio/Otheryn@e417c5e7c22986bf4acef0495eb47f7b72c97cce
blocks:
  - DYN-ATLAS-001 Game-owned bounded semantic export artifact/digest
cross_repository_coordination_id: OTERYN-GAME-ATLAS-V1-THAIS-Z7
external_repositories:
  - Oteryn/Oteryn-Atlas
  - blakinio/Otheryn
```

## Goal

Produce a deterministic bounded Game-owned semantic export artifact for DYN-ATLAS-001 Thais Z7 without making OTBM, the legacy renderer, source asset files or the proof serializer canonical World authority.

## Fixed inputs

```text
migration repo = blakinio/Otheryn@e417c5e7c22986bf4acef0495eb47f7b72c97cce
world.otbm sha256 = 3bd40d14fefec41f24c4b3ae879e420be1a831ef55b95dcbec721e587a09b034
legacy bounds = X 32280..32440, Y 32155..32305, Z 7
native bounds = x [32280,32441), y [32155,32306), floor -7
asset = 15.32.zip
Drive file id = 1Dlo3bS4K1nS3mw4BhPZdlHT7lX5zRAvv
asset ZIP sha256 = 1a6bad8b7598cd874f534cd4aae2d249fb3d9b4458b3ccfa75754f91bb27870f
catalog sha256 = 35639e000c4c108665a091cfbdf699d549d995b37670bc08de575ab6cd380d85
appearance sha256 = dc4f4c01e3701c77877c67895168e4399837046122d6d17e3e608a12a2fed075
```

## Acceptance criteria

- [x] Game-owned exporter/verifier/self-tests exist under `tools/game-atlas-thais-fixture/`.
- [x] OTBM exists only at producer/import boundary; emitted records are explicit semantic projection.
- [x] Z7 -> native `floor=-7` and same-position order are explicit.
- [x] nested container children are not emitted as visible spatial records.
- [x] concrete frame/phase/pattern/layer/sprite selection is producer-owned.
- [x] visual coverage/displacement follows the accepted 15.32 profile and is not gameplay occupancy.
- [x] legacy appearance/sprite IDs remain provenance IDs; canonical entity identity is not invented.
- [x] proof uses non-production `dyn-atlas-thais-z7-jsonl-v0` packaging.
- [x] canonical output excludes operational timestamps/runner paths.
- [x] two independent builds are byte-identical and deliberate corruption is rejected.
- [x] verifier recomputes digests/order/bounds/identity/primitive constraints.
- [x] default-deny public-field policy validates exact manifest/tile/primitive key shapes; unexpected fields fail closed.
- [x] representative negative self-test proves an unallowlisted `text` field is rejected.
- [x] workflow Actions are pinned to exact SHAs.
- [x] no raw source pixels are committed or emitted by the semantic fixture.
- [ ] final frozen head passes dedicated exact-head fixture workflow and all repository merge gates.

## Proven integration evidence before final freeze

Workflow run `32117767429`, job `95651058334`, producer head `ee242eb10b0d91cc1d6a0a812bc6f9d8af3d4711`: **SUCCESS**.

```text
tiles = 24311
ground_items = 24292
top_level_tile_items = 14990
source_item_tree_without_ground = 14993
presentation_records = 39282
resolved_primitives = 39282
unique appearance ids = 862
unique sprite ids = 990
tiles.jsonl bytes = 28040344
tiles.jsonl sha256 = ff14efee3fc376d8f18432c628294c64ffe89450a59aaa498a28e6d705815984
diagnostics bytes = 19
diagnostics sha256 = 60326e4e048106d4366a2fd8fe472ccfdf06667fcd0f234977febfeaa38f31b8
```

Three nested descendants are proven by `14993 - 14990 = 3` and are excluded from visible presentation records. The first run produced a byte-identical double build and rejected deliberate corruption.

The first head exposed one governance finding: unpinned action tags. This was repaired with exact action SHAs. A later self-review added the default-deny public shape policy so future accidental source/private fields cannot be silently serialized.

## Final-head evidence rule

Because `producer_repository_sha` is part of the canonical manifest, recording a terminal artifact digest in a new pre-merge commit would change that digest again. Final frozen-head run/artifact/digest identities must therefore be written to PR #335 metadata after the head is fixed and then to the post-merge archive record.

## Evidence classification

### PROVEN

- source/profile authority, exact input digests and selected bounds;
- deterministic semantic exporter and fail-closed verifier;
- 24,311 tiles / 39,282 visible presentation records / zero diagnostics in the successful proof run;
- nested source children excluded from spatial presentation;
- semantic output contains no source pixel bytes;
- repository-policy action pinning;
- default-deny canonical public field shape.

### DERIVED

- proof-only JSONL v0 is a bounded inspectable packaging choice, not permanent serializer authority;
- export record IDs are deterministic proof-scope identities while unresolved canonical entity identity remains explicit.

### UNKNOWN / outside this task

- exact-digest public pixel publication authorization for 15.32.zip;
- permanent serializer/chunk profile and production resource ceilings;
- generalized animation/live appearance semantics.

## Context checkpoint

```yaml
checkpoint_version: 3
updated_at: 2026-08-18T11:02:00+02:00
reviewed_content_head: 687f5affca92f23fa574623add7beb05b83de8c1
branch: feat/OTV2-20260818-dyn-atlas-thais-z7-export-fixture
pr: 335
status: validating
context_routes: [architecture, agent-governance, testing, implementation]
proven:
  - deterministic bounded export works on exact pinned map/assets
  - default-deny public projection guard is now explicit
  - action pinning satisfies repository policy
derived:
  - terminal producer-SHA-dependent identity belongs in frozen-head PR metadata and post-merge closeout
unknown:
  - final frozen-head workflow/artifact identity until current head run completes
conflicts: []
first_failure:
  marker: unpinned-workflow-actions
  evidence: semantic run passed; repository policy rejected action tags; repaired with exact SHAs
rejected_hypotheses:
  - let Atlas parse OTBM directly
  - emit nested container items as visible stack records
  - invent canonical content identity from legacy numeric IDs
  - treat proof JSONL packaging as permanent serializer
  - permit unallowlisted canonical fields
validation:
  - command: Game Atlas Thais Z7 fixture run 32117767429
    result: PASS
  - command: pure producer self-tests before public-policy guard
    result: PASS_7_OF_7
  - command: final current-head workflow and repository gates
    result: PENDING
blockers: []
next_action: freeze this head, wait for final exact-head fixture + repository gates, update PR metadata only with terminal identities, Ready/squash-merge if green
```

## Source branch closeout

```yaml
source_branch_disposition: auto_delete_after_merge
source_branch_reason: ordinary same-repository implementation task
source_branch_evidence: pending
```
