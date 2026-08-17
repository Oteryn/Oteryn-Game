# OTV2-20260816-stage-c-vsl-contracts

```yaml
task_id: OTV2-20260816-stage-c-vsl-contracts
title: Close Stage-C movement, combat and content vertical-slice architecture
mode: COORDINATE
status: validating_owner_acceptance
repository: blakinio/Oteryn-v2
base_branch: main
branch: docs/stage-c-vsl-contracts-20260816
issue: 310
pr: 311
base_sha: bf2a2ae279516f62626a5d8f4dc1aeb587535c62
owner: Architecture Coordinator
created_at: 2026-08-16T21:16:12+02:00
updated_at: 2026-08-16T22:14:00+02:00
execution_budget_minutes: 120
owned_paths:
  - docs/agents/tasks/active/OTV2-20260816-stage-c-vsl-contracts.md
  - docs/architecture/VSL-MOVE-01_MINIMAL_MOVEMENT_VISIBILITY_CONTRACT_CANDIDATE.md
  - docs/architecture/VSL-COMBAT-01_MINIMAL_COMBAT_DEATH_LOOT_CONTRACT_CANDIDATE.md
  - docs/architecture/VSL-CONTENT-01_MINIMAL_NATIVE_CONTENT_SLICE_CONTRACT_CANDIDATE.md
  - docs/architecture/OTERYN_V2_STAGE_C_VSL_OWNER_DECISION_PACKAGE_20260816.md
  - docs/architecture/OTERYN_V2_STAGE_C_VSL_OWNER_ACCEPTANCE_20260816.md
public_contracts:
  - docs/architecture/VSL-MOVE-01_MINIMAL_MOVEMENT_VISIBILITY_CONTRACT_CANDIDATE.md
  - docs/architecture/VSL-COMBAT-01_MINIMAL_COMBAT_DEATH_LOOT_CONTRACT_CANDIDATE.md
  - docs/architecture/VSL-CONTENT-01_MINIMAL_NATIVE_CONTENT_SLICE_CONTRACT_CANDIDATE.md
  - docs/architecture/OTERYN_V2_STAGE_C_VSL_OWNER_DECISION_PACKAGE_20260816.md
  - docs/architecture/OTERYN_V2_STAGE_C_VSL_OWNER_ACCEPTANCE_20260816.md
blocks:
  - genuinely independent exact-head review because VSL-COMBAT-01 touches durable loot/value invariants
  - final lifecycle/current-status reconciliation
  - executor-prompt handoff audit
cross_repository_coordination_id: OTV2-NATIVE-FOUNDATION
external_repositories: []
```

## Outcome

Close only the minimum Stage-C architecture needed so implementation agents can build the first real-boundary movement/combat/content vertical slice without making architecture decisions inside code.

## Trusted state

- `PROVEN` — first-wave architecture is owner-accepted on `main` through PR #309.
- `PROVEN` — the Stage-C candidates preserve FND-02/FND-03/FND-04, DUR-03/DUR-04, GAME-ABILITY, GAME-INTERACTION, GAME-AI, GAME-ITEM, GAME-CHAR, ALPHA-CLIENT and Reference fail-closed boundaries.
- `PROVEN` — the owner explicitly clarified that acceptance applies to all remaining architecture decisions, including `VSL-MOVE-01`, `VSL-COMBAT-01` and `VSL-CONTENT-01`.
- `PROVEN` — `OTERYN_V2_STAGE_C_VSL_OWNER_ACCEPTANCE_20260816.md` records those three dispositions as `ACCEPTED` while keeping implementation `NOT_STARTED`.
- `PROVEN` — root `AGENTS.md` requires genuinely independent review for durable loot/value invariant changes.
- `PROVEN` — the prior Stage-C architecture semantic workflow did not constitute that independent review because its Stage-C profile was `NOT_APPLICABLE`.

## Accepted result

```text
VSL-MOVE-01:    ACCEPTED
VSL-COMBAT-01:  ACCEPTED
VSL-CONTENT-01: ACCEPTED
```

Acceptance does not authorize implementation, production, DDL, final content-format selection, entitlement work or Reference parity.

## Review/repair history

- Initial draft exact head `54cfa3325027825b6b792409b013809208ff33e6`: Architecture semantic audit PASS, Merge authority audit PASS; Agent governance failed only because the PR title exceeded 72 characters.
- PR title repaired to `docs(architecture): define Stage-C VSL contracts`.
- Post-repair head `6d817f38ebb91113886dfad9a4ca4c4baf3b707c` passed required repository workflow gates and had zero review threads / zero requested changes / `behind_by=0` at owner handoff.
- Owner acceptance was then recorded on a newer head; all exact-head validation and required independent review must therefore be repeated for the final acceptance head.

## Acceptance criteria

- [x] Movement authority, retry, visibility and scope-handoff boundaries are explicit.
- [x] Combat/death/loot/XP/pickup identity and anti-duplication boundaries are explicit.
- [x] Content semantic/compiler/loader/activation seams preserve DUR-04 final-format spike.
- [x] Exact Reference values remain evidence-gated.
- [x] Owner accepted all three Stage-C gates.
- [ ] Full-diff self-review is clean on the final acceptance head.
- [ ] Fresh Agent governance / Merge authority / applicable CI / `Merge gate / validate` pass on one unchanged final head.
- [ ] Genuinely independent review of the exact final Stage-C acceptance head has no open material finding.
- [ ] Zero unresolved review threads/requested changes and `behind_by=0` before merge.
- [ ] Post-merge status/register/index and lifecycle closeout are complete.
- [ ] Final executor prompt DAG is audited and released before any implementation worker is started.

## Hard exclusions

No runtime/client/server/protocol/content implementation; no PostgreSQL DDL/migration; no final World Project/Bundle physical encoding; no exact Global movement/combat/loot/XP formula; no production/deployment; no entitlement implementation; no Reference parity promotion; no cross-repository write.

## Executor state

```text
EXECUTOR_PROMPTS: HOLD
IMPLEMENTATION_AUTHORITY: NONE
```

## Context checkpoint

```yaml
last_progress: owner acceptance recorded for all three Stage-C gates on PR #311
status: validating_owner_acceptance
branch: docs/stage-c-vsl-contracts-20260816
issue: 310
pr: 311
owner_action_required: none for architecture disposition
blocker: genuinely independent exact-head review required for durable loot/value semantics
next_action: complete fresh exact-head CI/self-review, obtain genuinely independent review, merge Stage-C, then reconcile closeout/status and finish the executor prompt DAG
executor_prompts: HOLD
```

`IMPLEMENTATION_AUTHORITY: NONE`
