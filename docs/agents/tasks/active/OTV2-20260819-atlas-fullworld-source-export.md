# OTV2-20260819-atlas-fullworld-source-export

```yaml
task_id: OTV2-20260819-atlas-fullworld-source-export
title: Game-owned Atlas full-world source projection adapter
mode: IMPLEMENT
status: implementing
repository: Oteryn/Oteryn-Game
base_branch: main
branch: feat/OTV2-20260819-atlas-fullworld-source-export
pr: null
base_sha: 63a6cb8cb3e69b7c2f792475f24093e90bd7fd81
head_sha: null
final_head_sha: null
final_head_frozen_at: null
owner: ChatGPT autonomous execution
created_at: 2026-08-19T09:52:53+02:00
updated_at: 2026-08-19T09:52:53+02:00
execution_budget_minutes: 120
large_budget_reason: Required cross-repository prerequisite for heavy full-world local generation and exact-source qualification.
owned_paths:
  - tools/game-atlas-fullworld-source/**
  - docs/agents/tasks/active/OTV2-20260819-atlas-fullworld-source-export.md
  - docs/agents/evidence/OTV2-20260819-atlas-fullworld-source-export.md
public_contracts: []
depends_on: []
blocks:
  - ATLAS-FULLWORLD-LOCAL-GENERATION-FABRIC
cross_repository_coordination_id: ATLAS-FULLWORLD-COORDINATOR
external_repositories:
  - Oteryn/Oteryn-Atlas
```

## Outcome

Provide a Game-owned offline API that projects every tile/floor of the exact accepted migration source with the same qualified semantic transform used by DYN-ATLAS-001, so Atlas local generation never calls proof-private Game functions or treats OTBM as Atlas authority.

## Architecture and source of truth

### PROVEN

- `Oteryn/Oteryn-Game@63a6cb8cb3e69b7c2f792475f24093e90bd7fd81` is the programme's pinned Game source revision at task start.
- `tools/game-atlas-thais-fixture/README.md` labels the existing producer a bounded proof implementation and fixes it to Thais Z7.
- `Oteryn/Oteryn-Atlas/docs/agents/tasks/active/ATLAS-FULLWORLD-COORDINATOR.md` permits a separate Game task/branch/PR when the local generation agent proves an exporter/tooling change is required.
- The exact migration inputs remain digest-pinned: legacy repository `e417c5e7c22986bf4acef0495eb47f7b72c97cce`, `world.otbm` SHA-256 `3bd40d14fefec41f24c4b3ae879e420be1a831ef55b95dcbec721e587a09b034`, and `15.32.zip` SHA-256 `1a6bad8b7598cd874f534cd4aae2d249fb3d9b4458b3ccfa75754f91bb27870f`.

### DERIVED

- A Game-owned adapter may reuse the qualified per-tile DYN transform while broadening only spatial selection. This does not change the semantic contract or make the bounded JSONL package permanent.

### UNKNOWN

- Full-world counts/floors/bounds remain unknown until the local generation fabric executes against this adapter.

## Acceptance criteria

- [ ] New API lives under `tools/game-atlas-fullworld-source/**` and owns no Atlas publication/runtime decisions.
- [ ] Exact input validation remains delegated to the qualified Game producer and fails closed for other digests.
- [ ] Exact pinned Thais Z7 projection through the new API is byte-identical to the already-qualified `tiles.jsonl` SHA-256 `ff14efee3fc376d8f18432c628294c64ffe89450a59aaa498a28e6d705815984`.
- [ ] Full-world caller can stream every record exactly once and project any tile without importing proof-private functions from Atlas.
- [ ] Focused validation, self-review, changed-file review and exact-head CI are recorded before closeout.

## Excluded scope

No final Atlas serializer/chunk format, browser/runtime change, semantic layers, canonical entity guessing, pixel redistribution, Synology deployment or Atlas PR merge.

## Implementation / findings

The adapter deliberately delegates the already-qualified semantic transform to the bounded DYN implementation inside the same Game repository. The new API changes only the addressable spatial scope and exposes a stable Game-owned call surface for the full-world local generation programme.

## Validation

### Focused

- command/run: pending
- result: pending

### Component/integration

- command/run: pending
- result: pending

### E2E

- scenario: exact pinned Thais Z7 byte-parity replay through the new producer API
- result: pending

### Exact-head CI

- final head: pending
- trigger source: pending
- workflow/run/job: pending
- runner assignment: pending
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
- related/superseded PRs: pending
- protected auto-merge: pending
- merge commit/result: pending
- ownership release: pending

## Context checkpoint

```yaml
last_progress: Game-owned full-world adapter and authority-boundary README added on the dedicated branch.
status: implementing
branch: feat/OTV2-20260819-atlas-fullworld-source-export
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
next_action: run exact pinned Thais parity through the new Game producer API
```
