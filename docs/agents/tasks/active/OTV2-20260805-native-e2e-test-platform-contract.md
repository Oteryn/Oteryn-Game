# OTV2-20260805-native-e2e-test-platform-contract

```yaml
task_id: OTV2-20260805-native-e2e-test-platform-contract
title: Define the native end-to-end test platform contract
mode: CONTRACT
status: validating
repository: blakinio/Oteryn-v2
base_branch: main
branch: docs/qa-e2e-architecture-20260805
pr: 35
base_sha: d4b5755cd45bfc6689f3614173f7c5701f56bb36
head_sha: pending exact-final validation
owner: ChatGPT architecture coordinator
created_at: 2026-08-05T17:33:00+02:00
updated_at: 2026-08-05T17:56:00+02:00
execution_budget_minutes: 60
large_budget_reason: null
owned_paths:
  - docs/agents/tasks/active/OTV2-20260805-native-e2e-test-platform-contract.md
  - docs/architecture/ADR-0007-native-end-to-end-test-platform.md
  - docs/architecture/FOUNDATION_DECISION_BACKLOG.md
  - docs/architecture/GLOBAL_ARCHITECTURE_DECISION_REGISTER.md
  - docs/agents/BUILD_TEST_MATRIX.md
  - docs/agents/END_TO_END_FEATURE_COMPLETENESS.md
public_contracts:
  - docs/architecture/ADR-0007-native-end-to-end-test-platform.md
  - docs/architecture/FOUNDATION_DECISION_BACKLOG.md
  - docs/architecture/GLOBAL_ARCHITECTURE_DECISION_REGISTER.md
depends_on:
  - ADR-0001
  - ADR-0002
  - ADR-0003
  - ADR-0004
blocks:
  - VSL-01 completion until QA-E2E-01 is implemented and its required evidence exists
cross_repository_coordination_id: OTV2-QA-E2E-01
external_repositories:
  - blakinio/canary
  - blakinio/Oteryn-Platform
```

## Outcome

Persist an accepted architecture contract for a reusable native Oteryn v2 E2E platform that proves the complete client-to-server product path without making graphical client automation the only mechanism for failure, concurrency and recovery coverage.

## Architecture and source of truth

### PROVEN

- `blakinio/canary@f69632104888ece1d9afb801a90a66244694a627` contains a shared physical-client E2E platform, declarative scenario manifests and retained evidence for login, movement, combat, multiclient, NPC, Platform and recovery scenarios.
- Canary's `login/relog` scenario starts a disposable database, an exact controlled server and a real OTClient, then records semantic client markers, SQL assertions, logs, screenshots, session records and runtime hashes.
- Canary retained repeated-run evidence in which nine of ten physical `login/relog` attempts passed and one infrastructure/client-configuration attempt failed; the population was classified as unstable rather than silently retried into success.
- Oteryn v2 already requires named client/server E2E, character lease/relog, crash recovery, multichannel isolation, no-duplication and persistence failure evidence in `docs/agents/BUILD_TEST_MATRIX.md`.

### DERIVED

- A real native client is necessary to prove the supported product path, but using the graphical client as the only driver would make broad concurrency, protocol fault and failure-injection coverage too slow and fragile.
- A shared manifest-driven runner is preferable to feature-owned workflow copies because lifecycle, cleanup, evidence and security controls must remain consistent.

### ACCEPTED

- `QA-E2E-01` is the stable gate for the Native End-to-End Test Platform Contract.
- The platform uses three execution tiers: headless system E2E, instrumented native-client E2E and production-binary smoke E2E.
- `QA-E2E-01` blocks the claim that `VSL-01` is complete, but does not block architecture discovery, `FND-01` or the controlled `VSL-02` client migration.

## Acceptance criteria

- [x] ADR-0007 records the three-tier execution model and its ownership boundaries.
- [x] ADR-0007 defines scenario, evidence, cleanup, deterministic-control, failure-injection and flakiness requirements.
- [x] The build/test matrix names `QA-E2E-01` and distinguishes the three tiers.
- [x] End-to-end feature completeness requires evidence from the appropriate tier and forbids synthetic or environment-only success claims.
- [x] The foundation backlog and global architecture register expose `QA-E2E-01`, its dependencies and its `VSL-01` completion gate.
- [x] No Rust workspace, runtime, client, protocol, Platform or CI implementation is introduced by this task.
- [ ] Governance validation passes on the exact final head.

## Excluded scope

- No test runner, test client, client adapter, workflow, container stack or fixture implementation.
- No changes to `blakinio/canary`, `blakinio/Oteryn-Platform`, `blakinio/otclient` or `blakinio/Otheryn`.
- No final choice of Rust test libraries, container orchestration library, renderer backend or CI runner fleet.
- No production credentials, endpoints, data or assets.

## Implementation / findings

- ADR-0007 accepts the three-tier platform and stable gate `QA-E2E-01`.
- `BUILD_TEST_MATRIX.md` now defines tier purpose, placement, mandatory evidence, high-risk scenarios and population classification.
- `END_TO_END_FEATURE_COMPLETENESS.md` now requires each feature to name its tiers and prevents Tier 1, instrumented-client or environment-only evidence from being overclaimed.
- `FOUNDATION_DECISION_BACKLOG.md` now records the stable gate, accepted direction, implementation order and `VSL-01` start condition.
- `GLOBAL_ARCHITECTURE_DECISION_REGISTER.md` now records ADR-0007 as accepted and prevents `VSL-01` or `ALPHA-QUALITY-01` from creating a competing E2E platform.
- Canary is retained as architecture evidence and a migration/reference source, not as a runtime dependency.
- Direct SQL assertions are restricted to persistence/migration ownership; ordinary gameplay scenarios should prefer stable read-only probes, domain events and audit evidence.
- The instrumented client adapter may observe and submit normal product actions but may not bypass Platform admission, create authoritative sessions, mutate server state or replace server validation.
- Agent governance run `31022445259` proved both repository validators pass and found one PR metadata defect: the body lacked the mandatory `## Summary` heading.
- PR #35 now includes `## Summary`, `## Scope` and `## Validation`; this commit triggers a fresh exact-head validation rather than reusing the failed event payload.

## Validation

### Focused

- command/run: `python tools/agents/validate_governance.py`
- result: PASS inside workflow run `31022445259`
- command/run: `python tools/repository/validate_repository_policy.py`
- result: PASS inside workflow run `31022445259`

### Component/integration

- command/run: `NOT_APPLICABLE` — architecture-only delivery with no executable runtime change
- result: `NOT_APPLICABLE`

### E2E

- scenario: `NOT_APPLICABLE` — this task defines the future E2E platform and changes no executable product path
- result: `NOT_APPLICABLE`

### Exact-head CI

- prior head: `bcb1e7efd4efd401a9f44fad03e07e290a9c72b6`
- Dependency review run `31022446050`: PASS
- Agent governance run `31022445259`: FAIL only at PR metadata validation; repository/governance validators passed
- repair: added the required `## Summary` PR-body heading
- exact final head/workflow: pending after this repair checkpoint

## Independent audit

- audit pass 1: found that ADR-0007 was not yet represented in the foundation backlog and global architecture register.
- remediation: added the stable gate, accepted baseline, ordering and `VSL-01` dependency to both canonical registers.
- audit pass 2: exact compare against `main@d4b5755cd45bfc6689f3614173f7c5701f56bb36` showed only intended architecture additions and narrow policy replacements; no prior decision was removed.
- exact final head: pending after this checkpoint commit
- method/auditor: separate exact-diff architecture, terminology, scope and unsupported-claim review by ChatGPT
- open material findings: none identified before exact-final CI
- verdict: conditional `PASS`, pending exact-final changed-file and CI verification

## PR and closeout

- changed-file review: six declared documentation paths in PR #35; exact-final review pending after this checkpoint commit
- unresolved review threads: zero before this checkpoint; exact-final verification pending
- related/superseded PRs: none identified
- merge commit/result: pending
- ownership release: pending

## Context checkpoint

```yaml
last_progress: PR metadata was repaired after the workflow proved repository governance valid but rejected the missing Summary heading.
status: validating
branch: docs/qa-e2e-architecture-20260805
head_sha: pending after checkpoint commit
pr: 35
ci_check_generation: next exact-head generation
ci_checks_for_current_head: 0
terminal_ci_wait_started_at: null
terminal_ci_checks_for_current_generation: 0
unchanged_state_checks: 0
identical_failure_retries: 0
repair_cycles_for_current_gate: 1
stall_warnings: 0
blocker: null
next_action: Verify the fresh exact-head governance, dependency and CodeQL results and merge only if all required gates pass.
```
