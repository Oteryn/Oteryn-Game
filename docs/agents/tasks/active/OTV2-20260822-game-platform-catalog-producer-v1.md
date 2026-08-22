# OTV2-20260822-game-platform-catalog-producer-v1

```yaml
task_id: OTV2-20260822-game-platform-catalog-producer-v1
title: Add native Game to Platform Game Catalog producer v1
mode: IMPLEMENT
status: validating
repository: Oteryn/Oteryn-Game
issue: 52
base_branch: main
branch: agent/game-platform-catalog-producer-v1
pr: null
base_sha: a2a5da955dd8f580c9e768c8ac6a741db388cb22
head_sha: a2a5da955dd8f580c9e768c8ac6a741db388cb22
owner: chat-github-20260822-game-platform-catalog
created_at: 2026-08-22T19:52:46+02:00
updated_at: 2026-08-22T20:40:00+02:00
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
- [x] Bounded counts/string/depth limits are enforced before expensive work.- [x] Applicable resource limits registered in `RESOURCE_LIMITS_REGISTRY.json`.
- [ ] Cross-repository lock has a truthful pending PR entry.
- [x] Focused tests, Ruff, py_compile and governance validation pass before PR creation.
- [ ] Exact-head PR validation passes.
- [ ] Whole-diff self-review has zero material findings on final head.
- [x] No production/deployment/live-service mutation performed.

## Excluded scope

No Platform consumer mutation, no production publication/activation, no broad native content population, no Atlas/Canary/CrystalServer fallback, no Wave 1 path mutation, and no final native World Bundle format decision.

## Validation

### Focused
- TDD RED: missing `producer`, missing I/O functions, unknown-capability acceptance and mutable `generated_at` integrity were each observed failing before implementation/fix.
- `python -m unittest discover -s tools/game-platform-catalog -p "test_*.py" -q` -> 18 tests PASS.
- `ruff check tools/game-platform-catalog` -> PASS after repairing 9 findings.
- explicit `python -m py_compile ...` -> PASS.
- CLI produce+verify on `fixtures/unsupported-native-input.json` -> PASS.

### Governance
- `python tools/agents/validate_governance.py` -> PASS before shared-registry expansion; rerun required after lock entry.
- `git diff --check` -> PASS before final lock entry.

### E2E
- `NOT_APPLICABLE` for this producer framework until a real native capability adapter and Platform consumer exist.
## Context checkpoint

```yaml
last_progress: producer contract/tool/tests are GREEN and the full fixed v1 resource-limit set is registered
status: validating
head_sha: a2a5da955dd8f580c9e768c8ac6a741db388cb22
pr: null
blocker: draft PR number is required before the pending cross-repository lock entry can be valid
owner_action_required: null
next_action: commit and push the validated pre-PR package, open the draft PR, then bind its number in the pending cross-repository lock entry
```
