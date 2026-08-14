# OTV2-20260815-game-interaction-architecture

```yaml
task_id: OTV2-20260815-game-interaction-architecture
title: Design GAME-INTERACTION-01 world interaction architecture
mode: ARCHITECTURE
status: ready
repository: blakinio/Oteryn-v2
base_branch: main
branch: docs/arch-d-game-interaction
pr: 269
base_sha: 088b46638ac014cd7928d6b0b75cee44902fe22c
head_sha: c0b064a16dc8b47d529d7a1679d4438fea9acfe0
final_head_sha: null
final_head_frozen_at: null
owner: DOMAIN_ARCHITECTURE_DESIGN_AGENT_D
created_at: 2026-08-15T00:18:00+02:00
updated_at: 2026-08-15T00:38:00+02:00
execution_budget_minutes: 60
owned_paths:
  - docs/agents/tasks/active/OTV2-20260815-game-interaction-architecture.md
  - docs/architecture/GAME-INTERACTION-01_WORLD_INTERACTION_ANALYSIS.md
  - docs/architecture/GAME-INTERACTION-01_WORLD_INTERACTION_CONTRACT_CANDIDATE.md
public_contracts:
  - docs/architecture/GAME-INTERACTION-01_WORLD_INTERACTION_CONTRACT_CANDIDATE.md
depends_on:
  - FND-03 accepted authoritative runtime execution contract
  - GAME-CHANNEL-01 accepted channel product policy
  - GAME-ITEM-01 accepted item model/equipment contract
  - DUR-03 accepted item transaction and anti-duplication contract
  - DUR-04 accepted content/world/scripting contract
  - SIM-DETERMINISM-01 accepted authoritative simulation contract
  - GAME-ABILITY-01 owner-accepted partial baselines; overall gate remains open
blocks:
  - GAME-INTERACTION-01 implementation authority until coordinator canonicalization and a later implementation task
cross_repository_coordination_id: OTV2-NATIVE-FOUNDATION
external_repositories: []
```

## Outcome

Produce a bounded, paper-only GAME-INTERACTION-01 analysis and contract candidate for Architecture Coordinator audit. The package covers interaction intent/routing, server authority, deterministic object state transitions, doors/switches/levers/teleports/fields/traps/hazards, readable/writable objects, item-assisted use, movement/timer triggers, multichannel scope, reset/persistence/recovery/content-revision migration, script capability boundaries and anti-abuse/resource-limit obligations.

The task grants **no runtime, client, protocol, database/DDL, Platform, production or merge authority**.

## Architecture/source truth

Primary authority consumed by this worker:

- `AGENTS.md` and `AGENTS.override.md`;
- `docs/agents/AGENTS.md`;
- `docs/agents/prompts/OTV2_DOMAIN_ARCHITECTURE_DESIGN_AGENT.md`;
- `docs/agents/MULTI_AGENT_ARCHITECTURE_ORCHESTRATION.md`;
- `docs/agents/programs/OTERYN_V2_ARCHITECTURE_PARALLEL_WORK_ALLOCATION.md`;
- `docs/architecture/FND-03_RUNTIME_EXECUTION_CONTRACT.md`;
- `docs/architecture/GAME-CHANNEL-01_CHANNEL_PRODUCT_POLICY_CONTRACT.md`;
- `docs/architecture/GAME-ITEM-01_ITEM_MODEL_AND_EQUIPMENT_CONTRACT.md`;
- `docs/architecture/DUR-03_ITEM_TRANSACTION_AND_ANTI_DUPLICATION_CONTRACT.md`;
- `docs/architecture/DUR-04_CONTENT_WORLD_AND_SCRIPTING_CONTRACT.md`;
- `docs/architecture/SIM-DETERMINISM-01_AUTHORITATIVE_SIMULATION_CONTRACT.md`;
- GAME-ABILITY-01 owner-accepted partial baselines;
- foundation error/resource/failure registries.

## Acceptance checklist

- [x] analysis covers required GAME-INTERACTION-01 taxonomy and alternatives
- [x] candidate declares authoritative owner and scope model
- [x] item legality/location/value conservation remain delegated to GAME-ITEM/DUR-03
- [x] ability/combat semantics remain delegated to GAME-ABILITY
- [x] scripts remain bounded proposal-only components under DUR-04
- [x] World/Channel/Instance state obeys FND-03 and GAME-CHANNEL ownership
- [x] deterministic ordering/replay requirements obey SIM-DETERMINISM-01
- [x] reset, persistence, recovery and content-revision migration are explicit
- [x] anti-abuse/resource classes are explicit without invented numeric limits
- [x] deterministic acceptance/failure scenarios are included
- [x] `DECISIONS_NOT_TAKEN` and `CROSS_DOMAIN_FINDINGS` are explicit
- [x] manual governance/authority/scope review is complete
- [ ] exact-head ordinary PR CI is inspected; final immutable evidence must be recorded externally on PR #269 without moving the validated head
- [x] full pre-freeze diff self-review is complete and identified metadata findings were repaired
- [x] draft PR remains draft for Architecture Coordinator audit

## Excluded

- Rust/runtime/script implementation;
- protocol/client UI implementation;
- PostgreSQL DDL/migrations;
- Platform or external-repository changes;
- GAME-ITEM item legality/location semantics;
- DUR-03 value-conservation mechanics;
- GAME-ABILITY targeting/combat/effect-formula ownership;
- AI behavior policy;
- coordinator-only global architecture/status overlays;
- Reference-specific interaction formulas/ordering without sufficient evidence;
- numeric resource maxima without measured/accepted evidence;
- production activation, deployment or merge.

## Findings

Five report-only cross-domain findings are recorded in both architecture artifacts:

1. `GAME-INTERACTION-01-XD-ABILITY-TRIGGER` — GAME-ABILITY non-player-origin effect invocation/coupled-failure boundary;
2. `GAME-INTERACTION-01-XD-MOVEMENT-RELOCATION` — movement occurrence/relocation/handoff owner surface;
3. `GAME-INTERACTION-01-XD-WRITABLE-TEXT` — durable player-authored text ownership/moderation/retention;
4. `GAME-INTERACTION-01-XD-COUPLED-WORKFLOW` — named owner-specific multi-owner coupled workflows;
5. `GAME-INTERACTION-01-XD-RESOURCE-LIMITS` — measured numeric interaction limits and shared registry entries.

All remain `REPORT_ONLY`; this worker changed no sibling-domain or coordinator-owned contract.

## Validation

### Focused

Manual exact-scope/authority review against the worker prompt, orchestration policy, allocation, status vocabulary, FND-03, GAME-CHANNEL-01, GAME-ITEM-01, DUR-03, DUR-04, SIM-DETERMINISM-01 and the shared failure/error/resource policies is complete.

A local checkout-based repository validator could not be executed in the current sandbox because `github.com` DNS resolution failed. This is recorded as a limitation and is **not** claimed as a local PASS. Final repository validation therefore relies on ordinary GitHub Actions for the unchanged PR head.

### Component

`NOT_APPLICABLE` — documentation/architecture only; no executable component changed.

### E2E

`NOT_APPLICABLE` — paper-only architecture change with no runtime/client/protocol/DDL boundary.

### Exact-head CI

The first exact-head generation on `c0b064a16dc8b47d529d7a1679d4438fea9acfe0` produced:

- `Merge authority audit` run `31847103092` — `SUCCESS`;
- `Agent governance` run `31847103148` — `FAILURE` in the metadata pre-check before checkout;
- `Merge gate` run `31847103088` — queued in the observed generation.

Root cause was deterministic and non-architectural: `.github/workflows/agent-governance.yml` requires exact PR body headings `## Summary`, `## Scope` and `## Validation`, while the initial worker handoff body used uppercase variants and `## OWNED_PATHS`. The PR body was repaired to the workflow-required headings **without moving the branch head**.

This task-record update is a material CI bookkeeping repair that records the failure/root cause and intentionally creates a new `synchronize` head so ordinary pull-request workflows consume the corrected current PR metadata. Final exact-head results must be recorded in GitHub checks/a PR checkpoint comment rather than by another file edit.

## Self-review

Full worker diff review identified and repaired these handoff findings before final validation:

1. task metadata initially had `pr: null` and an authoring-stage checkpoint;
2. the analysis header initially used descriptive `PROPOSED ANALYSIS` rather than canonical `DecisionStatus: PROPOSED`;
3. the PR body initially did not satisfy the workflow's case-sensitive/shape-sensitive `## Summary`, `## Scope`, `## Validation` metadata contract; the body is now repaired.

No open material architecture finding was identified. Final unchanged-head review must confirm this after the current CI-repair checkpoint commit.

Explicitly checked:

- no whole-gate `ACCEPTED` claim;
- no runtime/client/protocol/DDL/Platform/production authority;
- no coordinator-only overlay edit;
- no arbitrary script mutation authority;
- no process-global mutable interaction owner;
- no GAME-ITEM/DUR-03 value re-ownership;
- no GAME-ABILITY combat/effect re-ownership;
- no AI behavior policy;
- no generic cross-domain transaction escape hatch;
- no invented Reference behavior or numeric resource limits.

Worker self-review is not independent Architecture Coordinator acceptance.

## Independent review

Architecture Coordinator audit is the only authorized next review stage for this worker lane. No owner-funded external/Codex/OpenAI review is authorized for this task.

## PR closeout

Worker closeout is limited to `INTEGRATION_READY — DRAFT PR — COORDINATOR ACTION REQUIRED` after final exact-head evidence is clean. The worker must not merge, mark ready-for-review, archive this task, close issue #262 or perform lifecycle closeout.

## Context checkpoint

```yaml
last_progress: GAME-INTERACTION-01 analysis/candidate are complete; pre-freeze metadata/status findings are repaired. Exact-head CI on c0b064a16dc8b47d529d7a1679d4438fea9acfe0 exposed a PR metadata-heading mismatch before checkout; the PR body is corrected and this material task checkpoint triggers a fresh ordinary synchronize generation for the corrected metadata.
status: ready
branch: docs/arch-d-game-interaction
head_sha: c0b064a16dc8b47d529d7a1679d4438fea9acfe0
pr: 269
final_head_sha: null
final_head_frozen_at: null
ci_trigger_source: pull_request
ci_check_generation: metadata-repair-synchronize-pending
ci_checks_for_current_head: 3
ci_run_ids:
  - 31847103148
  - 31847103092
  - 31847103088
ci_job_ids:
  - 94915672726
runner_assignment_state: assigned-on-prior-generation
terminal_ci_wait_started_at: null
terminal_ci_checks_for_current_generation: 0
unchanged_state_checks: 2
identical_failure_retries: 0
repair_cycles_for_current_gate: 2
ci_recovery_actions_for_current_head: 0
stall_warnings: 0
owner_action_required: false
blocker: null
next_action: Architecture Coordinator audit PR #269 after the fresh exact-head ordinary CI generation passes; keep the PR draft and do not merge or lifecycle-close.
```

MERGE_AUTHORITY: ARCHITECTURE_COORDINATOR_ONLY
