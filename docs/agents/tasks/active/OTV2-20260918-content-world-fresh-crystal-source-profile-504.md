# OTV2-20260918-content-world-fresh-crystal-source-profile-504

```yaml
task_id: CONTENT_WORLD_FRESH_CRYSTAL_SOURCE_GENERATION_ADMISSION_504
title: Admit exact fresh CrystalServer source generation
mode: IMPLEMENT
status: validating
repository: Oteryn/Oteryn-Game
base_branch: main
branch: agent/content-world-fresh-crystal-source-profile-504
issue: 162
pr: null
base_sha: 988b17f609a9a156a5f2d7e7eef1daf0b23c47e8
head_sha: null
final_head_sha: null
final_head_frozen_at: null
owner: "Oteryn: content world build"
created_at: 2026-09-18
updated_at: 2026-09-18
execution_policy: continuous_progress
owned_paths:
  - tools/game-atlas-fullworld-source/producer.py
  - tools/game-atlas-fullworld-source/self_test.py
  - tools/game-atlas-fullworld-source/README.md
  - docs/contracts/OTERYN_CRYSTALSERVER_FRESH_SOURCE_GENERATION_PROFILE_V2.md
  - docs/agents/tasks/active/OTV2-20260918-content-world-fresh-crystal-source-profile-504.md
public_contracts:
  - docs/contracts/OTERYN_CRYSTALSERVER_FRESH_SOURCE_GENERATION_PROFILE_V2.md
depends_on:
  - "#162 comment 5728471141"
blocks:
  - CONTENT_WORLD_D3_REAL_BATCH_BUNDLE_MEASUREMENT_504
cross_repository_coordination_id: null
external_repositories:
  - zimbadev/crystalserver@ff7ede593c69d4c658b382c97443e8155926924a
  - blakinio/Otheryn@e417c5e7c22986bf4acef0495eb47f7b72c97cce
```

## Outcome

Admit one explicit exact-digest fresh CrystalServer map generation through the existing Game-owned fullworld producer so D3 can consume real source data without bypassing provenance. Preserve the historical default source profile unchanged.

## Architecture and source of truth

- `PROVEN`: allocation authority is #162 comment `5728471141`.
- `PROVEN`: protected admission baseline was `main@988b17f609a9a156a5f2d7e7eef1daf0b23c47e8`; all 22 open PR changed-path sets had zero overlap with this task's custody before first tracked write.
- `PROVEN`: fresh source is `zimbadev/crystalserver@ff7ede593c69d4c658b382c97443e8155926924a`, `world.otbm` SHA-256 `09cce62af6c86644b5579fba460c674585261eb987ca5aa1f52baef9e91f8bbb`, Git blob `e95e8f7c7a95d1b634b49a5dea5a5dc76021406b`, size `52267895` bytes.
- `PROVEN`: parser remains the clean exact `blakinio/Otheryn@e417c5e7c22986bf4acef0495eb47f7b72c97cce` generation.
- `PROVEN`: the existing 15.32 ZIP/catalog/appearance hashes match the admitted profile.
- `PROVEN`: `OTERYN_CRYSTALSERVER_LEGACY_SPATIAL_IMPORT_PROFILE_V1.md` remains unchanged.
- `UNKNOWN`: Global target geometry/collision/order/footprint truth remains unproven and is not promoted by this task.

## High-risk authority/recovery qualification

`NOT_APPLICABLE`: this is an offline, exact-source import admission. It performs no production mutation, session/lease/generation authority change, durable write, deployment or live-environment action.

## Acceptance criteria

- [x] Historical `load_runtime(...)` default still delegates to the qualified legacy validator.
- [x] Historical default accepts the historical exact map and rejects the fresh map.
- [x] Fresh profile is explicit, exact-versioned and rejects unknown/floating profile IDs.
- [x] Fresh map byte length, SHA-256 and Git blob are checked fail-closed.
- [x] Asset ZIP/catalog/appearance identities are checked fail-closed.
- [x] Pinned parser repository revision, clean worktree, tracked blobs and actual loaded module roots are checked fail-closed.
- [x] Exact fresh map exhausts through the pinned parser and patched producer with `strict=True`.
- [x] Existing real tile projection initializes without bypassing validation.
- [x] Existing source-identity tests remain green; no target-sensitive fact is promoted.

## Excluded scope

No CW2/CW3/D3 mutation, parser rewrite, corridor recapture, target parity claim, permanent bundle format, compression/chunk decision, production resource maxima, Cargo/app/workflow/registry mutation, Atlas/Platform/META write, or production/deployment authority.

## Implementation / findings

The minimum sufficient change adds an explicit source-generation profile to the existing fullworld producer. Omitting the new selector preserves the previous qualified validation path. The new selector validates exact source, assets and parser identity before reusing the same parser/projection implementation.

A Windows checkout initially exposed that raw working-tree bytes are not a portable proxy for Git blob identity because line-ending checkout conversion can differ while the repository is clean. Parser provenance therefore uses exact Git `HEAD`, clean status and `git hash-object` for required tracked paths; source map integrity remains SHA-256 first with Git blob only as corroborating provenance.

## Validation

### Focused

- `python -m py_compile tools/game-atlas-fullworld-source/producer.py tools/game-atlas-fullworld-source/self_test.py` — PASS.
- `python tools/game-atlas-fullworld-source/self_test.py` — PASS.
- real historical default runtime with historical map/assets — PASS; profile fields remain `None`.
- real historical default runtime with fresh map — expected FAIL: `canonical world.otbm SHA-256 mismatch`.
- explicit fresh profile runtime + first real tile projection — PASS; profile revision 2, canonical tile 774 bytes.
- exact fresh map full strict stream through patched producer — PASS: 1 MapHeader, 18,997,668 Tile, 33 Town, 18 Waypoint; 143.699 seconds.
- full visible-presentation/appearance census — PASS: 24,502,036 visible ground/top-level presentations; exactly one unresolved occurrence, server ID `2141`, matching the producer's already-supported explicit unresolved case.

### Component/integration

- `python tools/reference-world-corridor-census/content_source_batch_self_test.py` — PASS.
- `python tools/agents/validate_governance.py` — PASS after adding the required `issue: 162` task binding.
- `git diff --check` and staged `git diff --cached --check` — PASS before commit.

### E2E

`NOT_APPLICABLE`: this prerequisite admits an offline source generation and does not mutate runtime gameplay or production state. The real full-source strict parse and projection qualification are the applicable integration evidence.

### Exact-head CI

Pending PR creation. Repository-required exact-head checks and Merge Queue qualification remain authoritative and cannot be replaced by these local results.

## Self-review

- method/reviewer: implementing `Oteryn: content world build` session, whole-diff review against #162 allocation `5728471141`.
- material findings: raw working-tree hashing was rejected as parser Git-identity evidence on Windows and replaced before freeze with exact Git HEAD + clean worktree + `git hash-object` checks.
- verdict: no remaining material scope, provenance, semantic-promotion or backward-compatibility finding identified before candidate freeze.

## Independent review

- required: `YES` because the candidate adds a durable source-admission contract and provenance gate.
- exact head/method/result: pending hosted exact-head Architecture Semantic Audit / repository checks after PR creation.

## Context checkpoint

```yaml
last_progress: local source/parser/asset and full strict-stream qualification passed
status: validating
branch: agent/content-world-fresh-crystal-source-profile-504
pr: null
blocker: null
next_action: freeze, commit and publish the exact candidate through normal Git push, then open one PR against main
```
