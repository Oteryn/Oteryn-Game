# OTV2-20260824-prep-durability-topology

```yaml
task_id: OTV2-20260824-prep-durability-topology
title: Prepare first Durability implementation topology
mode: CONTRACT
execution_mode: PREPARATION
status: validating
repository: Oteryn/Oteryn-Game
base_branch: main
branch: docs/otv2-prep-durability-topology-94
issue: 94
allocation_pr: 118
allocation_merge_sha: 58459c275ba62714741e6794b92d8935b140a37c
pr: null
allocation_base_sha: 22a3eb866dae19d048969edff1e1fa5012a429b6
worker_base_sha: 58459c275ba62714741e6794b92d8935b140a37c
head_sha: null
final_head_sha: null
final_head_frozen_at: null
owner: OTV2-PREP-DURABILITY-TOPOLOGY
created_at: 2026-08-24T21:00:00+02:00
updated_at: 2026-08-24T21:46:00+02:00
execution_budget_minutes: 60
large_budget_reason: null
owned_paths:
  - docs/architecture/reviews/OTERYN_GAME_DURABILITY_TOPOLOGY_DECISION_PACKET_2026-08-24.md
  - docs/agents/tasks/active/OTV2-20260824-prep-durability-topology.md
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

- final head: pending PR freeze
- trigger source: pending PR
- workflow/run/job: repository `game-gate` after PR creation
- runner assignment: GitHub Actions
- classification: required docs/governance exact-head gate
- result: pending PR generation

## Self-review

- exact head: to be bound in immutable PR review evidence after PR metadata is created
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

- changed-file review: packet + this worker task only
- unresolved review threads: pending PR creation
- related/superseded PRs: allocation #118; Issue #94
- protected auto-merge: disabled until exact-head checks
- merge commit/result: pending PR
- ownership release: coordinator closeout after packet merge

## Context checkpoint

```yaml
last_progress: Issue #94 topology packet written from exact allocation merge 58459c275ba62714741e6794b92d8935b140a37c; topology is complete and implementation release is truthfully blocked on DUR-03 hard-max owner decisions
status: validating
branch: docs/otv2-prep-durability-topology-94
head_sha: null
pr: null
final_head_sha: null
final_head_frozen_at: null
ci_trigger_source: pull_request_after_open
ci_check_generation: null
ci_checks_for_current_head: 0
ci_run_ids: []
ci_job_ids: []
runner_assignment_state: not_started
terminal_ci_wait_started_at: null
terminal_ci_checks_for_current_generation: 0
unchanged_state_checks: 0
identical_failure_retries: 0
repair_cycles_for_current_gate: 0
ci_recovery_actions_for_current_head: 0
stall_warnings: 0
owner_action_required: DUR-03 hard-max acceptance belongs to a separate Durability-specific owner decision task; no owner action is required to merge this truthful preparation packet
blocker: OTV2-IMPL-DURABILITY remains blocked; preparation packet itself has no blocker before PR validation
next_action: open the exact two-file preparation PR, bind whole-diff self-review to its head, pass game-gate, squash merge, then archive/release the worker task and close Issue #94 preparation
```
