# OTV2-20260822-game-platform-catalog-producer-v1

```yaml
task_id: OTV2-20260822-game-platform-catalog-producer-v1
title: Add native Game to Platform Game Catalog producer v1
mode: IMPLEMENT
status: completed
repository: Oteryn/Oteryn-Game
issue: 52
base_branch: main
branch: null
pr: 57
base_sha: a2a5da955dd8f580c9e768c8ac6a741db388cb22
head_sha: 34f80e061f9e2b74c4006e2049d1510873466b11
final_head_sha: 34f80e061f9e2b74c4006e2049d1510873466b11
merge_sha: 96ea673839f1d93190a40c17ae8036ac82096ded
owner: chat-github-20260822-game-platform-catalog
created_at: 2026-08-22T19:52:46+02:00
updated_at: 2026-08-22T22:29:50+02:00
execution_budget_minutes: 60
cross_repository_coordination_id: OTERYN-GAME-PLATFORM-CATALOG-V1
owned_paths:
  - docs/contracts/OTERYN_GAME_PLATFORM_CATALOG_EXPORT_V1.md
  - docs/contracts/RESOURCE_LIMITS_REGISTRY.json
  - docs/contracts/CROSS_REPOSITORY_CONTRACT_LOCK.json
  - tools/game-platform-catalog/**
  - docs/agents/tasks/active/OTV2-20260822-game-platform-catalog-producer-v1.md
public_contracts:
  - docs/contracts/OTERYN_GAME_PLATFORM_CATALOG_EXPORT_V1.md
```
## Outcome

Deliver the first Game-owned deterministic native snapshot producer boundary for Platform Game Catalog without claiming unavailable broad native content.

## Proven boundaries

- `PROVEN`: `Oteryn/Oteryn-Game` is the current Game product authority.
- `PROVEN`: Platform's accepted native Game Catalog semantic contract requires authority/revision/profile manifests and immutable snapshot data.
- `PROVEN`: active Wave 1 paths are disjoint from this task's allocated paths.
- `PROVEN`: Atlas exports are not a Platform Game Catalog authority substitute.
- `UNKNOWN`: broad native NPC/spell/quest/achievement inventories are not yet implemented on current Game main and receive no completeness credit.

## Acceptance criteria

- [x] TDD RED observed before producer implementation.
- [x] Stable native ContentKey validation rejects legacy numeric identity substitutes.
- [x] Canonical output is independent of input collection ordering.
- [x] Exact authority epoch, source revision, ruleset and content profile are mandatory.
- [x] Capability support and completeness are explicit and fail closed.
- [x] Duplicate entities/relations, dangling relations and contradictory tombstones are rejected.
- [x] Tombstones require complete authoritative capability coverage.
- [x] Payload digest is independently verifiable, deterministic and provenance-protecting.
- [x] Bounded counts/string/depth limits are enforced before expensive work.
- [x] Applicable resource limits registered in `RESOURCE_LIMITS_REGISTRY.json`.
- [x] Cross-repository lock terminalized with the merged canonical commit and contract digest.
- [x] Focused tests, Ruff, py_compile and governance validation pass before PR creation.
- [x] Exact-head PR validation passes.
- [x] Whole-diff self-review has zero unresolved material findings on final head.
- [x] No production/deployment/live-service mutation performed.

## Excluded scope

No Platform consumer mutation, no production publication/activation, no broad native content population, no Atlas/Canary/CrystalServer fallback, no Wave 1 path mutation, and no final native World Bundle format decision.

## Validation

### Focused
- TDD RED: missing `producer`, missing I/O functions, unknown-capability acceptance and mutable `generated_at` integrity were each observed failing before implementation/fix.
- `python -m unittest discover -s tools/game-platform-catalog -p "test_*.py" -q` -> 19 tests PASS.
- `ruff check tools/game-platform-catalog` -> PASS after repairing 9 findings.
- explicit `python -m py_compile ...` -> PASS.
- CLI produce+verify on `fixtures/unsupported-native-input.json` -> PASS.

### Governance
- `python tools/agents/validate_governance.py` -> PASS after resource-limit and pending-lock registration.
- `git diff --check` -> PASS after pending-lock registration.

### E2E
- `NOT_APPLICABLE`: producer framework exposes no user-facing path and performs no Platform import/activation.

## Terminal closeout

- PR #57 exact final head `34f80e061f9e2b74c4006e2049d1510873466b11`.
- deterministic independent Architecture semantic audit run `32596697044` -> PASS after Ready transition.
- Merge gate run `32596254556`, aggregate `Merge gate / validate` -> PASS.
- Agent governance run `32596254559` -> PASS.
- review threads -> 0.
- squash merge -> `96ea673839f1d93190a40c17ae8036ac82096ded`.
- canonical contract Git-blob SHA-256 -> `9bc87fba5b565e5c7d4d3f6ca7a9bd75d45d8110de64a2a50f8f74d9ba181cad`.
- cross-repository lock terminalized as `LOCKED`; Platform consumer remains pending.
- production remains disabled.

