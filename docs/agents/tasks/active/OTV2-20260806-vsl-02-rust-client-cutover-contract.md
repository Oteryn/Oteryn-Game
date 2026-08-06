# OTV2-20260806-vsl-02-rust-client-cutover-contract

```yaml
task_id: OTV2-20260806-vsl-02-rust-client-cutover-contract
title: Define exact Rust client migration and cutover contract
mode: CONTRACT
status: validating
repository: blakinio/Oteryn-v2
base_branch: main
branch: docs/vsl-02-rust-client-cutover-contract
pr: 48
base_sha: 9034bd4bfa491eac6a898b29bc8151c94a4c2b89
head_sha: null
owner: GPT-5.6-Thinking-architecture-coordinator
created_at: 2026-08-06T09:50:00+02:00
updated_at: 2026-08-06T10:42:00+02:00
execution_budget_minutes: 120
large_budget_reason: VSL-02 pins the exact source subtree, reconciles source PRs/tasks/workflows, defines path/provenance/dependency mapping, one atomic destination PR, source freeze/marker sequencing, validation and single-writer rollback across two repositories.
owned_paths:
  - docs/migration/VSL-02_RUST_CLIENT_MIGRATION_AND_CUTOVER_CONTRACT.md
  - docs/migration/VSL-02_SOURCE_RECONCILIATION.md
  - docs/migration/rust-client-path-map.json
  - docs/migration/rust-client-provenance-plan.json
  - docs/agents/tasks/active/OTV2-20260806-vsl-02-rust-client-cutover-contract.md
public_contracts:
  - docs/migration/VSL-02_RUST_CLIENT_MIGRATION_AND_CUTOVER_CONTRACT.md
  - docs/migration/VSL-02_SOURCE_RECONCILIATION.md
  - docs/migration/rust-client-path-map.json
  - docs/migration/rust-client-provenance-plan.json
depends_on:
  - FND-01 accepted at 3e11cf36ffdc1191fabd60c09e8da9818594e189
  - ADR-0002
  - ADR-0008
  - ADR-0011
blocks:
  - the one atomic destination Rust-client migration/workspace PR
  - FND-ID-01 until destination migration and source marker are terminal
  - FND-02
  - FND-03
  - FND-04
cross_repository_coordination_id: OTV2-RUST-CLIENT-CUTOVER-20260806
external_repositories:
  - blakinio/otclient (read-only evidence; later marker PR requires separate owner authorization)
```

## Outcome

Deliver an implementation-ready VSL-02 contract that pins the exact Rust-client source commit and subtree, and defines the complete destination import, transformation, provenance, dependency, validation, rollout, freeze, source-marker and single-writer rollback procedure without moving code.

## Architecture and source of truth

### PROVEN

- Destination base is `9034bd4bfa491eac6a898b29bc8151c94a4c2b89`.
- FND-01 is accepted and archived; the 19-member graph and source dispositions are canonical.
- Selected source commit is `c923ad8a1dff17b4933a6110931b0823cec2c590`.
- Selected Rust subtree tree is `c0928dafca6df19ff11d7901e503ed85a5199439`.
- Source manifest blob is `037013e8e4a762a65f0f2a30f7761ee14725a3fc`; lockfile blob is `2143408c12c50132883890f0821278320a331fde`.
- No open PR changes `oteryn-client/**` at reconciliation.
- Source PRs #23, #48 and #97 are legacy/operational lanes outside the Rust subtree.
- Root source active records are stale, historical or Canary-reference work.
- The nested source native task and its workflow enforce a superseded dual-protocol programme.

### CONTRACT RESULT

- Import always uses the pinned commit, not mutable source `main`.
- Source `main` may advance outside the Rust subtree; current subtree tree equality is the preflight gate.
- All 26 source workspace members and relevant non-members have exact machine dispositions.
- The nested native task is `SUPERSEDED_REFERENCE_ONLY`; its correspondence receives a notice and its obsolete workflow is deleted only by the post-destination source marker.
- One atomic destination PR creates exactly 19 members.
- Tokio `1.51.4` is retained; exact reqwest `0.13.4` with explicit rustls/form/json/stream features replaces blocking `ureq`.
- Production and synthetic harness closures are separate.
- Provenance is machine-readable and does not claim cross-repository ancestry.
- Rollout and rollback keep exactly one writable canonical Rust client; rollback uses a destination non-writable hold before source authority is restored.

### DEFERRED

- Physical import/Cargo workspace belongs to the atomic destination migration task.
- Source README/AGENTS/workflow/task changes belong to a later separately authorized source-marker task.
- Public identifiers belong to FND-ID-01.
- Native protocol/transport belongs to FND-02.
- Game Session/admission belongs to FND-04.

## Acceptance criteria

- [x] Exact destination base, source commit and subtree tree verified.
- [x] Source drift, open PRs, root tasks, nested task and associated workflow reconciled.
- [x] Every source workspace member and relevant non-member has one closed-vocabulary disposition.
- [x] Reference-only technical-login classification has no false destination copy.
- [x] Provenance/history policy is machine-readable and truthful.
- [x] Exact async HTTP/TLS migration dependency policy is fixed.
- [x] Atomic branch/PR contents, equivalence and validation evidence are fixed.
- [x] Freeze, source-marker paths and superseded-workflow closeout are fixed.
- [x] Rollout and rollback prevent zero/dual writable canonical ownership.
- [x] Independent final architecture/migration audit reports zero open material findings.
- [ ] Exact-head Agent governance, Dependency Review and CodeQL pass.
- [ ] PR #48 is marked ready and squash-merged.
- [ ] Task is archived and ownership released.

## Excluded scope

- No Rust code or Cargo workspace creation.
- No write to `blakinio/otclient`.
- No source marker or source freeze implementation yet.
- No native protocol, server runtime, admission, persistence, content or Studio implementation.
- No production deployment or live Platform operation.

## Implementation / findings

Delivered:

- exact source commit/tree reconciliation;
- source PR, root/nested task and obsolete workflow classification;
- complete 26-member path map and non-member exclusions;
- machine provenance, dependency, freeze, source-marker, rollout and rollback plan;
- exact atomic 19-member implementation package;
- disposition-specific equivalence requirements;
- Windows/Linux/harness/security/provenance validation matrices;
- destination-first canonical ownership and single-writer rollback.

Resolved material findings:

1. whole-repository equality was too broad; preflight now checks the Rust subtree tree;
2. nested dual/native task was omitted; it is now superseded reference-only;
3. technical-login reference-only mapping implied a copy; destination path is now empty;
4. direct native-tls removal wording was too broad; transitive occurrences require explanation;
5. rollback could create a dual-writable interval; destination rollback hold is now mandatory;
6. obsolete native correspondence workflow was unclassified; it is reference-only and deleted by the later source marker;
7. workflow disposition temporarily escaped the closed FND-01 vocabulary; it now uses `REFERENCE_ONLY` plus a separate source-marker action.

## Validation

### Focused

- command/run: exact source/destination revision and source-tree inspection; open PR/task/workflow reconciliation; 26-member disposition audit; machine-document structural consistency review; complete five-file changed-path review; adversarial rollout/rollback authority review
- result: `PASS`; no missing member, path conflict, hidden Canary/native placeholder, false ancestry/equivalence claim, release-closure leak or zero/dual-writable ownership interval remains

### Component/integration

- command/run: `NOT_APPLICABLE` — architecture/migration-contract task only
- result: `NOT_APPLICABLE`

### E2E

- scenario: `NOT_APPLICABLE` — no executable migration occurs in this task
- result: `NOT_APPLICABLE`

### Exact-head CI

- candidate architecture head before this task checkpoint: `9433d84cd2a480cbc3f376eedb0b2a30d1ccdeac`
- final task-record head: pending this commit
- workflow/run: pending
- result: pending

## Independent audit

- exact architecture head: `9433d84cd2a480cbc3f376eedb0b2a30d1ccdeac`
- method/auditor: adversarial source-to-target audit against FND-01, ADR-0002, ADR-0008, ADR-0011, selected source tree, nested source programme/workflow, provenance truthfulness, dependency/release closure and single-writer rollback safety
- resolved material findings: subtree boundary, nested task, false test mapping, native-tls wording, rollback hold, obsolete workflow, closed disposition vocabulary
- open material findings: none
- unresolved review threads: 0
- verdict: `PASS`

## PR and closeout

- PR: #48, draft pending exact-head checks
- changed-file review: exactly five declared task/migration files
- unresolved review threads: 0
- related PRs: source #23/#48/#97 classified but not modified; destination #38 unrelated stale lifecycle cleanup
- merge commit/result: pending exact-head validation
- ownership release: pending archive PR

## Context checkpoint

```yaml
last_progress: Final VSL-02 source, mapping, provenance, dependency, source-marker and rollback audit passed with zero open material findings.
status: validating
branch: docs/vsl-02-rust-client-cutover-contract
head_sha_before_checkpoint: 9433d84cd2a480cbc3f376eedb0b2a30d1ccdeac
pr: 48
ci_check_generation: pending final task-record commit
ci_checks_for_current_head: 0
terminal_ci_wait_started_at: null
terminal_ci_checks_for_current_generation: 0
unchanged_state_checks: 0
identical_failure_retries: 0
repair_cycles_for_current_gate: 0
stall_warnings: 0
blocker: exact-head required checks
next_action: Inspect exact-head governance, Dependency Review and CodeQL; mark ready and squash-merge only if all pass.
```
