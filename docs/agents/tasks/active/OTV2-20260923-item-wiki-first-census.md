# OTV2-20260923-item-wiki-first-census

```yaml
task_id: OTV2-20260923-item-wiki-first-census
title: Wiki-first TibiaWiki Item census
mode: IMPLEMENT
status: implementing
repository: Oteryn/Oteryn-Game
base_branch: main
branch: agent/item-wiki-first-census-20260923
pr: 803
base_sha: 79a1d6966b7fbe41e8366ea873d207c59d441033
head_sha: null
final_head_sha: null
final_head_frozen_at: null
owner: "single autonomous Item wiki-first census writer"
created_at: 2026-09-23T21:19:00+02:00
updated_at: 2026-09-23T21:37:00+02:00
execution_policy: continuous_progress
owned_paths:
  - tools/reference-world-corridor-census/item_wiki_first_census.py
  - tools/reference-world-corridor-census/item_wiki_first_census_self_test.py
  - .github/workflows/item-wiki-first-census.yml
  - docs/agents/evidence/OTV2-20260923-item-wiki-first-census.json
  - docs/agents/tasks/active/OTV2-20260923-item-wiki-first-census.md
public_contracts: []
depends_on:
  - "PR #798 protected WorldProject/v2 full schema coverage"
  - "PR #767 protected bounded TibiaWiki Item source collector primitives"
blocks:
  - WIKI_FIRST_ITEM_IDENTITY_CROSSWALK
cross_repository_coordination_id: null
external_repositories: []
```

## Outcome

Replace the old Crystal-first discovery direction for the next Item population wave with one bounded TibiaWiki-first census. Discover current main-namespace pages directly using the Item infobox template, fetch only bounded infobox source evidence, retain the complete infobox field partition in scratch output, and emit a compact manifest suitable for the next multi-signal identity-crosswalk gate.

## Architecture and source of truth

- PROVEN: protected main includes WorldProject/v2 Document and modern Item authoring coverage from PR #798; this task must not add another Item or readable-content model.
- PROVEN: the archived PR #767 Crystal-first current-source run started from 38,157 protected identities and closed at MATCHED=22, NOT_FOUND=729, AMBIGUOUS=36,736, CONFLICT=670. That result is retained as predecessor evidence, not reused as the discovery root.
- PROVEN: TibiaWiki remains structured reference data, not Reference gameplay truth.
- PROVEN: long-form TibiaWiki/Tibia prose and book bodies are outside the permitted bulk corpus; only bounded Infobox Item fields are collected.
- DERIVED: direct main-namespace transclusion of the TibiaWiki Item infobox is the narrowest current wiki-first discovery surface that avoids starting from Crystal identity.

## High-risk authority/recovery qualification

```yaml
applicable: false
model: NOT_APPLICABLE
authority_invariants: []
consumer_boundaries:
  - read-only public-source census evidence
mutation_operators:
  applicable: []
  considered_not_applicable:
    - production authority
    - persistence mutation
    - session or lease fencing
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

- [ ] Discovery starts from TibiaWiki Item infobox transclusion in namespace 0, never from the 38,157 Crystal-derived identity closure.
- [ ] Discovery pagination, continuation, response shape, duplicate IDs/titles and maximum page/request bounds fail closed.
- [ ] Every admitted page is fetched at an exact current revision and contains an Item infobox.
- [ ] Both already-normalized and previously-unmapped bounded infobox fields are retained in deterministic scratch output.
- [ ] Long-form article/book body text and raw wikitext are not retained in the product or committed manifest.
- [ ] Full scratch output is not committed; compact manifest records page/field counts, digests, invariants and limitations.
- [ ] No Oteryn identity minting, Crystal/OTS identity resolution, semantic promotion, runtime/client mutation or balance change occurs.
- [ ] Focused self-test plus one live hosted wiki-first census pass on the frozen exact head.
- [ ] Repository-required exact-head PR qualification passes before Merge Queue admission.

## Excluded scope

No identity crosswalk, no field-verification promotion, no WorldProject population, no Reference semantic promotion, no description/book-body import, no runtime/client/protocol/persistence mutation, no second Item model, and no broad copy of TibiaWiki content.

## Implementation / findings

Candidate `7df3227a2639343531a5c9ccb4464c5a1ac34fc7` was unfrozen after dedicated workflow run `35910105739` failed before tests: `actions/checkout` received a literal backslash-prefixed SHA because the authored YAML escaped GitHub expression syntax as `\\${{ ... }}`. This is accepted as a workflow-only P1 qualification defect; collector semantics were not executed. AUTHORING was reopened for the exact expression-escape repair. The workflow successor removes exactly five literal backslashes before GitHub expression tokens; no collector, self-test or scope semantics changed.\n\nSuccessor `53cfa7a9e0c45cf9b23a1c676ab0327479ae8f51` passed checkout and predecessor self-test, then new self-test failed before live collection in `compile_census`: `Counter.update(mapping)` attempted to add dict field payloads as counts. This is accepted as a local aggregation bug. AUTHORING is reopened for the minimal three-site change to count mapping keys rather than mapping values.\n\nThe implementation reuses the protected bounded TibiaWiki request/cache/Infobox parser primitives rather than forking a second HTTP or Item parser. The new discovery direction is independent of Crystal identities and enumerates main-namespace pages that directly transclude `Predefinição:Infobox Item`. A small compatibility shim accepts the MediaWiki-equivalent space/underscore spelling of the infobox marker without changing the protected predecessor collector. PR #803 was opened from the four-path authored candidate. Its base is protected main `6b2c5237cebcc9cf757747478a2990cde4109750`; the only protected-main change since admission base `79a1d6966b7fbe41e8366ea873d207c59d441033` is the path-disjoint archive move for the completed #798 task.

## Validation

### Focused

- command/run: `python tools/reference-world-corridor-census/item_wiki_first_census_self_test.py`
- result: pending exact-head PR #803 generation

### Component/integration

- command/run: hosted live `item_wiki_first_census.py` against TibiaWiki public MediaWiki API
- result: pending authored candidate

### E2E

- scenario: NOT_APPLICABLE; source-evidence census has no executable lowering
- result: NOT_APPLICABLE

### Exact-head CI

- final head: pending
- trigger source: pull_request
- workflow/run/job: pending
- runner assignment: repository-selected
- classification: content evidence tooling
- result: pending

## Self-review

- exact head: pending
- method/reviewer: implementing/coordinating agent
- material findings: pending
- verdict: pending

## Independent review

- required: NO unless final diff introduces authority/security/durable-state semantics
- exact head: NOT_APPLICABLE
- method/auditor: NOT_APPLICABLE
- material findings: NOT_APPLICABLE
- verdict: NOT_APPLICABLE

## PR and closeout

- changed-file review: pending
- unresolved review threads: pending
- related/superseded PRs: #767 #770 #774 #782 #792 #798
- protected auto-merge: forbidden substitute; native Merge Queue only
- merge commit/result: pending
- ownership release: pending

## Context checkpoint

```yaml
last_progress: successor passed checkout; focused self-test exposed Counter mapping-value aggregation bug before live source collection; minimal key-count repair authorized
status: implementing
branch: agent/item-wiki-first-census-20260923
head_sha: null
pr: 803
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
repair_cycles_for_current_gate: 2
ci_recovery_actions_for_current_head: 2
stall_warnings: 0
owner_action_required: null
blocker: null
next_action: change only the three Counter updates to count mapping keys, then refreeze and rerun exact-head qualification
```
