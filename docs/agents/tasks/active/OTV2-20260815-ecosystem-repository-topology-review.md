# OTV2-20260815-ecosystem-repository-topology-review

```yaml
task_id: OTV2-20260815-ecosystem-repository-topology-review
title: Review target Oteryn ecosystem repository topology
mode: AUDIT
status: validating
repository: blakinio/Oteryn-v2
base_branch: main
branch: docs-ecosystem-repository-topology-review-20260815
pr: 278
base_sha: cb98fd32a2bb71fce83234ebf8bf69bdd1a1970e
head_sha: null
final_head_sha: null
final_head_frozen_at: null
owner: architecture-review-agent
created_at: 2026-08-15T13:49:00+02:00
updated_at: 2026-08-15T13:59:10+02:00
execution_budget_minutes: 120
large_budget_reason: Cross-repository topology review against accepted native client/server/protocol/world/Studio architecture and current repository governance; documentation-only, no implementation authority.
owned_paths:
  - docs/agents/tasks/active/OTV2-20260815-ecosystem-repository-topology-review.md
  - docs/architecture/reviews/OTERYN_ECOSYSTEM_REPOSITORY_TOPOLOGY_REVIEW_2026-08-15.md
public_contracts: []
depends_on:
  - accepted ADR-0001, ADR-0002, ADR-0003, ADR-0005, ADR-0007, ADR-0008 and applicable successor contracts
  - docs/architecture/FOUNDATION_PROGRAMME_CURRENT_STATUS.md
  - docs/architecture/GLOBAL_ARCHITECTURE_DECISION_REGISTER.md
  - docs/architecture/DUR-04_CONTENT_WORLD_AND_SCRIPTING_CONTRACT.md
  - docs/agents/CROSS_REPO_CONTRACTS.md
blocks: []
cross_repository_coordination_id: OTV2-ECOSYSTEM-REPOSITORY-TOPOLOGY-20260815
external_repositories:
  - blakinio/Oteryn-Platform
  - blakinio/Otheryn
  - blakinio/otclient
implementation_authorized: false
production_authorized: false
external_repository_writes_authorized: false
```

## Outcome

Produce one critical, evidence-backed architecture review of the proposed future repository topology from the canonical `blakinio/Oteryn-v2` perspective. The review may recommend later cross-repository ADRs, repository naming and ownership changes, but it does not itself rename repositories, move code, create repositories, alter runtime/production behavior, or supersede accepted ADRs.

## Acceptance criteria

- [x] Verify current `main`, governance, active task ownership, open PR state and relevant accepted architecture before conclusions.
- [x] Evaluate the proposed `Oteryn` meta repository, future `Oteryn-Game`, `Oteryn-Platform` and proposed `Oteryn-Atlas` boundaries critically rather than assuming Platform-side acceptance is Oteryn-v2 authority.
- [x] Decide whether Client + Server + `protocol-oteryn` + canonical World/Content + compiler/validation + legacy import boundary + Studio should remain together.
- [x] Assign exact target owners for OTBM parser, legacy IR, canonical world schema, compiler, validation, Studio and Atlas export.
- [x] Define the required future Game-to-Atlas contract shape, compatibility/security/rollback rules and cross-repository decision needs.
- [x] Classify current ADRs/contracts as compatible, conflicting, or requiring later supersession/coordination.
- [x] Assess CI, release, compatibility, schema evolution, agent ownership, CODEOWNERS and future extraction risk.
- [x] Persist only the review document and this task record.

## Excluded scope

- Repository rename, creation, deletion, transfer or code movement.
- Git submodules or monorepo conversion.
- Runtime/client/server/protocol/world/compiler/Studio implementation.
- Platform, Atlas, Otheryn or otclient writes.
- CI/CD, production, deployment, database, secrets or release changes.
- Changing accepted ADR/contract semantics in this delivery.

## Validation plan

- Full changed-file/diff inspection against exact PR head.
- Documentation/architecture focused validation through the repository's GitHub Actions governance and merge gates.
- Runtime/component/E2E: `NOT_APPLICABLE` because the change is documentation-only and changes no executable contract or implementation.
- Exact-head self-review before merge eligibility.

## Validation status

- Review document written at `docs/architecture/reviews/OTERYN_ECOSYSTEM_REPOSITORY_TOPOLOGY_REVIEW_2026-08-15.md`.
- PR #278 opened as draft to establish immutable PR identity before final exact-head validation.
- Full-diff self-review and exact-head GitHub Actions evidence are pending this final task-metadata commit.
- Independent review is not required by default for this delivery because it is a non-authoritative architecture audit/recommendation that changes no accepted ADR/contract, governance rule, protocol, runtime, data, security control or production path. Any later ADR accepting the proposed cross-repository topology remains separately gated.

## Context checkpoint

```yaml
last_progress: Review document completed and PR #278 opened; task metadata now records the PR before final exact-head full-diff self-review and GitHub Actions validation.
status: validating
branch: docs-ecosystem-repository-topology-review-20260815
head_sha: null
pr: 278
final_head_sha: null
final_head_frozen_at: null
ci_trigger_source: pull_request
ci_check_generation: pending_final_head
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
owner_action_required: false
blocker: null
next_action: Freeze the exact PR head, inspect the complete two-path diff, record self-review evidence, mark PR ready, then accept merge only if required exact-head checks and thread state are clean.
```
