# OTV2-20260912-ref-combat-death-corpse-loot-evidence

```yaml
task_id: OTV2-20260912-ref-combat-death-corpse-loot-evidence
title: Close Reference ordinary death/corpse/loot evidence for #506/#513
mode: AUDIT
status: validating
repository: Oteryn/Oteryn-Game
base_branch: main
branch: agent/ref-combat-death-corpse-loot-evidence-506-513
pr: null
base_sha: 1a9cb71f424a821633fd42f8a1a19920ffeff2c3
head_sha: null
final_head_sha: null
final_head_frozen_at: null
owner: ChatGPT GPT-5.6 Sol
created_at: 2026-09-12T09:13:00+02:00
updated_at: 2026-09-12T09:13:00+02:00
execution_policy: continuous_progress
owned_paths:
  - docs/agents/tasks/active/OTV2-20260912-ref-combat-death-corpse-loot-evidence.md
  - docs/agents/evidence/OTV2-20260912-reference-combat-death-corpse-loot-chain.md
public_contracts: []
depends_on:
  - "#483"
  - "#506"
  - "#513"
blocks: []
cross_repository_coordination_id: null
external_repositories: []
```

## Outcome

Retain one official-first evidence pack for the ordinary single-player creature path required by #506/#513:

`committed lethal result -> death -> XP consequence + corpse/loot selection -> protected corpse interaction -> durable pickup boundary`.

The evidence pack must preserve the immutable Reference target `global-tibia-observable-2026-07-28-post-server-save`, keep party XP/PvP/boss/quick-loot breadth out of scope, and must not invent Global durability ownership or transaction internals.

## Architecture and source of truth

- `PROVEN`: live #506 binds the first Combat death workflow and owner separation.
- `PROVEN`: live #513 binds the DUR-03 materialization/pickup boundary and durable item/value ownership.
- `PROVEN`: #483 binds the official-first source hierarchy and immutable 2026-07-28 target.
- `DERIVED`: ordinary Global corpse authority uses the highest-damage principal during the protected 10-second interval, with corpse immovability, from a long official CipSoft continuity chain.
- `PROVEN`: the 2026-07-28 CipSoft balancing note gives the direct Iceplume Strider XP boundary value `8,150`.
- `UNKNOWN`: exact server-tick/rounding semantics at the 10-second expiry boundary.
- `UNKNOWN`: proprietary/internal Global durable item identity, commit, custody and retry implementation.
- `CONFLICT`: treating last hit as universal loot ownership, treating one generic killer owner as both XP and loot authority, or claiming all loot is necessarily rolled at death.

## High-risk authority/recovery qualification

```yaml
applicable: false
reason: docs-only read-only evidence retention; no production mutation, PREPARE/COMMIT authority, controller installation, durable write, recovery interpretation or live-state mutation
```

## Acceptance criteria

- [x] Evidence is restricted to ordinary single-player creature death/corpse/loot semantics required by #506/#513.
- [x] Atomic claims are classified conservatively as `PROVEN`, `DERIVED`, `UNKNOWN` or `CONFLICT`.
- [x] Exact target continuity is distinguished from current/historical official evidence.
- [x] Kill attribution, XP consequence, corpse creation/access, protected window/immovability, ordinary corpse interaction and loot-selection/durable-pickup boundary are all covered.
- [x] Questions owned by #506 and #513 are separated explicitly.
- [x] Party/shared XP, PvP, bosses/events and quick-loot breadth are excluded.
- [x] No Global durability ownership or proprietary transaction internals are invented.
- [x] Positive and negative minimal fixture shapes are retained.

## Excluded scope

No runtime/client/protocol/Cargo/workflow/registry/contract/manifest/persistence/schema/migration/production mutation. No party/shared XP breadth, PvP/player-death behavior, bosses/events, broad quick-loot aggregation, exact corpse decay timing, or Evolved mechanics. No OTS implementation is used as Reference truth.

## Implementation / findings

The research is retained in `docs/agents/evidence/OTV2-20260912-reference-combat-death-corpse-loot-chain.md`.

The main evidence result is owner separation:

```text
Global observable semantics
  -> kill/death consequence
  -> XP contribution consequence
  -> corpse + protected access rule
  -> ordinary corpse/item selection

Oteryn authority boundary
  Combat loot-selection intent
  -> DUR-03 materialization
  -> LOOT_READY
  -> pickup TRANSFER of the same durable ItemInstance
```

No durability ownership is inferred from Global public behavior.

## Validation

### Focused

- command/run: `python tools/agents/validate_governance.py`
- result: pending exact-head CI/local-capable validation

### Component/integration

- command/run: `NOT_APPLICABLE` — documentation/evidence only
- result: `NOT_APPLICABLE`

### E2E

- scenario: `NOT_APPLICABLE` — no runtime behavior changed
- result: `NOT_APPLICABLE`

### Exact-head CI

- final head: pending
- trigger source: pull request
- workflow/run/job: pending
- runner assignment: pending
- classification: neutral documentation / governance docs; exact GitHub classifier remains authority
- result: pending

## Self-review

- exact head: pending
- method/reviewer: implementing/coordinating agent
- material findings: pending
- verdict: pending

## Independent review

- required: NO — docs-only retained evidence, no contract/runtime/authority mutation; repository CI and self-review remain required
- exact head: `NOT_APPLICABLE`
- method/auditor: `NOT_APPLICABLE`
- material findings: `NOT_APPLICABLE`
- verdict: `NOT_APPLICABLE`

## PR and closeout

- changed-file review: pending
- unresolved review threads: pending
- related/superseded PRs: none known
- protected auto-merge: forbidden; Merge Queue policy remains authoritative if integration is later authorized
- merge commit/result: pending
- ownership release: pending

## Context checkpoint

```yaml
last_progress: dedicated docs-only branch created from protected main and evidence retention started
status: validating
branch: agent/ref-combat-death-corpse-loot-evidence-506-513
head_sha: null
pr: null
final_head_sha: null
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
repair_cycles_for_current_gate: 0
ci_recovery_actions_for_current_head: 0
stall_warnings: 0
owner_action_required: null
blocker: null
next_action: add the bounded official-first evidence document and open a docs-only PR
```
