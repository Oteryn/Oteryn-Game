# OTV2-20260919-content-world-cw3-b2-item-creature-loot-semantics-504

```yaml
task_id: OTV2-20260919-content-world-cw3-b2-item-creature-loot-semantics-504
title: CW3-B2 Item + Creature + Loot typed semantic model expansion
mode: IMPLEMENT
status: validating
repository: Oteryn/Oteryn-Game
base_branch: main
branch: agent/content-world-cw3-b2-item-creature-loot-semantics-504
issue: 162
pr: null
base_sha: c22388da53ea721b0ac5fb74ad1b1f95e3b74814
head_sha: pending
final_head_sha: pending
final_head_frozen_at: pending
owner: "Oteryn: content world build"
created_at: 2026-09-19T19:37:00+02:00
updated_at: 2026-09-19T19:56:59+02:00
execution_policy: continuous_progress
owned_paths:
  - apps/game-server/src/content/reference_playable.rs
  - apps/game-server/tests/content_reference_playable.rs
  - docs/agents/tasks/active/OTV2-20260919-content-world-cw3-b2-item-creature-loot-semantics-504.md
public_contracts: []
depends_on:
  - "#671 / protected CW3-B1 typed item semantics"
  - "#674 / protected CW2-B2 candidate creature/spawn evidence"
  - "#676 / protected CW2-B3 candidate loot/item-binding evidence"
blocks: []
cross_repository_coordination_id: null
external_repositories: []
```

## Outcome

Extend the existing Reference playable static semantic graph with the minimum typed Creature and Loot definition families required to compose with the protected CW3-B1 Item family. Preserve Definition-vs-Instance ownership, exact typed revision references, client/server authority boundaries and fail-closed unresolved identity behavior.

## Architecture and source of truth

- **PROVEN** — #162 comment `5743928226` is the live allocation and exact three-path custody authority.
- **PROVEN** — admission base is protected `main@c22388da53ea721b0ac5fb74ad1b1f95e3b74814`.
- **PROVEN** — CW3-B1 keeps `ReferenceItemDefinition` as the canonical typed item family.
- **PROVEN** — the Content World design dossier distinguishes CreatureDefinition from CreatureInstance and requires loot selection algorithms to distinguish independent Bernoulli attempts, weighted selection, guaranteed entries and nested groups.
- **PROVEN** — protected CW2-B2/B3 data remains candidate-only; native imported creature/spawn/item bindings are unresolved.
- **DERIVED** — this child can safely type creature presentation/behavior/loot references without adopting candidate numeric creature stats.
- **DERIVED** — current accepted PPM and count domains can represent bounded Bernoulli and guaranteed-entry shapes; weighted-entry numeric meaning and nested-group binding remain fail-closed until separately accepted.

## High-risk authority/recovery qualification

`NOT_APPLICABLE` — this child changes only an in-memory/static Reference Content semantic API. It performs no production mutation, persistence write, PREPARE/COMMIT, controller/session replacement, recovery interpretation or live authority transition.

## Acceptance criteria

- [x] Item, Creature and Loot families reject Generic fallback.
- [x] Creature presentation/behavior/loot references require exact typed family/key/revision targets.
- [x] Loot entries require exact typed Item targets.
- [x] Bernoulli PPM and count domains validate fail closed; guaranteed entries are distinct.
- [x] Weighted and nested-group executable entry semantics remain fail closed without separately accepted typed semantics.
- [x] Loot selection authority is server-only; client-safe Creature projection exposes presentation only.
- [x] Enumeration order canonicalizes definitions and loot entries.
- [x] Existing CW3-B1 item behavior remains green.
- [x] Tests use only synthetic `oteryn:reference.*` keys and do not promote unresolved CW2 identity.
- [ ] Exact-head repository CI with aggregate `game-gate` passes.
- [ ] One independent HIGH whole-diff semantic/API review passes on the immutable final head.

## Excluded scope

No CW2 importer/evidence mutation; no migration of B2 health/speed/armor/defense/resistances; no native creature/item identity minting; no ItemInstance/value/custody/economy; no loot materialization or reward settlement; no GAME-AI/SIM runtime behavior; no CW4/CW5, protocol/schema/registry, persistence/migrations, Cargo/workspace/lock, workflow/governance/resource-limit, external-repository or production changes.

## Implementation / findings

- Added `DefinitionFamily::Loot`, typed `ReferenceCreatureDefinition`, typed `ReferenceLootDefinition`/`ReferenceLootEntry`, and explicit `ReferenceLootSelectionAlgorithm`.
- Creature definitions bind typed Presentation, Behavior and optional Loot references; all resolve through the existing exact family/key/revision resolver.
- Loot entries bind only typed Item definitions.
- `IndependentBernoulliPpm` requires explicit `probability_ppm <= 1_000_000`; counts require a positive ordered range.
- `GuaranteedEntries` rejects probability fields. Weighted/nested executable entries reject until separately accepted weight/group semantics exist.
- Client projection omits Loot entirely and projects only Creature presentation; Item materialization/destination authority remains omitted as in CW3-B1.
- Definition and loot-entry enumeration is canonicalized before linked output.
- First focused run exposed one existing CW3-B1 error-contract regression caused only by match-arm precedence; reordered validation to preserve the prior wrong-family result while retaining Creature/Loot Generic fail-closed behavior.

## Validation

### Focused

- `cargo +1.94.0 fmt --all --check` — PASS.
- `cargo +1.94.0 test --locked -p oteryn-game-server --test content_reference_playable` — PASS, 27/27.
- `cargo +1.94.0 test --locked -p oteryn-game-server --test content_first_production` — PASS, 4/4.

### Component/integration

- `cargo +1.94.0 clippy --locked -p oteryn-game-server --all-targets -- -D warnings` — PASS.
- `cargo +1.94.0 test --locked -p oteryn-game-server` — PASS.
- `python tools/agents/validate_governance.py` — PASS, 26 required policy documents / 9 project lanes.
- `git diff --check` — PASS.

### E2E

- scenario: `NOT_APPLICABLE`
- reason: static shared semantic-model increment only; no executable creature/loot/materialization runtime is authorized.
- result: `NOT_APPLICABLE`

### Exact-head CI

- final head: pending
- trigger source: pending PR publication
- workflow/run/job: pending
- runner assignment: pending
- classification: pending
- result: pending

## Self-review

- exact head: final staged candidate; immutable commit SHA recorded externally after commit
- method/reviewer: Oteryn: content world build, whole-diff adversarial review
- material findings: 0 open; one pre-freeze B1 match-precedence regression was repaired and requalified
- verdict: `PASS_ZERO_MATERIAL_FINDINGS`

## Independent review

- required: YES — shared Reference Content semantic/API extension per #162 allocation
- exact head: pending
- method/auditor: one independent HIGH whole-diff review; current standing owner authorization covers this single stable-head request
- material findings: pending
- verdict: pending

## PR and closeout

- changed-file review: PASS — exactly three allocated custody paths staged
- unresolved review threads: pending PR
- related/superseded PRs: none
- protected auto-merge: forbidden for worker
- merge commit/result: coordinator-only
- ownership release: pending handoff

## Context checkpoint

```yaml
last_progress: local candidate fully qualified, governance/custody/self-review pass, exactly three allocated paths staged
status: validating
branch: agent/content-world-cw3-b2-item-creature-loot-semantics-504
head_sha: pending
pr: null
final_head_sha: pending
final_head_frozen_at: null
ci_trigger_source: null
ci_check_generation: null
ci_checks_for_current_head: 0
ci_run_ids: []
ci_job_ids: []
runner_assignment_state: unknown
terminal_ci_wait_started_at: null
terminal_ci_checks_for_current_generation: 0
unchanged_state_checks: 0
identical_failure_retries: 0
repair_cycles_for_current_gate: 1
ci_recovery_actions_for_current_head: 0
stall_warnings: 0
owner_action_required: null
blocker: null
next_action: commit and non-force push the exact staged candidate, then open PR and qualify exact-head CI
```