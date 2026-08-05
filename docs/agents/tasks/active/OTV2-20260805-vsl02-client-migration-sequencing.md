# OTV2-20260805-vsl02-client-migration-sequencing

```yaml
task_id: OTV2-20260805-vsl02-client-migration-sequencing
title: Move exact Rust client migration before identifier and protocol freeze
mode: CONTRACT
status: ready
repository: blakinio/Oteryn-v2
base_branch: main
branch: arch/vsl02-client-migration-sequencing-20260805
pr: 19
base_sha: 14132133493a7e0990235c45fed63efa47233d9e
head_sha: 8b1212f6f84f8aea3cbb62f305ae1e970c81e003
owner: architecture-coordinator
created_at: 2026-08-05T14:56:00+02:00
updated_at: 2026-08-05T15:13:00+02:00
execution_budget_minutes: 60
large_budget_reason: null
owned_paths:
  - docs/architecture/ADR-0002-repository-ownership-and-client-migration.md
  - docs/architecture/FOUNDATION_DECISION_BACKLOG.md
  - docs/architecture/GLOBAL_ARCHITECTURE_DECISION_REGISTER.md
  - docs/architecture/MULTICHANNEL_SYSTEM_SCOPE_MATRIX.md
  - docs/agents/tasks/active/OTV2-20260805-foundation-preimplementation-contracts.md
  - docs/agents/tasks/active/OTV2-20260805-vsl02-client-migration-sequencing.md
public_contracts:
  - docs/architecture/ADR-0002-repository-ownership-and-client-migration.md
  - docs/architecture/FOUNDATION_DECISION_BACKLOG.md
  - docs/architecture/GLOBAL_ARCHITECTURE_DECISION_REGISTER.md
depends_on:
  - FND-01 remains the first unresolved foundation contract
  - ADR-0002 accepted repository ownership and client migration direction
blocks:
  - FND-ID-01 and FND-02 sequencing until this package is merged
cross_repository_coordination_id: OTV2-NATIVE-FOUNDATION
external_repositories:
  - blakinio/otclient
```

## Outcome

Record the owner-accepted architecture decision that `VSL-02` and the controlled Rust client migration/cutover must occur immediately after `FND-01` and before `FND-ID-01`, `FND-02`, `FND-03` or `FND-04` freeze shared client/server contracts in a destination workspace that does not yet contain the canonical client.

Also remove the stale statement that final database technology remains open, because ADR-0004 already selected PostgreSQL.

## Architecture and source of truth

- `PROVEN`: ADR-0002 makes `blakinio/Oteryn-v2` the canonical repository for the Rust client, server and shared protocol/domain contracts.
- `PROVEN`: the existing client workspace remains under `blakinio/otclient/oteryn-client` and contains substantial reusable Rust architecture.
- `PROVEN`: the accepted programme previously placed `VSL-02` after protocol, runtime, admission, persistence and content contracts.
- `DERIVED`: leaving the migration that late risks designing shared contracts against a stale client inventory and reintroduces cross-repository drift that ADR-0002 was accepted to eliminate.
- `ACCEPTED_OWNER_DECISION`: execute `FND-01`, then the separately authorized `VSL-02` migration/cutover programme, then create/complete the canonical workspace bootstrap and continue with `FND-ID-01` and layer contracts.

## Acceptance criteria

- [x] ADR-0002 explicitly records the mandatory sequencing and stale-inventory protection.
- [x] Foundation backlog and global register place `VSL-02` immediately after `FND-01`.
- [x] The active programme checkpoint has exactly one next action and reflects the revised order.
- [x] The multichannel matrix no longer lists database technology as unresolved.
- [x] No runtime, Cargo workspace, client code or external repository is modified.
- [x] Governance validation passes for the complete architecture diff on head `8b1212f6f84f8aea3cbb62f305ae1e970c81e003` in run `31008828453`.
- [x] Independent full-diff audit finds no material contradiction.

## Excluded scope

- no Rust workspace bootstrap;
- no client source migration;
- no writes to `blakinio/otclient`;
- no protocol, runtime, identifier or admission implementation;
- no decision on the exact migration mechanism or source SHA beyond requiring those to be frozen by `VSL-02`.

## Implementation / findings

- Owner accepted moving `VSL-02` immediately after `FND-01`.
- ADR-0002 now makes the sequence normative and rejects an isolated placeholder destination client.
- The backlog and global register promote `VSL-02` from a late vertical-slice gate to a foundation bootstrap/cutover gate.
- The programme checkpoint now requires FND-01 to terminate into VSL-02.
- The migration remains a separate cross-repository programme with one task/branch/PR per written repository.
- Workspace bootstrap may be combined with destination migration only if `VSL-02` explicitly owns and validates that operation; otherwise it follows immediately after migration.
- The multichannel matrix now treats PostgreSQL as accepted while leaving schema, isolation, locking, partitioning, migration and messaging details open.

## Validation

### Focused

- command/run: full PR #19 diff review against base `14132133493a7e0990235c45fed63efa47233d9e`
- result: six declared documentation files changed; no runtime/code paths changed

### Component/integration

- command/run: `NOT_APPLICABLE` — architecture-only package
- result: `NOT_APPLICABLE`

### E2E

- scenario: `NOT_APPLICABLE` — no runtime behavior changes
- result: `NOT_APPLICABLE`

### Exact-head CI

- validated architecture head: `8b1212f6f84f8aea3cbb62f305ae1e970c81e003`
- workflow/run: Agent governance `31008828453`
- result: `PASS`
- final task-checkpoint-only head: pending automatic governance verification

## Independent audit

- exact head: `8b1212f6f84f8aea3cbb62f305ae1e970c81e003`
- method/auditor: independent full architecture diff review
- material findings: none
- verdict: `PASS`

Audit conclusions:

- sequencing is consistent across ADR-0002, the ordered backlog, global register and programme checkpoint;
- FND-01 remains the immediate next architecture package;
- VSL-02 is now its mandatory terminal handoff;
- no Platform, database, multichannel, protocol, runtime, persistence, content or Game Intelligence authority boundary was weakened;
- no speculative crate, workspace, runtime or external-repository write was introduced.

## PR and closeout

- changed-file review: six declared documentation files; clean
- unresolved review threads: none observed before final checkpoint update
- related/superseded PRs: none known
- merge commit/result: pending final checkpoint-only exact-head governance
- ownership release: pending merge and archive

## Context checkpoint

```yaml
last_progress: Full PR #19 architecture audit passed and Agent governance run 31008828453 passed on the complete architecture head; only the final task-checkpoint commit remains to verify.
status: ready
branch: arch/vsl02-client-migration-sequencing-20260805
head_sha: 8b1212f6f84f8aea3cbb62f305ae1e970c81e003
pr: 19
ci_check_generation: final checkpoint-only commit
ci_checks_for_current_head: 1
terminal_ci_wait_started_at: null
terminal_ci_checks_for_current_generation: 0
unchanged_state_checks: 0
identical_failure_retries: 0
repair_cycles_for_current_gate: 0
stall_warnings: 0
blocker: null
next_action: Verify Agent governance on the final checkpoint-only head and squash-merge PR #19.
```
