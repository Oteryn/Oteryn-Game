# OTV2-20260922-content-world-item-schema-readiness-504

```yaml
task_id: OTV2-20260922-content-world-item-schema-readiness-504
title: D6-M1 Item Schema Readiness
mode: IMPLEMENT
status: implementing
repository: Oteryn/Oteryn-Game
base_branch: main
branch: agent/content-world-item-schema-readiness-504
pr: null
base_sha: 9daf3522efbf799c5d9ffe9817215895d4fa8af0
head_sha: null
final_head_sha: null
final_head_frozen_at: null
owner: Oteryn: item schema lead
created_at: 2026-09-22T10:44:00+02:00
updated_at: 2026-09-22T10:44:00+02:00
execution_policy: continuous_progress
owned_paths:
  - apps/game-server/src/content/reference_playable.rs
  - apps/game-server/src/content/project.rs
  - apps/game-server/src/content/reference_artifact.rs
  - apps/game-server/src/content/cw2_b1_import.rs
  - apps/game-server/tests/content_reference_playable.rs
  - apps/game-server/tests/content_reference_artifact.rs
  - apps/game-server/tests/content_world_project.rs
  - apps/game-server/tests/content_world_project_fs.rs
  - apps/game-server/tests/content_world_cw2_b1_import.rs
  - docs/agents/evidence/OTV2-20260922-content-world-item-schema-readiness.json
  - docs/agents/tasks/active/OTV2-20260922-content-world-item-schema-readiness-504.md
public_contracts: []
depends_on:
  - protected PR #737 / full 38,157 Item identity closure
blocks:
  - full 38,157 Item classification / field verification / semantic promotion
cross_repository_coordination_id: null
external_repositories: []
```

## Outcome

Extend the existing canonical Item path so every admitted immutable Item field family can be represented, lowered and deterministically projected to server-authoritative or client-safe artifact data without creating a second Item system or silently losing preserved B1 candidate observations.

## Architecture and source of truth

- PROVEN — #737 is protected as main `9daf3522efbf799c5d9ffe9817215895d4fa8af0`; its 38,157 source-to-native Item identity map is immutable input to this task.
- PROVEN — GAME-ITEM owns immutable ItemType semantics; mutable quantity/charges/durability/active imbues/timers/location/custody/container contents remain ItemInstance/DUR-owned.
- PROVEN — owner direction #504 comments `5771837645`, `5771909726`, `5772730929`, `5773340420` defines required capability families and fail-closed evidence rules.
- PROVEN — B1 candidate observations are migration/provenance evidence, not Reference truth.
- DERIVED — this generation closes schema/round-trip readiness only; value verification/promotion remains a later generation.

Required typed destinations include Presentation, Classification, Physical, Stack, Equipment, Weapon, Protection, SkillModifiers, Charges, Temporal, Container, Imbuement, UseTransform and admitted TradeRestrictions. B1 Fluid, Read/Write and presentation-binding candidates must receive either a bounded typed capability or an explicit unsupported/loss disposition.

## High-risk authority/recovery qualification

```yaml
applicable: false
reason: immutable Content schema/compiler/artifact work only; no production mutation, runtime authority, PREPARE/COMMIT, session replacement or persisted recovery interpretation
```

## Acceptance criteria

- [ ] Preserve all 38,157 #737 native Item identities with zero remap.
- [ ] Unknown / not-applicable remain distinct from numeric zero / false.
- [ ] Every current B1 `GAME_ITEM_CANDIDATE` family has a typed destination or explicit unsupported/loss record.
- [ ] Raw source numeric Item IDs never survive as runtime semantic identity for transform/decay/read-write targets.
- [ ] Project -> Reference -> server artifact -> client-safe artifact round-trip is deterministic.
- [ ] Server-authoritative vs client-safe field allowlisting is explicit.
- [ ] Representative weapon, equipment, container, charges/use, rune, material/loot and presentation records round-trip without silent field loss.
- [ ] Existing v1/v2/v3 Item artifact compatibility and #737 family bounds remain intact.
- [ ] Imbuement slot/family/tier remains typed gameplay power, not presentation metadata.
- [ ] Unsupported/provenance-only/unknown/conflict/excluded source fields are not silently promoted.
- [ ] Exact-head repository-selected qualification passes before freeze.

## Excluded scope

No second Crystal/B1 source import. No full-family semantic value promotion. No TibiaWiki/current-value promotion. No client UI/tooltip consumer wiring, World Bundle/Foundation/Cargo change, runtime/persistence/protocol/value authority, or Ability/Creature/NPC/Quest/spatial schema widening. `world_runtime.rs` is excluded unless separately reallocated after fresh evidence.

## Implementation / findings

Start from the already-retained B1 source-field -> typed capability matrix in #504 comment `5773340420`. Prefer bounded typed structures over arbitrary maps. Resolve cross-Item references through the protected #737 native map.

## Validation

### Focused

- command/run: pending
- result: pending

### Component/integration

- command/run: pending
- result: pending

### E2E

- scenario: Project -> Reference -> server/client artifact representative Item round-trip
- result: pending

### Exact-head CI

- final head: pending
- trigger source: pull_request
- workflow/run/job: pending
- runner assignment: repository-selected
- classification: pending
- result: pending

## Self-review

- exact head: pending
- method/reviewer: implementing/coordinating agent
- material findings: pending
- verdict: pending

## Independent review

- required: pending
- exact head: pending
- method/auditor: pending
- material findings: pending
- verdict: pending

## PR and closeout

- changed-file review: pending
- unresolved review threads: pending
- related/superseded PRs: #737 prerequisite protected
- protected auto-merge: forbidden substitute; governed Merge Queue only
- merge commit/result: pending
- ownership release: pending

## Context checkpoint

```yaml
last_progress: D6-M1 single writer released after protected #737 and zero-overlap census
status: implementing
branch: agent/content-world-item-schema-readiness-504
head_sha: null
pr: null
final_head_sha: null
final_head_frozen_at: null
ci_trigger_source: pull_request
ci_check_generation: null
ci_checks_for_current_head: 0
ci_run_ids: []
ci_job_ids: []
runner_assignment_state: repository-selected
terminal_ci_wait_started_at: null
terminal_ci_checks_for_current_generation: 0
unchanged_state_checks: 0
identical_failure_retries: 0
repair_cycles_for_current_gate: 0
ci_recovery_actions_for_current_head: 0
stall_warnings: 0
owner_action_required: null
blocker: null
next_action: implement the minimal typed schema and representative deterministic round-trip
```
