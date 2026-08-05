# OTV2-20260805-vsl02-atomic-destination-pr

```yaml
task_id: OTV2-20260805-vsl02-atomic-destination-pr
title: Freeze one atomic destination PR for Rust client migration
mode: CONTRACT
status: ready
repository: blakinio/Oteryn-v2
base_branch: main
branch: arch/vsl02-atomic-destination-pr-20260805
pr: 21
base_sha: 03da87a20e4cc5802d4a4887c9e7db5c6e0a3db4
head_sha: 07ea6b45f040ebe47b21408e3a17d0825d061569
owner: architecture-coordinator
created_at: 2026-08-05T15:15:00+02:00
updated_at: 2026-08-05T15:33:00+02:00
execution_budget_minutes: 60
large_budget_reason: null
owned_paths:
  - docs/architecture/ADR-0002-repository-ownership-and-client-migration.md
  - docs/architecture/FOUNDATION_DECISION_BACKLOG.md
  - docs/architecture/GLOBAL_ARCHITECTURE_DECISION_REGISTER.md
  - docs/agents/tasks/active/OTV2-20260805-foundation-preimplementation-contracts.md
  - docs/agents/tasks/active/OTV2-20260805-vsl02-atomic-destination-pr.md
public_contracts:
  - docs/architecture/ADR-0002-repository-ownership-and-client-migration.md
  - docs/architecture/FOUNDATION_DECISION_BACKLOG.md
  - docs/architecture/GLOBAL_ARCHITECTURE_DECISION_REGISTER.md
depends_on:
  - ADR-0002 accepted early VSL-02 sequencing
  - FND-01 remains the first unresolved foundation contract
blocks:
  - final VSL-02 delivery topology until this accepted owner decision is merged
cross_repository_coordination_id: OTV2-NATIVE-FOUNDATION
external_repositories:
  - blakinio/otclient
```

## Outcome

Record the owner-accepted decision that the destination-side Rust client migration, root-workspace creation/completion, accepted FND-01 crate dispositions, `protocol-canary` isolation and destination validation are delivered as one atomic pull request in `blakinio/Oteryn-v2`.

A later coordinated PR in `blakinio/otclient` may only mark the source path moved/non-canonical after the destination merge is immutable and validated; it is not a second destination implementation phase.

## Architecture and source of truth

- `PROVEN`: ADR-0002 makes `blakinio/Oteryn-v2` the canonical repository for client, server and shared Rust contracts.
- `PROVEN`: repository policy requires squash merge for delivery PRs.
- `DERIVED`: squash merge cannot preserve imported cross-repository commit ancestry as destination mainline ancestry.
- `ACCEPTED_OWNER_DECISION`: use one atomic destination PR and preserve history through immutable source retention, exact source SHA/range, machine-readable path/provenance mapping, copyright/license records and source links rather than claiming ancestry continuity.

## Acceptance criteria

- [x] ADR-0002 requires one atomic destination PR and defines the separate source-marker rollout.
- [x] The backlog and global register remove the option for a separate post-migration workspace-bootstrap PR.
- [x] Provenance/history language is compatible with mandatory squash merge and does not claim false Git ancestry preservation.
- [x] The programme checkpoint reflects the atomic destination package.
- [x] No runtime, Cargo workspace, client source or external repository is modified.
- [x] Governance validation passes on the complete architecture head.
- [x] Independent full-diff audit finds no material contradiction.

## Excluded scope

- no execution of FND-01;
- no VSL-02 implementation or source SHA selection;
- no Rust workspace or client code migration;
- no write to `blakinio/otclient`;
- no protocol, runtime, identifier or admission implementation.

## Implementation / findings

- Owner selected one destination PR rather than separate import and workspace-consolidation PRs.
- ADR-0002 now requires the import, FND-01 dispositions, root workspace, dependency enforcement, `protocol-canary` isolation, provenance, validation and rollback evidence on one destination head.
- The source repository remains immutable history/provenance evidence after cutover.
- Mandatory squash merge is reconciled with truthful provenance through exact source SHA/range and machine-readable source-to-destination mapping, not a false ancestry claim.
- The later source-marker PR is cross-repository closeout and cannot carry destination architecture or runtime changes.
- Until that source marker is terminal, a cutover hold prevents two independently developed canonical clients.

## Validation

### Focused

- command/run: full PR #21 diff review against base `03da87a20e4cc5802d4a4887c9e7db5c6e0a3db4`
- result: exactly five declared documentation files changed; temporary workflow removed; no runtime, Cargo or external-repository paths changed

### Component/integration

- command/run: `NOT_APPLICABLE` — architecture-only package
- result: `NOT_APPLICABLE`

### E2E

- scenario: `NOT_APPLICABLE` — no runtime behavior changes
- result: `NOT_APPLICABLE`

### Exact-head CI

- validated architecture head: `07ea6b45f040ebe47b21408e3a17d0825d061569`
- workflow/run: Agent governance `31010433313`
- result: `PASS`
- final task-checkpoint-only head: pending automatic Agent governance

## Independent audit

- exact architecture head: `07ea6b45f040ebe47b21408e3a17d0825d061569`
- method/auditor: independent full architecture diff review
- material findings: none
- verdict: `PASS`

Audit conclusions:

- one destination PR is compatible with the accepted early VSL-02 sequencing;
- source freeze plus the later marker prevents simultaneous active canonical development;
- mandatory squash merge is described truthfully and does not claim imported source ancestry;
- rollback authority remains with VSL-02 and source remains canonical if the destination does not merge;
- no Platform, protocol, runtime, persistence, content, multichannel or Game Intelligence boundary changed;
- no runtime capability or migration completion is claimed.

## PR and closeout

- changed-file review: five declared documentation files; clean
- unresolved review threads: none observed before final checkpoint update
- related/superseded PRs: none known
- merge commit/result: pending final checkpoint-only exact-head governance
- ownership release: pending merge and archive

## Context checkpoint

```yaml
last_progress: Full PR #21 architecture audit passed and Agent governance run 31010433313 passed on the complete architecture head; only the final task-checkpoint commit remains to verify.
status: ready
branch: arch/vsl02-atomic-destination-pr-20260805
head_sha: 07ea6b45f040ebe47b21408e3a17d0825d061569
pr: 21
ci_check_generation: final checkpoint-only commit
ci_checks_for_current_head: 1
terminal_ci_wait_started_at: null
terminal_ci_checks_for_current_generation: 0
unchanged_state_checks: 0
identical_failure_retries: 0
repair_cycles_for_current_gate: 0
stall_warnings: 0
blocker: null
next_action: Verify Agent governance on the final checkpoint-only head and squash-merge PR #21.
```
