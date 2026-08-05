# OTV2-20260805-foundation-contract-hardening

```yaml
task_id: OTV2-20260805-foundation-contract-hardening
title: Harden foundation ordering, migration evidence and cross-repository contracts
mode: CONTRACT
status: ready
repository: blakinio/Oteryn-v2
base_branch: main
branch: docs/foundation-contract-hardening-20260805
pr: 15
base_sha: 5c074ad76c52168efede31824d7f28f482f844ce
head_sha: 2d35599e4465d70c17bcb2877503499b8c6f2717
owner: architecture coordinator
created_at: 2026-08-05T13:15:00+02:00
updated_at: 2026-08-05T13:49:00+02:00
execution_budget_minutes: 60
large_budget_reason: null
owned_paths:
  - docs/architecture/FOUNDATION_DECISION_BACKLOG.md
  - docs/architecture/GLOBAL_ARCHITECTURE_DECISION_REGISTER.md
  - docs/agents/tasks/active/OTV2-20260805-foundation-preimplementation-contracts.md
  - docs/agents/prompts/OTV2_GLOBAL_ARCHITECTURE_DECISION_COORDINATOR.md
  - docs/agents/CROSS_REPO_CONTRACTS.md
  - docs/agents/BUILD_TEST_MATRIX.md
  - docs/agents/REPOSITORY_MAP.md
  - docs/contracts/CROSS_REPOSITORY_CONTRACT_LOCK.json
  - docs/contracts/RESOURCE_LIMITS_REGISTRY.json
  - docs/contracts/FOUNDATION_ERROR_VOCABULARY.md
  - docs/contracts/FOUNDATION_FAILURE_SCENARIOS.md
  - docs/agents/tasks/active/OTV2-20260805-foundation-contract-hardening.md
  - docs/agents/GOVERNANCE_CONTRACT.json
  - tools/agents/validate_governance.py
public_contracts:
  - docs/architecture/FOUNDATION_DECISION_BACKLOG.md
  - docs/architecture/GLOBAL_ARCHITECTURE_DECISION_REGISTER.md
  - docs/contracts/CROSS_REPOSITORY_CONTRACT_LOCK.json
  - docs/contracts/RESOURCE_LIMITS_REGISTRY.json
  - docs/contracts/FOUNDATION_ERROR_VOCABULARY.md
  - docs/contracts/FOUNDATION_FAILURE_SCENARIOS.md
depends_on:
  - ADR-0001 through ADR-0005
  - merged foundation execution guardrails PRs 13 and 14
blocks:
  - FND-01 launch until this package is merged and archived
cross_repository_coordination_id: OTV2-NATIVE-FOUNDATION
external_repositories:
  - blakinio/Oteryn-Platform
  - blakinio/Otheryn
  - blakinio/otclient
```

## Outcome

Persist the owner-approved architecture hardening discovered during the final pre-FND-01 review: move the minimum identifier vocabulary before wire/session freezing, make FND-01 an evidence-backed migration contract for the existing Rust client workspace, require machine-readable dependency and cross-repository locks, and add durable limit, error and failure-scenario registries without implementing runtime behavior.

## Architecture and source of truth

### PROVEN

- The canonical Oteryn v2 repository has no root Cargo workspace at base SHA `5c074ad76c52168efede31824d7f28f482f844ce`.
- `blakinio/otclient/oteryn-client` already contains merged Tokio transport, deterministic simulation/snapshot, architecture checks and other reusable Rust contracts that FND-01 must classify at an exact source SHA.
- Platform PR #540 merged as `c0b8703d326a04b43ae8e06f6192b0cb91c859b7`; canonical contract revision `2`, schema revision `2` and schema SHA-256 `9c67f19525400fb9890d2a3541ceb6d02eb955061540ad39ca1c1d891c06eba9` are now locked without claiming Oteryn-v2 FND-02 acceptance or runtime completion.
- The previous order placed the full durable identifier contract after protocol and admission contracts even though those contracts require stable identifier meanings first.

### DERIVED

- Freezing wire/session schemas before a minimum identifier vocabulary risks later incompatible wire and database changes.
- Designing FND-01 without an exact migration inventory risks duplicate crates, duplicate domain ownership and loss of already validated Rust behavior.

## Acceptance criteria

- [x] Introduce stable gate `FND-ID-01` before `FND-02` and `FND-04`, while retaining `DUR-01` for full durable/database identity representation.
- [x] Expand FND-01 to require exact existing-Rust migration classification and evidence, Cargo/toolchain/feature/target policy, and a machine-readable dependency-boundary model.
- [x] Require a machine-readable cross-repository contract lock that accepts only merged canonical revisions and leaves unresolved revisions non-canonical.
- [x] Lock the merged Platform contract correction at its exact merge commit, contract/schema revisions and canonical schema digest without claiming implementation or FND-02 acceptance.
- [x] Add an initial machine-readable resource-limit registry and normative foundation error/failure scenario catalogues without inventing runtime values.
- [x] Add clock semantics, a bounded world-format spike and exact client cutover requirements to the correct later gates.
- [x] Reconcile backlog, global register, programme checkpoint, coordinator prompt and agent cross-repository/build/repository maps.
- [x] Retain the new contracts through permanent governance inputs and semantic validation.
- [x] Governance validation passes on the exact audited content head.
- [x] Independent full-diff audit reports zero open material findings.

## Excluded scope

- No root Cargo workspace, crate, protocol codec, runtime, database schema, client source migration or production behavior.
- No writes to external repositories.
- No acceptance of a mutable PR head as a canonical revision.
- No claim that the disabled Platform producer, Rust server or Rust client consumers are migrated or implemented.
- No final UUID/integer, tick frequency, queue size, chunk size or contract-owned numeric error-code selection.

## Implementation / findings

- Task, dedicated branch and draft PR #15 were created from exact current `main` with no overlapping active package or open PR.
- The ordered programme now separates `FND-ID-01` semantic identity from `DUR-01` durable/database representation.
- FND-01 now requires a pinned existing-Rust inventory and one explicit migration disposition per crate/subsystem: migrate as-is, migrate/rename, merge, split, rewrite, reference-only or drop.
- Cargo/toolchain, target/feature matrix, machine-readable workspace boundaries, cross-repository locking, resource limits, stable error categories and failure scenarios are mandatory retained inputs.
- Platform PR #540 changed from open to merged during execution; the contract lock was refreshed from live state rather than preserving a stale pending entry.

## Audit repairs

- `OTV2-FND-HARDEN-001`: stale programme wording still assigned wire-identity freezing to `DUR-01`; repaired by making `FND-ID-01` the wire/session semantic gate and renaming `DUR-01` to the durable representation contract.
- `OTV2-FND-HARDEN-002`: new JSON registries initially had only parse-time validation; repaired by adding them to permanent governance inputs and validating lock state, SHA/revision grammar, required limit fields, uniqueness and hard maxima.
- `OTV2-FND-HARDEN-003`: temporary validation generated Python bytecode in the branch; removed, with no generated or temporary workflow path remaining.
- `OTV2-FND-HARDEN-004`: external Platform contract state changed during the audit; refreshed to merged commit `c0b8703d326a04b43ae8e06f6192b0cb91c859b7` and immutable schema identity.

## Validation

### Focused

- core architecture application run `31001676593`, job `92291795974`: governance, JSON parsing and diff hygiene `PASS`;
- audit-repair run `31002153221`, attempt `2`, job `92293869768`: repaired governance, Python syntax and diff hygiene `PASS`;
- permanent semantic governance validator on content head `2d35599e4465d70c17bcb2877503499b8c6f2717`: `PASS`.

### Component/integration

- result: `NOT_APPLICABLE` — documentation, machine-readable architecture contracts and their governance validator only.

### E2E

- result: `NOT_APPLICABLE` — no executable runtime behavior changed.

### Exact-head CI

- audited content head: `2d35599e4465d70c17bcb2877503499b8c6f2717`;
- Agent governance run `31002575466`: `PASS`;
- final task-checkpoint head requires the standard exact-head Agent governance gate before merge.

## Independent audit

- audited content head: `2d35599e4465d70c17bcb2877503499b8c6f2717`;
- method: adversarial full-diff review of ordering, authority, mutable-versus-merged external revisions, migration ownership, validator behavior, generated files and scope boundaries;
- critical findings: `0`;
- high findings: `0`;
- open material-medium findings after repair: `0`;
- verdict: `PASS`.

## PR and closeout

- changed-file review: `PASS` — intended architecture, governance, contract and validator paths only;
- temporary workflows/generated bytecode remaining: `0`;
- unresolved review threads/comments/requested changes: `0`;
- related/superseded PRs: none;
- implementation merge: pending protected squash merge of PR #15;
- ownership release: pending separate lifecycle archive after merge.

## Context checkpoint

```yaml
last_progress: Final architecture content and merged Platform contract lock are audited with zero open material findings; exact content-head Agent governance passed.
status: ready
branch: docs/foundation-contract-hardening-20260805
head_sha: 2d35599e4465d70c17bcb2877503499b8c6f2717
pr: 15
ci_check_generation: 31002575466
ci_checks_for_current_head: 1
terminal_ci_wait_started_at: null
terminal_ci_checks_for_current_generation: 1
unchanged_state_checks: 0
identical_failure_retries: 0
repair_cycles_for_current_gate: 2
stall_warnings: 0
blocker: null
next_action: Require Agent governance on this final task-checkpoint head, mark PR #15 ready for review, then squash-merge and archive the task in a separate lifecycle PR.
```
