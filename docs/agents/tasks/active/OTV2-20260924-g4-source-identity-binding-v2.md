# OTV2-20260924-g4-source-identity-binding-v2

```yaml
task_id: OTV2-20260924-g4-source-identity-binding-v2
title: G4 v2 source identity binding carrier
mode: IMPLEMENT
status: implementing
repository: Oteryn/Oteryn-Game
base_branch: main
branch: agent/otv2-g4-source-identity-binding-v2
pr: null
base_sha: 3cbcc87a00ebe377f8275161df1e91cd73401831
head_sha: null
final_head_sha: null
final_head_frozen_at: null
owner: single allocated G4 source identity binding writer
created_at: 2026-09-24
updated_at: 2026-09-24
execution_policy: continuous_progress
owned_paths:
  - apps/game-server/src/content/project/v2.rs
  - apps/game-server/tests/content_world_project_v2.rs
  - docs/agents/tasks/active/OTV2-20260924-g4-source-identity-binding-v2.md
  - docs/agents/evidence/OTV2-20260924-g4-source-identity-binding-v2.json
public_contracts:
  - docs/architecture/OTERYN_G4_MULTI_SOURCE_IDENTITY_BINDING_DECISION.md
depends_on:
  - "Merged #839 Wiki-first census"
  - "Merged #840 G4 identity decision"
blocks: []
cross_repository_coordination_id: null
external_repositories: []
```

## Outcome

Add the minimum typed WorldProject/v2 provenance carrier for exact external source entity identities mapped to exact canonical definition revisions. This task defines carrier and validation only; it does not populate G4 entities.

## Architecture and source of truth

- PROVEN: the accepted G4 decision keeps canonical Oteryn definition identity independent from source IDs, presentation/assets, and runtime/wire IDs.
- PROVEN: `ProjectV2Source` binds a source key/revision to its exact import batch and artifact digest.
- PROVEN: this implementation joins each entity binding to that exact source key/revision and resolves its target to an existing exact `(family, key, revision)` definition reference.
- PROVEN: identity uniqueness is `(source key, source revision, identity namespace, verbatim external ID)`; two lexical IDs such as `7` and `0007` remain distinct.
- PROVEN: `EXACT` and `ACCEPTED_ALIAS` are the only serializable binding dispositions. Ambiguous, conflicting, probable, and unmatched evidence is not representable as an accepted binding.
- PROVEN: client appearance IDs can bind to `Presentation`; the carrier does not infer an `Item` target from a presentation/asset ID.
- PROVEN: the repository binding pins META policy 3.1.0 at authority commit `21bc49bccef4874b037aabcbde9732b904187c32`. The policy document is not present in this checkout; the active control plane is tracking the #841 repin gate.

## High-risk authority/recovery qualification

```yaml
applicable: false
model: NOT_APPLICABLE
authority_invariants: []
consumer_boundaries:
  - offline project provenance parser and canonical writer
mutation_operators:
  applicable: []
  considered_not_applicable:
    - production authority or persistence mutation
    - session, lease, or generation fencing
    - PREPARE or COMMIT
one_invariant_per_negative_case: NOT_APPLICABLE
independent_current_fact_sources: []
record_derived_matching_helper:
  allowed_for_positive_happy_path: false
  forbidden_for_negative_authority_or_provenance_cases: true
finding_family_sweep:
  sibling_apis: NOT_APPLICABLE
  protocol_versions: NOT_APPLICABLE
  direct_and_reconciled_paths: NOT_APPLICABLE
  fenced_durable_writes: NOT_APPLICABLE
  restart_retry_replay_concurrency_pg_reload: NOT_APPLICABLE
  evidence: []
finding_dispositions:
  p0_p1_accepted_and_repaired: []
  p0_p1_rejected_with_exact_evidence: []
  p2_fixed_accepted_or_deferred: []
```

## Acceptance criteria

- [x] Binding is typed in existing `provenance/sources.json`; empty bindings preserve the existing serialized shape.
- [x] Source tuple resolves to existing exact `ProjectV2Source`; that record continues to bind digest/import provenance.
- [x] Target resolves by exact family/key/revision to an existing canonical definition.
- [x] External ID is retained as a bounded lexical string and namespace uses explicit `source/identifier_kind` segments.
- [x] Duplicate or competing mappings for one exact source identity are rejected; bindings have deterministic tuple ordering.
- [x] Binding dispositions serialize only as `EXACT` or `ACCEPTED_ALIAS`.
- [x] No runtime, wire ID, asset, source population, or mass G4 data change.
- [ ] Focused Rust tests and formatting pass in an environment with the repository Rust toolchain.

## Excluded scope

No G4 population, source collection, identity adjudication, runtime/wire allocation, asset redistribution, ProjectV3, item schema changes, or edits outside the four owned paths.

## Implementation / findings

The carrier adds `ProjectV2SourceIdentityBinding` and a disposition enum to v2 state and the existing provenance document. Validation checks exact source revision, target resolution, lexical identity strings, namespaced identifier-kind syntax, duplicate/conflicting tuple keys, ordering, and a bounded binding count capped by the existing configurable `ProjectEvidenceLimits.max_decoded_fields`. `client/appearance_id` is constrained to a Presentation target. Canonical writing sorts by the source identity tuple. Tests cover lexical preservation, same IDs in separate namespaces/revisions, Presentation-vs-Item targets, round-trip behavior, orphan source/target rejection, duplicates/conflicts, and invalid IDs/namespaces.

The supplied Item pilot example (TibiaWiki page ID 8513 at revision 433952, no Infobox ID) is compatible: the carrier permits the page ID as a namespaced source identity and does not require an Infobox ID.

## Validation

### Focused

- command/run: `cargo fmt --check`; focused Cargo test not yet runnable
- result: BLOCKED locally because `cargo`/`rustc` are absent from this execution environment. Exact Rust validation must run on a repository runner before candidate qualification.

### Component/integration

- command/run: NOT_APPLICABLE; carrier-only parser/writer change
- result: NOT_APPLICABLE

### E2E

- scenario: NOT_APPLICABLE; no runtime lowering or G4 population
- result: NOT_APPLICABLE

### Exact-head CI

- final head: pending
- trigger source: pending
- workflow/run/job: pending
- runner assignment: pending
- classification: pending
- result: pending

## Self-review

- exact head: pending
- method/reviewer: implementing agent
- material findings: pending Rust compilation/format validation
- verdict: pending

## Independent review

- required: pending under the bound policy and exact candidate risk classification
- exact head: pending
- method/auditor: pending
- material findings: pending
- verdict: pending

## PR and closeout

- changed-file review: pending
- unresolved review threads: pending
- related/superseded PRs: #839 #840
- protected auto-merge: pending active control-plane route
- merge commit/result: pending
- ownership release: pending

## Context checkpoint

```yaml
last_progress: typed carrier and focused tests authored in isolated local worktree
status: implementing
branch: agent/otv2-g4-source-identity-binding-v2
head_sha: null
pr: null
final_head_sha: null
final_head_frozen_at: null
ci_trigger_source: null
ci_check_generation: null
ci_checks_for_current_head: 0
ci_run_ids: []
ci_job_ids: []
runner_assignment_state: unknown
terminal_ci_wait_started_at: null
terminal_ci_checks_for_current_generation: 0
unchanged_state_checks: 0
identical_failure_retries: 0
repair_cycles_for_current_gate: 0
ci_recovery_actions_for_current_head: 0
stall_warnings: 0
owner_action_required: null
blocker: Rust toolchain unavailable in local execution environment
next_action: run formatting and focused tests on a toolchain-equipped candidate runner
```
