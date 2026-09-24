# OTV2-20260924-g4-source-identity-binding-v2

```yaml
task_id: OTV2-20260924-g4-source-identity-binding-v2
title: G4 v2 source identity binding carrier
mode: IMPLEMENT
status: implementing
repository: Oteryn/Oteryn-Game
base_branch: main
branch: agent/otv2-g4-source-identity-binding-v2-r3
issue: 162
pr: null
base_sha: b003913926b7de941d0ed6b32b438eeb0fe96c04
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
- PROVEN: the repository binding at protected successor base `b003913926b7de941d0ed6b32b438eeb0fe96c04` pins META policy 3.1.0 at authority commit `1bfb5ff98c8aa156e73669a14e083a1d464c29fb`; merged PR #841 delivered the repin.
- PROVEN: PR #846 passed historical CI run `36017466314` but was superseded for commit cardinality. PR #847 at `5bdeec2da9835e581bc9750ff7052028cd548dd2` passed Rust build, Clippy, tests and formatting in run `36019707374`, but governance rejected the missing positive issue/PR field. Both are historical only and do not qualify this r3 candidate.

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

- command/run: predecessor PR #846 run `36017466314`, exact head `c439aaf01328d88255a8339c57acb48595acfcd7`
- result: predecessor historical PASS for format and Rust tests. It is not successor validation; new-head focused validation is pending.

### Component/integration

- command/run: NOT_APPLICABLE; carrier-only parser/writer change
- result: NOT_APPLICABLE

### E2E

- scenario: NOT_APPLICABLE; no runtime lowering or G4 population
- result: NOT_APPLICABLE

### Exact-head CI

- final head: pending
- trigger source: pending successor candidate publication
- workflow/run/job: pending
- runner assignment: pending
- classification: content v2 parser/writer and focused test changes
- result: successor validation pending. Historical predecessor run `36017466314` at `c439aaf01328d88255a8339c57acb48595acfcd7` passed its applicable gates, including Rust Linux workspace, formatting and `game-gate`; do not reuse that evidence as successor qualification.

## Self-review

- exact head: predecessor-only `c439aaf01328d88255a8339c57acb48595acfcd7`
- method/reviewer: implementing agent whole-diff review
- material findings: applied all rustfmt suggestions from job `107690327709`; no semantic changes
- verdict: predecessor static review only; successor whole-diff review pending

## Independent review

- required: YES under bound policy for source identity/provenance semantics
- exact head: pending successor head
- method/auditor: pending independent successor review; predecessor static review at `c439aaf01328d88255a8339c57acb48595acfcd7` found no material code issue
- material findings: pending
- verdict: pending; predecessor review is historical only

## PR and closeout

- changed-file review: four owned paths only; formatter-only code diff plus evidence/task CI and lifecycle updates
- unresolved review threads: pending successor review
- related/superseded PRs: #839 #840; predecessors #846 and #847 superseded
- protected auto-merge: pending active control-plane route; #841 is merged at successor base
- merge commit/result: pending
- ownership release: pending

## Context checkpoint

```yaml
last_progress: exclusive r3 successor allocated after PR #847 governance failure; protected base includes #841
status: implementing
branch: agent/otv2-g4-source-identity-binding-v2-r3
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
blocker: successor exact-head CI and independent review pending
next_action: deliver final four-file content and blob/tree hashes to active control plane
```
