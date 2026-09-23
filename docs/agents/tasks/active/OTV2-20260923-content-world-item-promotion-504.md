# OTV2-20260923-content-world-item-promotion-504

~~~yaml
task_id: OTV2-20260923-content-world-item-promotion-504
title: Item partial canonical semantic promotion
mode: IMPLEMENT
status: validating
repository: Oteryn/Oteryn-Game
base_branch: main
branch: agent/content-world-item-semantic-promotion-504-r4
pr: 782
base_sha: 19c15ebf8377608db5b72cf7082dc856c4fd76be
head_sha: 1c2aa8973b870bb471c44439248ddf5f18bb41fb
final_head_sha: null
final_head_frozen_at: null
owner: "single autonomous Item product batch writer"
created_at: 2026-09-23T07:34:00Z
updated_at: 2026-09-23T08:39:37Z
execution_policy: continuous_progress
owned_paths:
  - tools/reference-world-corridor-census/item_semantic_promotion.py
  - tools/reference-world-corridor-census/item_semantic_promotion_self_test.py
  - docs/agents/evidence/OTV2-20260923-content-world-item-semantic-promotion.json
  - apps/game-server/src/content/cw2_b1_import.rs
  - apps/game-server/tests/content_world_cw2_b1_import.rs
  - apps/game-server/tests/content_reference_artifact.rs
  - .github/workflows/item-content-promotion.yml
  - docs/agents/tasks/active/OTV2-20260923-content-world-item-promotion-504.md
public_contracts:
  - docs/agents/programs/OTERYN_REFERENCE_INVESTIGATION_SOURCE_REGISTRY_20260910.md
  - docs/contracts/REFERENCE_EVIDENCE_PARITY_MANIFEST_V1.json
depends_on:
  - "#749 protected typed Item schema/artifact v4"
  - "#763 protected exact 38157 Item identity crosswalk"
  - "#767 protected structured current-source collector"
  - "#770 protected atomic field verification"
  - "#774 protected target-continuity bridge"
  - "#773 protected one-batch Item orchestration"
blocks:
  - ITEM_DOWNSTREAM_RUNTIME_CLIENT_QUALIFICATION
cross_repository_coordination_id: null
external_repositories: []
~~~

## Outcome

Promote exactly the 69 atomic Item fields protected by #774 into the existing #749 ReferenceItemSemantics graph, preserve every sibling field as Unknown/Conflict or its existing state, and prove canonical Project -> artifact-v4 server/client roundtrip in this same product batch.

## Architecture and source of truth

- PROVEN: promotion r3 base is protected main `19c15ebf8377608db5b72cf7082dc856c4fd76be`; #773/#774/#775 are protected. The intervening protected-main delta from `899d43de62d64d030bf0a894eac24956c7899287` has zero overlap with this batch's eight owned paths.
- PROVEN: #774 closes 69 eligible atomic fields across 23 pages with DERIVED continuity and zero UNKNOWN/CONFLICT inside the eligible set.
- PROVEN: #749 owns the typed immutable Item schema and artifact-v4 encoding; no second model/parser/schema is allowed.
- PROVEN: #763 owns the exact 38,157 source-to-native allocation; promotion cannot remint or name-resolve identity.
- PROVEN: mutable wiki revision/timestamp metadata is provenance only and cannot become a stable semantic-key input.
- DERIVED: the protected TibiaWiki hit atom is a chance-to-hit percentage-point modifier; the accepted typed destination is ReferenceRationalPercent, so promotion stores the exact integer source value as a reduced rational over 100.
- PROVEN: partial promotion does not authorize whole-Item Known state or automatic materialization.

## High-risk authority/recovery qualification

~~~yaml
applicable: false
model: NOT_APPLICABLE
authority_invariants: []
consumer_boundaries: [immutable Item content semantics, artifact compiler/load validation]
mutation_operators:
  applicable: []
  considered_not_applicable: [production authority, persistence authority, PREPARE, COMMIT]
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
~~~

## Acceptance criteria

- [x] Regenerated protected lineage yields exactly 69 eligible atomic promotion rows.
- [x] Every promotion binds exact source numeric ID + native key + field path; no name-only fallback.
- [x] Stable packet contains no mutable wiki revision/timestamp/source-digest metadata.
- [x] Exact protected field partition is 22 name, 16 attack, 13 defense, 7 extra defense, 4 range, 2 hit chance, 3 armor, 1 charges, 1 container capacity.
- [x] Typed conversion uses existing #749 types and fail-closed bounds.
- [x] Existing 38,157 identity allocation and allocation digest remain unchanged.
- [x] Promotion does not change materializable or stack class.
- [x] Multiple promoted atoms on one Item merge without clobbering siblings.
- [x] Non-promoted sibling atoms remain Unknown/Conflict or their protected state.
- [x] Canonical Project lowers promoted full family through existing Reference linking.
- [x] Artifact v4 compiles byte-identically and loads server-authoritative + client-safe projections.
- [x] All 69 promoted atomic values round-trip through both allowed projections.
- [x] No second Crystal/B1 import, identity generator, parser, model, rule engine or schema.
- [ ] Exact-head hosted qualification and game-gate are green.

## Excluded scope

No broad rediscovery, no fields beyond the exact eligible 69, no automatic materialization, no balance formula, protocol/persistence mutation or native-client UI work beyond existing artifact projection proof.

## Implementation / findings

A deterministic promotion-packet compiler and fail-closed synthetic tests are implemented. The packet strips mutable source revision metadata and retains only exact identity, field path, source value and accepted typed conversion.

Precursor #776 exact-head promotion workflow `35834279290`, job `107093955608`, completed SUCCESS on the byte-identical eight-path implementation: 69 promoted fields across 23 items, packet SHA-256 `3e117c1f722e11d5b2e6339a512cd5ee906128e9644643083bef1d3632b4d1c1`; full protected identity/current/verification/continuity reproduction passed; the exact 69-row packet reproduced byte-identically; focused canonical importer plus artifact-v4 server-authoritative/client-safe roundtrip passed. #776 was superseded only because its old base predates the protected routing-contract fix. #779 replayed all eight owned-path blobs byte-for-byte on the refreshed protected base, then exact-head Merge Gate exposed only `cargo fmt --check` in job `107100545990`. The frozen #779 candidate was not mutated. #780/r3 applies exactly the 23 repo-pinned rustfmt hunks to the three Rust files; every hunk matched uniquely, and the resulting three blob SHAs are identical to the independently produced parallel format repair on the superseded #779 branch. Promotion semantics/evidence/workflow remain unchanged. #780 exact-head semantic-promotion workflow `35837247344` / `107103557120` then passed on the r3 semantics, while Merge Gate Rust Linux job `107103962880` failed only two `clippy::collapsible_if` findings in the promotion-count test helper. Frozen #780 was not mutated. #782/r4 collapses exactly those two nested test-only conditionals into Rust 1.94 let-chains; no product semantics, evidence, workflow, identity, parser/model/schema, routing or gameplay scope changed.

## Validation

### Focused

- command/run: `python tools/reference-world-corridor-census/item_semantic_promotion_self_test.py` in precursor workflow `35834279290`, job `107093955608`
- result: PASS on #780 semantic-promotion workflow `35837247344` / `107103557120`; final #782 exact-head rerun required

### Component/integration

- command/run: protected identity/current/verification/continuity reproduction -> promotion packet -> canonical Rust application -> artifact v4 compile/load
- result: precursor #776 PASS in `35834279290` / `107093955608`; #780 semantic-promotion workflow also PASS with identical 69 fields / 23 items / packet SHA-256 `3e117c1f722e11d5b2e6339a512cd5ee906128e9644643083bef1d3632b4d1c1`; final #782 exact-head rerun required

### E2E

- scenario: full 38,157 Project -> Reference -> v4 server/client roundtrip for exact 69 promoted atomic fields
- result: precursor exact test `protected_semantic_promotion_round_trips_exact_69_atoms_through_artifact_v4_server_and_client` PASS and #780 semantic-promotion workflow PASS; final #782 exact-head rerun required

### Exact-head CI

- final head: recorded in immutable PR/check evidence after this final authoring mutation; a commit cannot contain its own SHA
- trigger source: pull_request
- workflow/run/job: pending final frozen #782 generation; #780 product proof `35837247344` / `107103557120` SUCCESS
- runner assignment: repository-selected GitHub hosted runners
- classification: required final refreshed-base candidate qualification
- result: pending

## Self-review

- exact head: final frozen head will be bound in immutable PR/check evidence after this commit
- method/reviewer: implementing/coordinating agent
- material findings: #779 replay blob-equality check PASS for all eight owned paths versus #776; protected-main delta to refreshed base has zero owned-path overlap; #779 format-only failure reproduced and repaired from exact CI rustfmt diff; 23/23 hunks uniquely matched; final three Rust blob SHAs agree with the independently produced parallel format repair; semantic whole-diff review found no overwrite/clobber path because `promote_unknown` fails closed on non-Unknown atoms; final frozen-head checks still required
- verdict: pending final exact-head readback

## Independent review

- required: NO unless final diff introduces material control-plane/authority/security risk
- exact head: NOT_APPLICABLE
- method/auditor: NOT_APPLICABLE
- material findings: NOT_APPLICABLE
- verdict: NOT_APPLICABLE

## PR and closeout

- changed-file review: #782 retains the same eight owned paths; r4 delta over frozen #780 is Clippy-only in one Rust test file plus this task binding; final frozen whole-diff review pending
- unresolved review threads: 0 at r3 admission
- related protected predecessors: #749, #763, #767, #770, #773, #774, #775; superseded implementation PRs #776, #779 and #780 closed/unmerged after #782 establishment
- protected Merge Queue: pending final frozen-head qualification
- merge commit/result: pending
- ownership release: pending in this same product batch; no ordinary lifecycle-only successor

## Context checkpoint

~~~yaml
last_progress: #780 semantic-promotion workflow PASS; exact Rust Linux blocker reduced to two test-only collapsible_if findings; r4 repair applied and PR #782 established
status: validating
branch: agent/content-world-item-semantic-promotion-504-r4
head_sha: 1c2aa8973b870bb471c44439248ddf5f18bb41fb
pr: 782
final_head_sha: null
final_head_frozen_at: null
ci_trigger_source: pull_request
ci_check_generation: pending final authoring commit
ci_checks_for_current_head: 0
ci_run_ids: [35834279290, 35837247344, 35837247354]
ci_job_ids: [107093955608, 107103557120, 107103962880]
runner_assignment_state: product_success_clippy_only_failure_repaired
terminal_ci_wait_started_at: null
terminal_ci_checks_for_current_generation: 0
unchanged_state_checks: 0
identical_failure_retries: 0
repair_cycles_for_current_gate: 3
ci_recovery_actions_for_current_head: 3
stall_warnings: 0
owner_action_required: NONE
blocker: null
next_action: freeze the branch head returned by this final task-metadata mutation and require fresh #782 exact-head semantic-promotion workflow plus full Merge Gate
~~~
