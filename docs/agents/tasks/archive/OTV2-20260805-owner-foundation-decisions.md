# OTV2-20260805-owner-foundation-decisions

```yaml
task_id: OTV2-20260805-owner-foundation-decisions
title: Record owner-approved repository, admission and PostgreSQL decisions
mode: CONTRACT
status: completed
repository: blakinio/Oteryn-v2
base_branch: main
branch: docs/foundation-decisions-20260805
pr: 5
base_sha: 0eb3c329fd04211e1bb5c6e3f219f4daad5e500a
head_sha: cb838a759247eb81cdac21c47f0b4376f64294ef
merge_sha: 2d591d78fa4986469e4b0cdd87f4fa8a06cbc4a8
owner: released
created_at: 2026-08-05T09:28:00+02:00
updated_at: 2026-08-05T09:52:00+02:00
execution_budget_minutes: 60
large_budget_reason: null
owned_paths: []
public_contracts:
  - docs/architecture/ADR-0002-repository-ownership-and-client-migration.md
  - docs/architecture/ADR-0003-platform-identity-game-gateway-and-admission-boundary.md
  - docs/architecture/ADR-0004-postgresql-and-data-ownership.md
depends_on:
  - docs/architecture/ADR-0001-native-rust-multichannel-platform.md
blocks: []
cross_repository_coordination_id: OTV2-NATIVE-FOUNDATION
external_repositories:
  - blakinio/Oteryn-Platform
  - blakinio/otclient
  - blakinio/Otheryn
```

## Outcome

The owner decisions from the 2026-08-05 architecture conversation are now durable and merged. Future agents do not need the chat transcript to know that:

- the native Rust client moves into `blakinio/Oteryn-v2`;
- client, server and `protocol-oteryn` share one canonical Rust repository;
- Identity and Game Gateway remain in `blakinio/Oteryn-Platform`;
- the initial Game Gateway remains implemented in Go;
- Oteryn v2 does not add a second classic login server, password verifier or OAuth authority;
- PostgreSQL is the target database technology;
- Platform and game state use separate logical databases, owners, credentials and migrations;
- `protocol-oteryn` is not yet implemented or proven end to end.

## Delivered contracts

### ADR-0002 — repository ownership and client migration

- `blakinio/Oteryn-v2` is the canonical destination for the Rust client, Rust game server and shared crates.
- The migration must pin the exact source SHA, preserve provenance/history and prevent two active client copies.
- `protocol-canary` is not part of the target runtime.

### ADR-0003 — Platform Identity and Game Gateway boundary

- Platform remains the reusable-credential and account-security authority.
- Game Gateway remains a Platform control-plane service and the initial implementation stays in Go.
- The Rust client obtains a Game Session through Platform/Gateway; the Rust game server validates admission and owns gameplay.
- A later Gateway rewrite requires a separate evidence-backed ADR.
- The existing Platform native contract must be reconciled with the Rust-server target before implementation.

### ADR-0004 — PostgreSQL and data ownership

- PostgreSQL is selected for the greenfield game persistence and target Platform production persistence.
- `oteryn_platform` and `oteryn_game` are separate logical databases even when hosted on one cluster.
- Each data set has one primary owner; no unrestricted shared-table writes or shared admin credential are allowed.
- Cross-system writes use explicit APIs/contracts; public reads use query APIs or tightly bounded read-only projections.
- Redis is non-authoritative.

## Validation

### Focused / exact-head CI

- workflow: `Agent governance`
- exact PR head: `cb838a759247eb81cdac21c47f0b4376f64294ef`
- run: `30985981623`
- result: `PASS`

### Component/integration

- result: `NOT_APPLICABLE` — documentation-only architecture contracts

### E2E

- result: `NOT_APPLICABLE` — no runtime implementation or deployment was changed

## Independent audit

- reviewed exact head: `cb838a759247eb81cdac21c47f0b4376f64294ef`
- method: adversarial complete-diff review against ADR-0001, live Platform/otclient evidence and owner decisions
- resolved finding: an initial replacement of the long foundation checkpoint removed useful detailed requirements; the original blob was restored before readiness
- open material findings: none
- verdict: `PASS`

## PR and closeout

- PR: `#5`
- changed files: three ADRs plus bounded active task record
- review threads: none
- merge method: squash
- merge commit: `2d591d78fa4986469e4b0cdd87f4fa8a06cbc4a8`
- ownership: released

## Remaining foundation work

This task did not authorize implementation. The existing broad foundation task remains the continuation point for:

1. workspace and dependency contract;
2. `protocol-oteryn` v1 reconciliation and contract;
3. runtime execution contract;
4. exact Identity/Game Session/admission/lease contract;
5. detailed Persistence v1 schema, transactions, backup and recovery;
6. content migration/scripting contract;
7. executable foundation vertical-slice programme;
8. actual client migration and Platform PostgreSQL migration through separate cross-repository tasks.

## Context checkpoint

```yaml
last_progress: PR #5 merged the owner-approved client, Platform/Gateway and PostgreSQL architecture decisions and this task was archived.
status: completed
branch: docs/archive-owner-foundation-decisions-20260805
head_sha: 2d591d78fa4986469e4b0cdd87f4fa8a06cbc4a8
pr: 5
ci_check_generation: 30985981623
ci_checks_for_current_head: 1
terminal_ci_wait_started_at: null
terminal_ci_checks_for_current_generation: 0
unchanged_state_checks: 0
identical_failure_retries: 0
repair_cycles_for_current_gate: 1
stall_warnings: 0
blocker: null
next_action: Resume the broad foundation task with the workspace/dependency and protocol-oteryn reconciliation contracts.
```
