# OTV2-20260805-native-e2e-test-platform-contract

```yaml
task_id: OTV2-20260805-native-e2e-test-platform-contract
title: Define the native end-to-end test platform contract
mode: CONTRACT
status: completed
repository: blakinio/Oteryn-v2
base_branch: main
branch: docs/qa-e2e-architecture-20260805
pr: 35
base_sha: d4b5755cd45bfc6689f3614173f7c5701f56bb36
head_sha: 9b409c544e51a7b822747ede1dcd5554028a62b6
merge_sha: 99db6d5f4348a6199b58a3dae8c48a79e3162258
owner: released
created_at: 2026-08-05T17:33:00+02:00
updated_at: 2026-08-05T17:59:00+02:00
execution_budget_minutes: 60
large_budget_reason: null
owned_paths: []
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

Accepted and merged a reusable native Oteryn v2 E2E architecture that proves the complete client-to-server product path without making graphical client automation the only mechanism for failure, concurrency and recovery coverage.

## Architecture and source of truth

### PROVEN

- `blakinio/canary@f69632104888ece1d9afb801a90a66244694a627` contains the reference shared physical-client E2E platform, declarative scenario manifests and retained evidence for login, movement, combat, multiclient, NPC, Platform and recovery scenarios.
- Canary's `login/relog` scenario starts a disposable database, an exact controlled server and a real OTClient, then records semantic client markers, SQL assertions, logs, screenshots, session records and runtime hashes.
- Canary retained a ten-attempt physical `login/relog` population with nine clean passes and one infrastructure/client-configuration failure and classified it as unstable rather than silently retrying to green.
- Oteryn v2 requires named client/server E2E, character lease/relog, crash recovery, multichannel isolation, no-duplication and persistence-failure evidence.

### ACCEPTED

- `QA-E2E-01` is the stable Native End-to-End Test Platform gate.
- Tier 1 uses a headless production-protocol client for broad deterministic Platform/Gateway/protocol/server/PostgreSQL scenarios.
- Tier 2 uses the real instrumented Rust client for networking, input, reconciliation, UI and rendering evidence without bypassing server authority.
- Tier 3 uses exact production-default release-candidate binaries for a small packaging/product smoke journey.
- One shared versioned scenario and lifecycle platform owns provisioning, deterministic controls, evidence, first-divergence reporting, timeouts and cleanup.
- Hidden retry-until-green is forbidden; every counted attempt and cleanup outcome remains visible.
- `QA-E2E-01` blocks completion of `VSL-01`, but does not block `FND-01`, `VSL-02` or architecture discovery.

## Delivered

- `docs/architecture/ADR-0007-native-end-to-end-test-platform.md` — accepted three-tier architecture, scenario contract, environment ownership, failure injection, evidence envelope, cleanup, flakiness, CI placement and implementation acceptance.
- `docs/architecture/FOUNDATION_DECISION_BACKLOG.md` — stable gate, accepted direction, implementation order and `VSL-01` dependency.
- `docs/architecture/GLOBAL_ARCHITECTURE_DECISION_REGISTER.md` — ADR-0007 accepted foundation, Stage C gate and prohibition on competing E2E platforms.
- `docs/agents/BUILD_TEST_MATRIX.md` — tier purposes, placement, mandatory evidence, high-risk scenarios and repeated-run classifications.
- `docs/agents/END_TO_END_FEATURE_COMPLETENESS.md` — tier-aware completeness and explicit rejection of synthetic/environment-only overclaims.

Canary remains architecture evidence and a migration/reference source, not an Oteryn v2 runtime dependency.

## Acceptance criteria

- [x] ADR-0007 records the three-tier execution model and ownership boundaries.
- [x] ADR-0007 defines scenario, evidence, cleanup, deterministic-control, failure-injection and flakiness requirements.
- [x] The build/test matrix names `QA-E2E-01` and distinguishes all three tiers.
- [x] End-to-end feature completeness requires evidence from the appropriate tier and forbids synthetic or environment-only success claims.
- [x] The foundation backlog and global architecture register expose `QA-E2E-01`, its dependencies and its `VSL-01` completion gate.
- [x] No Rust workspace, runtime, client, protocol, Platform or CI implementation was introduced.
- [x] Exact-final governance, Dependency review and CodeQL passed.
- [x] PR #35 was squash-merged and ownership released.

## Excluded scope

- No test runner, test client, client adapter, workflow, container stack or fixture implementation.
- No writes to `blakinio/canary`, `blakinio/Oteryn-Platform`, `blakinio/otclient` or `blakinio/Otheryn`.
- No final choice of Rust testing libraries, container orchestration library, renderer backend or CI runner fleet.
- No production credentials, endpoints, data, assets or deployment action.

## Validation

### Focused

- `python tools/agents/validate_governance.py`: PASS in exact-final Agent governance run `31022789439`.
- `python tools/repository/validate_repository_policy.py`: PASS in exact-final Agent governance run `31022789439`.
- Pull-request title/body metadata: PASS after adding the mandatory `## Summary` heading.

### Component/integration

- `NOT_APPLICABLE` — architecture-only delivery with no executable runtime change.

### E2E

- `NOT_APPLICABLE` — the delivery defines the future E2E platform and changes no executable product path.

### Exact-head CI

- exact feature head: `9b409c544e51a7b822747ede1dcd5554028a62b6`
- Agent governance run `31022789439`: PASS
- Dependency review run `31022789215`: PASS
- CodeQL run `31022790016`: PASS

## Independent audit

- Pass 1 found that ADR-0007 was not yet represented in the foundation backlog and global architecture register.
- The stable gate, accepted baseline, ordering and `VSL-01` dependency were added to both canonical registers.
- Pass 2 compared the exact feature head against `main@d4b5755cd45bfc6689f3614173f7c5701f56bb36` and found only six declared documentation paths, no prior architecture decision removal and no runtime change.
- Final review-thread count: zero.
- Verdict: PASS.

## PR and closeout

- feature PR: `#35` — `docs(architecture): define native E2E test platform`
- feature head: `9b409c544e51a7b822747ede1dcd5554028a62b6`
- squash merge: `99db6d5f4348a6199b58a3dae8c48a79e3162258`
- merged at: 2026-08-05T15:56:28Z
- changed paths: six declared documentation paths
- unresolved review threads: zero
- ownership: released by this lifecycle package

## Context checkpoint

```yaml
last_progress: QA-E2E-01 architecture merged through PR #35 and all exact-final workflow runs passed.
status: completed
branch: docs/archive-qa-e2e-architecture-20260805
head_sha: pending lifecycle PR validation
pr: pending lifecycle PR
ci_check_generation: lifecycle exact-head
ci_checks_for_current_head: 0
terminal_ci_wait_started_at: null
terminal_ci_checks_for_current_generation: 0
unchanged_state_checks: 0
identical_failure_retries: 0
repair_cycles_for_current_gate: 1
stall_warnings: 0
blocker: null
next_action: Continue the global architecture programme with FND-01; QA-E2E-01 implementation begins incrementally only after its client/protocol/admission/persistence/content prerequisites exist.
```
