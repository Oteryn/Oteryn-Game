# OTV2-20260922-content-world-item-target-continuity-504

~~~yaml
task_id: OTV2-20260922-content-world-item-target-continuity-504
title: Item target continuity bridge
mode: IMPLEMENT
status: implementing
repository: Oteryn/Oteryn-Game
base_branch: main
branch: agent/content-world-item-target-continuity-504
pr: null
base_sha: dd32da467c8d1e172c7ddedc91feb6c04a8893b3
head_sha: pending
final_head_sha: null
final_head_frozen_at: null
owner: "single autonomous Item content implementation agent"
created_at: 2026-09-22T20:37:00Z
updated_at: 2026-09-22T20:37:00Z
execution_policy: continuous_progress
owned_paths:
  - tools/reference-world-corridor-census/item_target_continuity.py
  - tools/reference-world-corridor-census/item_target_continuity_self_test.py
  - docs/agents/evidence/OTV2-20260922-content-world-item-target-continuity.json
  - docs/agents/tasks/active/OTV2-20260922-content-world-item-target-continuity-504.md
  - .github/workflows/item-content-continuity.yml
public_contracts:
  - docs/agents/programs/OTERYN_REFERENCE_INVESTIGATION_SOURCE_REGISTRY_20260910.md
  - docs/contracts/REFERENCE_EVIDENCE_PARITY_MANIFEST_V1.json
depends_on:
  - "#767 protected TibiaWiki current-source collector"
  - "#770 protected atomic Item field verification"
  - "#771 protected lifecycle closeout"
blocks:
  - ITEM_SEMANTIC_PROMOTION
cross_repository_coordination_id: null
external_repositories: []
~~~

## Outcome

Build a bounded historical target-continuity bridge for atomic Item fields that the protected #770 rule engine classifies as CORROBORATED_CURRENT.

This generation does not promote Item semantics. It only upgrades continuity from UNKNOWN to at most DERIVED when historical structured-reference evidence spans the full target calendar day without a field-level contradiction.

## Architecture and source of truth

- PROVEN: protected main is dd32da467c8d1e172c7ddedc91feb6c04a8893b3; #770 and lifecycle #771 are protected.
- PROVEN: the #770 compact manifest closes 2,022,321 field slots and records 67 CORROBORATED_CURRENT slots, all still promotion-blocked because target continuity is unresolved.
- PROVEN: the protected source registry allows a consistent static field to become an explicitly reasoned DERIVED candidate when target continuity is supported and no stronger conflict exists; wiki-only never becomes automatic PROVEN.
- PROVEN: September 2026 current values are not automatically the 2026-07-28 target values.
- PROVEN: the protected #767 TibiaWiki parser/normalizer is the canonical structured Item parser for this lineage and must be reused.
- DERIVED: because the accepted target boundary has date-only precision, a conservative bridge can span the entire 2026-07-28 calendar day: compare the last normalized wiki revision before the day, every bounded revision during the day, and the current already-corroborated value. Equality supports DERIVED; disagreement is CONFLICT; incomplete history stays UNKNOWN.
- UNKNOWN: #770 intentionally retained only compact manifest + full-output digest, not the full 38,157 scratch bytes. This successor therefore regenerates a fresh current-source/field-verification scratch under the protected algorithms and must fail closed if its aggregate #770 state partition drifts from the protected compact manifest.

## High-risk authority/recovery qualification

~~~yaml
applicable: false
model: NOT_APPLICABLE
authority_invariants: []
consumer_boundaries:
  - read-only external structured evidence
  - evidence compiler only
mutation_operators:
  applicable: []
  considered_not_applicable:
    - runtime authority
    - persistence authority
    - semantic promotion
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
~~~

## Acceptance criteria

- [ ] Reuse the protected #767 parser/normalizer; no second Item parser.
- [ ] Examine only atomic fields freshly reproduced as CORROBORATED_CURRENT under the protected #770 algorithm.
- [ ] Fail closed if the fresh #770 aggregate field-state partition differs from the protected #770 compact manifest.
- [ ] Bind continuity by exact protected native key + source numeric ID + already-matched TibiaWiki page ID; never by name.
- [ ] Fetch at most one last revision before 2026-07-28 and at most 128 revisions during the target day per page.
- [ ] DERIVED requires the exact atomic normalized value to equal the current corroborated value in the pre-target revision and every 2026-07-28 revision that exists.
- [ ] Any differing historical value yields field-level CONFLICT.
- [ ] Missing pre-target history or missing/unparsed historical field yields UNKNOWN.
- [ ] Never emit PROVEN.
- [ ] Never perform semantic promotion.
- [ ] History cache is bounded, contains normalized evidence only, and allows a byte-identical second compiler run without another network fetch.
- [ ] Compact committed manifest records counts, digests and explicit non-promotion invariants.
- [ ] Exact-head hosted qualification is green.

## Excluded scope

No Crystal/B1 reimport, identity regeneration, broad rediscovery, Item schema mutation, canonical Project/Reference semantic mutation, server/client artifact mutation, runtime/client wiring, protocol, persistence, balance formula or gameplay authority.

## Implementation / findings

The bridge is intentionally restricted to static atomic field continuity. It does not use absence of a wiki edit as PROVEN; historical wiki evidence can produce at most DERIVED.

Target-day coverage uses the whole UTC calendar day rather than inventing an exact server-save minute. If the historical page lacks a pre-target revision, the field remains UNKNOWN.

## Validation

### Focused

- command/run: python tools/reference-world-corridor-census/item_target_continuity_self_test.py
- result: pending hosted exact-head execution

### Component/integration

- command/run: regenerate protected identity/crosswalk/current-source/field-verification scratch, assert #770 aggregate state partition, then collect historical continuity for the bounded corroborated field set
- result: pending

### E2E

- scenario: NOT_APPLICABLE; evidence-only generation
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
- method/reviewer: implementing/coordinating agent
- material findings: pending
- verdict: pending

## Independent review

- required: pending
- exact head: pending
- method/auditor: pending
- material findings: pending
- verdict: pending

## PR and closeout

- changed-file review: pending
- unresolved review threads: pending
- related/superseded PRs: #767, #770, #771 protected predecessors
- protected auto-merge: pending
- merge commit/result: pending
- ownership release: pending

## Context checkpoint

~~~yaml
last_progress: continuity compiler and synthetic self-test authored on allocated branch
status: implementing
branch: agent/content-world-item-target-continuity-504
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
next_action: add hosted full-lineage continuity qualification and execute it
~~~
