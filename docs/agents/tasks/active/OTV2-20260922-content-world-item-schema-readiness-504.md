# OTV2-20260922-content-world-item-schema-readiness-504

```yaml
task_id: OTV2-20260922-content-world-item-schema-readiness-504
title: D6-M1 Item Schema Readiness
mode: IMPLEMENT
status: implementing
repository: Oteryn/Oteryn-Game
base_branch: main
branch: agent/content-world-item-schema-readiness-504
pr: 749
base_sha: cf5c5f35476559450b6bbaf87dce519f7eead9d0
head_sha: 3d69fa467fc41d6ef7b85fa4c1cdca509a060a4b
final_head_sha: null
final_head_frozen_at: null
owner: Oteryn: item schema lead
created_at: 2026-09-22T10:44:00+02:00
updated_at: 2026-09-22T11:52:07Z
execution_policy: continuous_progress
owned_paths:
  - apps/game-server/src/content/reference_playable.rs
  - apps/game-server/src/content/project.rs
  - apps/game-server/src/content/project_fs.rs
  - apps/game-server/src/content/reference_artifact.rs
  - apps/game-server/src/content/cw2_b1_import.rs
  - apps/game-server/tests/content_reference_playable.rs
  - apps/game-server/tests/content_reference_artifact.rs
  - apps/game-server/tests/content_world_project.rs
  - apps/game-server/tests/content_world_project_fs.rs
  - apps/game-server/tests/content_world_project_publication.rs
  - apps/game-server/tests/content_world_cw2_b1_import.rs
  - docs/agents/evidence/OTV2-20260922-content-world-item-schema-readiness.json
  - docs/agents/tasks/active/OTV2-20260922-content-world-item-schema-readiness-504.md
  - docs/architecture/OTERYN_REFERENCE_ITEM_ARTIFACT_RESOURCE_PROFILE_V1.md
  - docs/contracts/RESOURCE_LIMITS_REGISTRY.json
  - tools/reference-item-resource-profile/item_resource_profile.py
public_contracts:
  - D6_M1_REFERENCE_ITEM_ARTIFACT_RESOURCE_PROFILE/v1
  - docs/contracts/RESOURCE_LIMITS_REGISTRY.json
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

- PROVEN — the protected #737 38,157 source-to-native Item identity map is inherited unchanged through protected main `cf5c5f35476559450b6bbaf87dce519f7eead9d0` and remains immutable input to this task.
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

- [x] Preserve all 38,157 #737 native Item identities with zero remap.
- [x] Unknown / not-applicable remain distinct from numeric zero / false.
- [x] Every current B1 `GAME_ITEM_CANDIDATE` family has a typed destination or explicit unsupported/loss record.
- [x] Raw source numeric Item IDs never survive as runtime semantic identity for transform/decay/read-write targets.
- [x] Project -> Reference -> server artifact -> client-safe artifact round-trip is deterministic.
- [x] Server-authoritative vs client-safe field allowlisting is explicit.
- [x] Representative weapon, equipment, container, charges/use, rune, material/loot and presentation records round-trip without silent field loss.
- [x] Existing v1/v2/v3 Item artifact compatibility and #737 family bounds remain intact.
- [x] Imbuement slot/family/tier remains typed gameplay power, not presentation metadata.
- [x] Unsupported/provenance-only/unknown/conflict/excluded source fields are not silently promoted.
- [ ] Exact-head repository-selected qualification passes before freeze.

## Excluded scope

No second Crystal/B1 source import. No full-family semantic value promotion. No TibiaWiki/current-value promotion. No client UI/tooltip consumer wiring, World Bundle/Foundation/Cargo change, runtime/persistence/protocol/value authority, or Ability/Creature/NPC/Quest/spatial schema widening. `world_runtime.rs` is excluded unless separately reallocated after fresh evidence.

## Implementation / findings

- PROVEN — live #162 custody amendment `5774684703` adds the exact resource-profile, registry and reproducible measurement-tool paths above; the same canonical #749 lineage and single-writer custody remain binding.
- PROVEN — resource profile acceptance is bound by #504 comment `5775184966` and #749 release comment `5775187195`; the accepted full typed record maxima are 3,555 server bytes and 3,433 client bytes.
- PROVEN — ordinary merge-up `c972f145dad43eb3ebc98e8108a84bebac6605c6` inherits protected main `cf5c5f35476559450b6bbaf87dce519f7eead9d0` without changing the accepted profile inputs.
- IMPLEMENTED — one canonical typed immutable Item model carries six-state field truth, bounded closed vectors and canonical cross-Item references through Project -> Reference -> artifact v4 body v2. Existing v1/v2/v3 codecs remain readable and all-UNKNOWN protected-family records retain their legacy encoding.
- IMPLEMENTED — client-safe data is a positive allowlist projection of the same semantic graph. Temporal, transform, trade, fluid and read/write authority groups are rejected in client records.
- IMPLEMENTED — all 90 B1 candidate fields have exactly one typed destination or explicit unsupported/loss disposition. The mapper leaves new real-corpus semantics UNKNOWN; no Crystal/B1 value promotion or identity regeneration occurs in this generation.
- PROVEN — independent Python-oracle bytes and SHA-256 values match Rust for identity-only, retained materializable core, melee, distance and exact maximum server/client records. The maximum fixture resolves target ordinal 38,156 through a full 38,157-entry synthetic index.

Start from the already-retained B1 source-field -> typed capability matrix in #504 comment `5773340420`. Prefer bounded typed structures over arbitrary maps. Resolve cross-Item references through the protected #737 native map.

## Validation

### Focused

- command/run: `cargo +1.94.0 test -p oteryn-game-server --lib typed_item_codec_tests`
- result: PASS, 5 passed / 0 failed on Rust 1.94.0; exact record maxima, max+1 rejection, independent byte goldens, malformed enums/rationals/vectors and projection denial are covered.

### Component/integration

- command/run: `cargo +1.94.0 fmt --all -- --check`
- result: PASS at `3d69fa467fc41d6ef7b85fa4c1cdca509a060a4b`.
- command/run: six focused integration targets `content_reference_playable`, `content_reference_artifact`, `content_world_project`, `content_world_project_fs`, `content_world_cw2_b1_import`, `content_world_project_publication`
- result: PASS for 71 executed Windows tests / 0 failed across the four platform-active targets. The two filesystem/publication targets compiled but executed zero tests under Windows because they are Linux-gated; hosted Linux qualification remains required.
- command/run: `cargo +1.94.0 clippy --locked --workspace --all-targets -- -D warnings -A dead-code`
- result: PASS at `3d69fa467fc41d6ef7b85fa4c1cdca509a060a4b`; `-A dead-code` isolates two pre-existing Windows-only cfg lints. Strict unmodified Linux clippy remains an exact-head gate.
- command/run: `cargo +1.94.0 test --locked --workspace`
- result: PASS at `3d69fa467fc41d6ef7b85fa4c1cdca509a060a4b` on Windows, including 472 game-server library tests and all workspace/doctest targets; Linux-gated PostgreSQL and filesystem/publication targets remain repository-selected CI work.

### E2E

- scenario: Project -> Reference -> server/client artifact representative Item round-trip
- result: PASS for 11 distinct synthetic capability fixtures plus full-family 38,157 structural compatibility. Fixtures prove schema/codec behavior and do not promote donor gameplay values.

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
last_progress: Accepted resource profile is implemented in the canonical typed Item model and v4 codec; focused Rust, byte-oracle, bounds and Windows integration checks pass
status: implementing
branch: agent/content-world-item-schema-readiness-504
head_sha: 3d69fa467fc41d6ef7b85fa4c1cdca509a060a4b
pr: 749
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
blocker: EXACT_HEAD_LINUX_WORKSPACE_AND_GOVERNED_QUALIFICATION_PENDING
next_action: complete workspace validation, independent exact-head review and repository-selected qualification before freeze
```
