# OTV2-20260806-fnd-01-workspace-migration-contract

```yaml
task_id: OTV2-20260806-fnd-01-workspace-migration-contract
title: Define the canonical Rust workspace and existing-client migration dispositions
mode: CONTRACT
status: validating
repository: blakinio/Oteryn-v2
base_branch: main
branch: docs/fnd-01-workspace-migration-contract
pr: 46
base_sha: cbc2150024d98bbdbfa9b1c17bc9b9df16bcd9f2
head_sha: e8fdfd05fe232bf77fae147c000edbb92b5c5d39
owner: GPT-5.6-Thinking-architecture-coordinator
created_at: 2026-08-06T08:13:00+02:00
updated_at: 2026-08-06T09:41:00+02:00
execution_budget_minutes: 120
large_budget_reason: FND-01 requires an exact source-SHA inventory, per-member migration dispositions, target workspace membership, dependency policy, toolchain/CI policy and reconciliation of protocol-free migration constraints.
owned_paths:
  - docs/architecture/FND-01_WORKSPACE_AND_RUST_MIGRATION_CONTRACT.md
  - docs/architecture/FND-01_RUST_SOURCE_INVENTORY.md
  - docs/architecture/FND-01_OWNER_ACCEPTANCE_AND_CRATE_FITNESS_REVIEW.md
  - docs/agents/tasks/active/OTV2-20260806-fnd-01-workspace-migration-contract.md
public_contracts:
  - docs/architecture/FND-01_WORKSPACE_AND_RUST_MIGRATION_CONTRACT.md
  - docs/architecture/FND-01_RUST_SOURCE_INVENTORY.md
  - docs/architecture/FND-01_OWNER_ACCEPTANCE_AND_CRATE_FITNESS_REVIEW.md
depends_on:
  - ADR-0001
  - ADR-0002
  - ADR-0005
  - ADR-0007
  - ADR-0008
  - ADR-0011
  - docs/architecture/FOUNDATION_DECISION_BACKLOG.md
blocks:
  - VSL-02 exact client migration and cutover contract until this package is merged
cross_repository_coordination_id: OTV2-NATIVE-FOUNDATION
external_repositories:
  - blakinio/otclient (read-only source evidence)
```

## Outcome

Deliver and owner-accept the `FND-01` workspace, dependency and existing-Rust migration contract against the exact current Rust-client source revision. The package remains architecture-only and does not create a Cargo workspace or move code.

## Architecture and source of truth

### PROVEN

- Destination baseline: `blakinio/Oteryn-v2@cbc2150024d98bbdbfa9b1c17bc9b9df16bcd9f2`.
- Source inventory revision: `blakinio/otclient@c923ad8a1dff17b4933a6110931b0823cec2c590`.
- The source workspace contains 26 members and direct Canary dependencies in the application and technical-login tests.
- Source account/directory/entry concerns are over-fragmented; source domain/simulation are client projection contracts; source assets are synthetic fixtures.
- The exact contract blob accepted by the owner is `b0127a91f201d4cba766053dc74517fe5cb49268`.

### ACCEPTED

- The initial destination workspace contains 19 named, immediately consumed members.
- Production `oteryn-client` and the non-release synthetic harness have separate dependency and packaging closures.
- `protocol-canary`, current `protocol-core`, gameplay `transport` and `game-session` do not enter the initial workspace.
- Platform/Identity I/O is asynchronous, cancellation-safe and application-runtime owned.
- Gameplay routes, credentials and admission state are absent before `FND-04`.
- Provisional source identifiers are renamed before `FND-ID-01`.
- A mandatory crate fitness review follows the first complete native vertical slice and precedes `VSL-01` completion.

### DEFERRED

- Exact async HTTP/TLS dependency selection belongs to `VSL-02` under the accepted boundary.
- Public identifiers belong to `FND-ID-01`.
- Native protocol/transport belongs to `FND-02`.
- Game Session/admission belongs to `FND-04`.

## Acceptance criteria

- [x] Every source workspace member and relevant non-member subsystem is inventoried at the exact source SHA.
- [x] Every source member has exactly one migration disposition.
- [x] The 19-member graph is acyclic and every member has an immediate consumer.
- [x] Production and synthetic executable closures are separate.
- [x] Canary, source protocol helpers, gameplay transport and Game Session are excluded from the initial graph.
- [x] Client projection/simulation is not presented as authoritative server state.
- [x] Synthetic assets are explicitly non-release fixture infrastructure.
- [x] Gameplay routes and credentials cannot enter pre-native Platform contracts.
- [x] Platform/Identity I/O is asynchronous and cancellation-safe; source `ureq` is reference-only.
- [x] Category and package dependency allowlists, target roles and CI matrices are complete.
- [x] Provisional identifier-name collisions are prevented.
- [x] Independent architecture audit reports zero open material findings.
- [x] Product owner accepted the graph and dispositions on 2026-08-06.
- [x] Mandatory post-slice crate fitness review is recorded.
- [ ] Branch is synchronized with the current `main` if required.
- [ ] Exact-head governance, Dependency Review and CodeQL pass after owner-acceptance commits.
- [ ] PR #46 is marked ready and squash-merged.
- [ ] Task is archived and ownership released.

## Excluded scope

- No physical client migration or Cargo workspace creation.
- No source-repository write, freeze or moved marker.
- No `protocol-oteryn` implementation.
- No server runtime, persistence, admission, content or Studio implementation.
- No final identifier representations.

## Implementation / findings

Delivered:

- exact 26-member source graph and dependency/consumer inventory;
- one disposition per source member and relevant non-member subsystem;
- accepted 19-member destination graph;
- production/pre-native and synthetic-harness separation;
- identifier, Platform/Identity, dependency, release-role, target, CI and machine-enforcement policies;
- owner acceptance bound to the exact audited contract blob;
- mandatory crate fitness review after the first complete native vertical slice.

Resolved material findings include candidate/accepted ambiguity, missing source-edge evidence, synthetic renderer leakage, production-app synthetic dependencies, client/server authority ambiguity, provisional identifier collisions, pre-native route exposure, blocking Platform I/O, incomplete test/tool allowlists and the synthetic asset/compiler dev-cycle pattern.

## Validation

### Focused

- command/run: source-to-target inventory audit, member/disposition counts, manual dependency topological sort, production/harness closure review and complete changed-file review
- result: `PASS`

### Component/integration

- command/run: `NOT_APPLICABLE` — architecture-only contract
- result: `NOT_APPLICABLE`

### E2E

- scenario: `NOT_APPLICABLE` — no executable product change
- result: `NOT_APPLICABLE`

### Exact-head CI

- previous validated head: `b36cea417602ea552b3dfbbec410cc067661cea7`
- Agent governance run `31078148358`: `PASS`
- Dependency review run `31078148528`: `PASS`
- CodeQL run `31078148473`: `PASS`
- owner-acceptance head: pending final check generation

## Independent audit

- exact accepted contract blob: `b0127a91f201d4cba766053dc74517fe5cb49268`
- method/auditor: adversarial source-to-target review against governing ADRs, exact source graph and later `FND-ID-01`, `FND-02` and `FND-04` ownership
- open material findings: none
- verdict: `PASS`

## PR and closeout

- changed-file review: four declared documentation/task files only
- unresolved review threads: 0 at pre-acceptance checkpoint
- related source PRs #23, #48 and #97 remain `VSL-02` reconciliation inputs
- merge commit/result: pending exact-head validation
- ownership release: pending archive PR

## Context checkpoint

```yaml
last_progress: Product owner accepted the exact FND-01 contract and mandatory post-slice crate fitness review; acceptance is recorded on PR #46.
status: validating
branch: docs/fnd-01-workspace-migration-contract
head_sha_before_checkpoint: e8fdfd05fe232bf77fae147c000edbb92b5c5d39
pr: 46
ci_check_generation: pending owner-acceptance task commit
ci_checks_for_current_head: 0
terminal_ci_wait_started_at: null
terminal_ci_checks_for_current_generation: 0
unchanged_state_checks: 0
identical_failure_retries: 0
repair_cycles_for_current_gate: 0
stall_warnings: 0
blocker: exact-head checks and branch-current verification
next_action: Verify branch currency, update PR metadata, mark ready after checks pass and squash-merge.
```
