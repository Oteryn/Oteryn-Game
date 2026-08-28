# OTV2-20260824-prep-durability-topology

```yaml
task_id: OTV2-20260824-prep-durability-topology
title: Prepare first Durability implementation topology
mode: CONTRACT
execution_mode: PREPARATION
status: completed
repository: Oteryn/Oteryn-Game
base_branch: main
branch: null
issue: 94
allocation_pr: 118
allocation_merge_sha: 58459c275ba62714741e6794b92d8935b140a37c
pr: 122
allocation_base_sha: 22a3eb866dae19d048969edff1e1fa5012a429b6
worker_base_sha: 58459c275ba62714741e6794b92d8935b140a37c
head_sha: 5f6d4c4440694b5edddf46f4b211e1a30955a4c6
final_head_sha: 5f6d4c4440694b5edddf46f4b211e1a30955a4c6
final_head_frozen_at: 2026-08-24T20:02:45Z
owner: null
created_at: 2026-08-24T21:00:00+02:00
updated_at: 2026-08-24T21:46:00+02:00
execution_budget_minutes: 60
large_budget_reason: null
owned_paths: []
public_contracts:
  - docs/agents/prompts/OTV2_PREP_DURABILITY_TOPOLOGY.md
  - docs/superpowers/plans/2026-08-24-oteryn-game-next-wave-master-plan.md
depends_on:
  - Oteryn-Game#94
blocks:
  - OTV2-IMPL-DURABILITY exact implementation allocation
cross_repository_coordination_id: OTV2-NATIVE-FOUNDATION
external_repositories: []
```

## Outcome

Freeze the exact Issue #94 Durability topology decision packet and future implementation-allocation proposal without mutating runtime, DDL, migrations, dependencies, registries, workflows or production infrastructure.

Preparation result: **COMPLETE**.

Implementation release result: **BLOCKED_ON_OWNER_DECISION** because accepted DUR-03 requires finite hard ceilings and the current registry has no DUR-03-owned entries.

## Architecture and source of truth

- `PROVEN`: coordinator allocation PR #118 merged as `58459c275ba62714741e6794b92d8935b140a37c`; that exact merge is the worker base.
- `PROVEN`: current status marks DUR-01/02/03 and ANL-01 accepted/lifecycle-closed, while FND-03/FND-04/GAME-CHAR/GAME-ITEM concrete seams are implemented.
- `PROVEN`: ADR-0004 selects PostgreSQL / `oteryn_game`; DUR-02 selects one game-owned migration history and no production startup DDL.
- `PROVEN`: DUR-03 section 28 requires finite hard ceilings for touched ItemInstances, location/custody lines, value lines, transform I/O, container expansion, workflow participants, audit contribution and retry/reconciliation work; missing ceilings block implementation.
- `PROVEN`: repository search of `docs/contracts/RESOURCE_LIMITS_REGISTRY.json` returns no `DUR03-` entry.
- `PROVEN`: Issue #93's completed packet owns Ability/Interaction/AI/Movement preparation and does not grant Durability-specific numeric authority.
- `PROVEN`: SQLx 0.9.0 declares Rust 1.94.0 and supplies Tokio/PostgreSQL/pooling/migrations/UUID support; SQLx migration locking defaults true and checksum/history mismatches fail closed.
- `DERIVED`: the bounded game-server-local module is sufficient for the first immediate consumer and avoids a speculative persistence crate/workspace mutation.

## Acceptance criteria

- [x] Exact runtime/migration/test/shared paths are proposed.
- [x] Viable Rust PostgreSQL/migration candidates are compared with Rust 1.94 compatibility evidence.
- [x] One DB client/migration approach is selected with maintenance/security/supply-chain rationale.
- [x] One immutable game-owned migration history and dedicated migration execution path are frozen.
- [x] Production startup auto-DDL is forbidden and schema compatibility fails closed.
- [x] Isolated non-production PostgreSQL test lifecycle is defined.
- [x] Async `PREPARE -> DB COMMIT/CLASSIFY -> RECONCILE` preserves the FND-03 writer-lane boundary.
- [x] Stable TransactionId/OperationId, ambiguous commit and required audit/outbox atomicity are explicit.
- [x] Every DUR-03 amplification/resource family named by section 28 is classified; missing accepted hard maxima remain blockers.
- [x] Final handoff is a precise `BLOCKED_ON_OWNER_DECISION` list rather than an invented implementation release.

## Excluded scope

No runtime code, DDL, migrations, dependencies, Cargo/lockfile, resource-registry mutation, workflow change, database provisioning, production configuration/secrets, Platform write or external-repository mutation. The packet cannot self-grant implementation authority.

## Implementation / findings

### Frozen topology

- game-server-local `apps/game-server/src/durability/**`;
- dedicated `apps/game-server/src/bin/oteryn-game-migrate.rs`;
- one authoritative `apps/game-server/migrations/**` history;
- isolated PostgreSQL DB-E2E under `apps/game-server/tests/**`;
- SQLx `=0.9.0` with minimal explicit Tokio/Postgres/Rustls/migrate/macros/uuid features;
- no new workspace crate and no `workspace-boundaries.toml` mutation;
- serialized shared lease required later for Cargo/lock, game-server composition, Rust workflow and `.gitattributes` only.

### Hard-max blocker

All eight DUR-03 section-28 resource families are `OWNER_DECISION_REQUIRED` until a Durability-specific owner task accepts exact finite values or an exact later implementation child plan excludes individual dimensions fail-closed. `OTV2-IMPL-DURABILITY.write_authority` therefore remains `none`.

## Validation

### Focused

- command/run: source/version compatibility evidence review against SQLx 0.9.0, tokio-postgres 0.7.18, accepted DUR-01/02/03, ANL-01 and current Cargo topology
- result: `PASS` — selected SQLx version matches Rust 1.94; alternative remains viable but has larger client/pool/migration surface.

### Component/integration

- command/run: packet completeness + exact-path/authority/placeholder/whole-diff review
- result: `PASS` before PR creation; repository exact-head governance remains authoritative.

### E2E

- scenario: `NOT_APPLICABLE` — preparation packet only; real PostgreSQL DB-E2E is a mandatory later implementation gate
- result: `NOT_APPLICABLE`

### Exact-head CI

- final head: `5f6d4c4440694b5edddf46f4b211e1a30955a4c6`
- trigger source: PR #122
- workflow/run/job: Merge gate run `32771591699`; `game-gate` job `97573336161`
- runner assignment: GitHub-hosted Actions
- classification: exact-head canonical aggregate
- result: `SUCCESS`

## Self-review

- exact head: `5f6d4c4440694b5edddf46f4b211e1a30955a4c6`; review `PRR_kwDOT8SzxM8AAAABKr2mRw`
- method/reviewer: preparation agent whole-diff review against Issue #94, accepted DUR/FND/GAME/ANL contracts and exact allocation
- material findings: `P0=0 / P1=0 / P2=0` before PR creation
- verdict: `PASS`

## Independent review

- required: `NO` for this preparation-only packet because it changes no runtime/schema/registry/authority semantics; the later persistence/item/value implementation is explicitly `YES`
- exact head: `NOT_APPLICABLE`
- method/auditor: `NOT_APPLICABLE`
- material findings: `NOT_APPLICABLE`
- verdict: `NOT_APPLICABLE`

## PR and closeout

- changed-file review: `PASS` - exactly packet + worker task
- unresolved review threads: `0`
- related/superseded PRs: allocation #118; Issue #94
- protected auto-merge: not used
- merge commit/result: PR #122 squash merged as `c92d2d0615ae1e969003d152b4b0dfa87acfb72d`
- ownership release: completed; worker branch is absent

## Context checkpoint

```yaml
last_progress: PR #122 squash-merged the complete Issue #94 topology packet as c92d2d0615ae1e969003d152b4b0dfa87acfb72d; exact-head game-gate passed; Issue #123 owns the remaining DUR-03 hard-max owner decision; worker ownership is released
status: completed
branch: null
head_sha: 5f6d4c4440694b5edddf46f4b211e1a30955a4c6
pr: 122
final_head_sha: 5f6d4c4440694b5edddf46f4b211e1a30955a4c6
final_head_frozen_at: 2026-08-24T20:02:45Z
ci_trigger_source: pull_request_122
ci_check_generation: merge_gate_run_32771591699
ci_checks_for_current_head: 4
ci_run_ids: [32771591562, 32771591648, 32771591745, 32771591699]
ci_job_ids: [97573336161]
runner_assignment_state: completed
terminal_ci_wait_started_at: null
terminal_ci_checks_for_current_generation: 4
unchanged_state_checks: 0
identical_failure_retries: 0
repair_cycles_for_current_gate: 0
ci_recovery_actions_for_current_head: 0
stall_warnings: 0
owner_action_required: issue_123_before_durability_implementation_allocation
blocker: none_for_preparation; implementation_blocked_by_issue_123
next_action: none_terminal
```
