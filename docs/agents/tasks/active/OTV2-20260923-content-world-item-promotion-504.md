# OTV2-20260923-content-world-item-promotion-504

~~~yaml
task_id: OTV2-20260923-content-world-item-promotion-504
title: Item partial canonical semantic promotion
mode: IMPLEMENT
status: implementing
repository: Oteryn/Oteryn-Game
base_branch: main
branch: agent/content-world-item-semantic-promotion-504
pr: null
base_sha: 899d43de62d64d030bf0a894eac24956c7899287
head_sha: pending
final_head_sha: null
final_head_frozen_at: null
owner: "single autonomous Item product batch writer"
created_at: 2026-09-23T07:34:00Z
updated_at: 2026-09-23T07:34:00Z
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

- PROVEN: protected main is 899d43de62d64d030bf0a894eac24956c7899287; #773 and #774 are protected.
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

- [ ] Regenerated protected lineage yields exactly 69 eligible atomic promotion rows.
- [ ] Every promotion binds exact source numeric ID + native key + field path; no name-only fallback.
- [ ] Stable packet contains no mutable wiki revision/timestamp/source-digest metadata.
- [ ] Exact protected field partition is 22 name, 16 attack, 13 defense, 7 extra defense, 4 range, 2 hit chance, 3 armor, 1 charges, 1 container capacity.
- [ ] Typed conversion uses existing #749 types and fail-closed bounds.
- [ ] Existing 38,157 identity allocation and allocation digest remain unchanged.
- [ ] Promotion does not change materializable or stack class.
- [ ] Multiple promoted atoms on one Item merge without clobbering siblings.
- [ ] Non-promoted sibling atoms remain Unknown/Conflict or their protected state.
- [ ] Canonical Project lowers promoted full family through existing Reference linking.
- [ ] Artifact v4 compiles byte-identically and loads server-authoritative + client-safe projections.
- [ ] All 69 promoted atomic values round-trip through both allowed projections.
- [ ] No second Crystal/B1 import, identity generator, parser, model, rule engine or schema.
- [ ] Exact-head hosted qualification and game-gate are green.

## Excluded scope

No broad rediscovery, no fields beyond the exact eligible 69, no automatic materialization, no balance formula, protocol/persistence mutation or native-client UI work beyond existing artifact projection proof.

## Implementation / findings

A deterministic promotion-packet compiler and fail-closed synthetic tests are being added first. The packet strips mutable source revision metadata and retains only exact identity, field path, source value and accepted typed conversion.

## Validation

### Focused

- command/run: python tools/reference-world-corridor-census/item_semantic_promotion_self_test.py
- result: pending hosted exact-head execution

### Component/integration

- command/run: protected identity/current/verification/continuity reproduction -> promotion packet -> canonical Rust application -> artifact v4 compile/load
- result: pending

### E2E

- scenario: full 38,157 Project -> Reference -> v4 server/client roundtrip for exact 69 promoted atomic fields
- result: pending

### Exact-head CI

- final head: pending
- trigger source: pending
- workflow/run/job: pending
- runner assignment: pending
- classification: pending
- result: pending

## Self-review

- exact head: pending
- method/reviewer: implementing/coordinating agent
- material findings: pending
- verdict: pending

## Independent review

- required: NO unless final diff introduces material control-plane/authority/security risk
- exact head: NOT_APPLICABLE
- method/auditor: NOT_APPLICABLE
- material findings: NOT_APPLICABLE
- verdict: NOT_APPLICABLE

## PR and closeout

- changed-file review: pending
- unresolved review threads: pending
- related protected predecessors: #749, #763, #767, #770, #773, #774, #775
- protected Merge Queue: pending
- merge commit/result: pending
- ownership release: pending in this same product batch; no ordinary lifecycle-only successor

## Context checkpoint

~~~yaml
last_progress: deterministic promotion compiler and synthetic tests authored
status: implementing
branch: agent/content-world-item-semantic-promotion-504
head_sha: pending
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
owner_action_required: NONE
blocker: null
next_action: add hosted one-batch promotion qualification and generate exact 69-row packet
~~~
