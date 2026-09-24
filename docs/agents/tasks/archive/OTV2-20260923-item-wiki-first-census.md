# OTV2-20260923-item-wiki-first-census

```yaml
task_id: OTV2-20260923-item-wiki-first-census
title: Wiki-first TibiaWiki Item census
mode: IMPLEMENT
status: completed
repository: Oteryn/Oteryn-Game
base_branch: main
branch: agent/item-wiki-first-census-20260923
pr: 803
base_sha: 79a1d6966b7fbe41e8366ea873d207c59d441033
head_sha: 28032e4a43e5eaaa73b8bbb0504cc85e3c234925
final_head_sha: 28032e4a43e5eaaa73b8bbb0504cc85e3c234925
final_head_frozen_at: 2026-09-23T22:24:18+02:00
owner: "single autonomous Item wiki-first census writer"
created_at: 2026-09-23T21:19:00+02:00
updated_at: 2026-09-23T22:42:07+02:00
execution_policy: continuous_progress
owned_paths:
  - tools/reference-world-corridor-census/item_wiki_first_census.py
  - tools/reference-world-corridor-census/item_wiki_first_census_self_test.py
  - .github/workflows/item-wiki-first-census.yml
  - docs/agents/evidence/OTV2-20260923-item-wiki-first-census.json
  - docs/agents/tasks/archive/OTV2-20260923-item-wiki-first-census.md
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

Replace the old Crystal-first discovery direction for the next Item population wave with one bounded TibiaWiki-first census. Discover current main-namespace pages directly from `Categoria:Itens`, fetch only bounded infobox source evidence, retain the complete infobox field partition in scratch output, and emit a compact manifest suitable for the next multi-signal identity-crosswalk gate.

## Architecture and source of truth

- PROVEN: protected main includes WorldProject/v2 Document and modern Item authoring coverage from PR #798; this task must not add another Item or readable-content model.
- PROVEN: the archived PR #767 Crystal-first current-source run started from 38,157 protected identities and closed at MATCHED=22, NOT_FOUND=729, AMBIGUOUS=36,736, CONFLICT=670. That result is retained as predecessor evidence, not reused as the discovery root.
- PROVEN: TibiaWiki remains structured reference data, not Reference gameplay truth.
- PROVEN: long-form TibiaWiki/Tibia prose and book bodies are outside the permitted bulk corpus; only bounded Infobox Item fields are collected.
- PROVEN: live candidate `ba04ce3827233e4b66de004418e6f66aa861b7de` showed that `embeddedin` for the Item template admits non-Item page `Abyssador`, so template dependency is not a safe Item identity census root.
- DERIVED: direct main-namespace membership in `Categoria:Itens` is the narrower wiki-owned discovery surface; admitted members may use heterogeneous source shapes; the census must retain their identity/page revision while collecting bounded Item-infobox fields only when that infobox is actually present.

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

- [x] Discovery starts from direct main-namespace membership in TibiaWiki `Categoria:Itens`, never from template dependency or the 38,157 Crystal-derived identity closure.
- [x] Discovery pagination, continuation, response shape, duplicate IDs/titles and maximum page/request bounds fail closed.
- [x] Every admitted category member is fetched at an exact current revision and records its source shape explicitly: Item infobox present or no Item infobox. Lack of the infobox never deletes a category-admitted Item from the census.
- [x] Both already-normalized and previously-unmapped bounded infobox fields are retained when the Item infobox parses losslessly; source-level infobox conflicts/errors retain the page as an explicit INFOBOX_ITEM_PARSE_ERROR without guessing a field value.
- [x] Long-form article/book body text and raw wikitext are not retained in the product or committed manifest.
- [x] Full scratch output is not committed; compact manifest records page/field counts, digests, invariants and limitations.
- [x] No Oteryn identity minting, Crystal/OTS identity resolution, semantic promotion, runtime/client mutation or balance change occurs.
- [x] Focused self-test plus one live hosted wiki-first census pass on the frozen exact head.
- [x] Repository-required exact-head PR qualification passes before Merge Queue admission.

## Excluded scope

No identity crosswalk, no field-verification promotion, no WorldProject population, no Reference semantic promotion, no description/book-body import, no runtime/client/protocol/persistence mutation, no second Item model, and no broad copy of TibiaWiki content.

## Implementation / findings

Candidate `7df3227a2639343531a5c9ccb4464c5a1ac34fc7` was unfrozen after dedicated workflow run `35910105739` failed before tests: `actions/checkout` received a literal backslash-prefixed SHA because the authored YAML escaped GitHub expression syntax as `\\${{ ... }}`. This is accepted as a workflow-only P1 qualification defect; collector semantics were not executed. AUTHORING was reopened for the exact expression-escape repair. The workflow successor removes exactly five literal backslashes before GitHub expression tokens; no collector, self-test or scope semantics changed.

Successor `53cfa7a9e0c45cf9b23a1c676ab0327479ae8f51` passed checkout and predecessor self-test, then new self-test failed before live collection in `compile_census`: `Counter.update(mapping)` attempted to add dict field payloads as counts. This is accepted as a local aggregation bug. AUTHORING is reopened for the minimal four-call change to count mapping keys rather than mapping values.

Successor `955bdd3dfb69c99c43f22b026325283cb4ae7c69` passed both focused self-test suites and entered live source collection. Live run `35910507151` then rejected discovered `page_id=25288` as `DISCOVERED_PAGE_WITHOUT_INFOBOX`. Because discovery itself came from MediaWiki `embeddedin` for the Item template, the bounded working inference is template-invocation spelling normalization rather than identity evidence. AUTHORING was reopened only to recognize MediaWiki-equivalent case/whitespace/optional namespace-prefix spellings, add synthetic coverage, and include the page title in any residual rejection. The successor uses one bounded regex for the direct base Item infobox invocation and explicitly rejects subtemplate names. Live rerun `35910837495` still admitted `Abyssador`; because that page is a Creature/Boss, the finding rejects `embeddedin` itself as the census root rather than expanding the parser to misclassify a creature. Category-root candidate `4ccad5d032ea79aa094accc5ce2db0f4296e7e1a` then admitted `0152551751 (Book)`, a real Item-category document page without the base Item infobox. This finding rejects the assumption that all Item-category pages share one infobox; absence becomes an explicit source-shape state, while long-form page/book text remains excluded. Heterogeneous-source candidate `2eb06304f1b7b66a409d773cbae2b17f08b6dd2e` passed 17 predecessor tests and 12 census tests, then live collection found `INFOBOX_DUPLICATE_CONFLICT:npcvalue`. The source page must remain in census without choosing between conflicting values; this introduces a third explicit source shape `INFOBOX_ITEM_PARSE_ERROR` carrying only a bounded parser error code.

The implementation reuses the protected bounded TibiaWiki request/cache/Infobox parser primitives rather than forking a second HTTP or Item parser. The new discovery direction is independent of Crystal identities and enumerates direct main-namespace members of `Categoria:Itens`. A small compatibility shim accepts the MediaWiki-equivalent space/underscore spelling of the infobox marker without changing the protected predecessor collector. PR #803 was opened from the four-path authored candidate. Its base is protected main `6b2c5237cebcc9cf757747478a2990cde4109750`; the only protected-main change since admission base `79a1d6966b7fbe41e8366ea873d207c59d441033` is the path-disjoint archive move for the completed #798 task.

## Validation

### Focused

- command/run: `python tools/reference-world-corridor-census/item_wiki_first_census_self_test.py`
- result: PASS in run `35911900308`, job `107353574189`; predecessor self-test 17/17 and wiki-first census self-test PASS before live collection

### Component/integration

- command/run: hosted live `item_wiki_first_census.py` against TibiaWiki public MediaWiki API
- result: PASS in run `35911900308`, job `107353574189`: 6,918 category members fetched; 5,475 INFOBOX_ITEM; 7 INFOBOX_ITEM_PARSE_ERROR; 1,436 NO_INFOBOX_ITEM; 83 distinct infobox fields; stable digest `389875abd364aa9bcb0b09a591989c82ece5098d63b3c23376274048f6ac2f5a`

### E2E

- scenario: NOT_APPLICABLE; source-evidence census has no executable lowering
- result: NOT_APPLICABLE

### Exact-head CI

- final head: `28032e4a43e5eaaa73b8bbb0504cc85e3c234925`
- trigger source: pull_request on PR #803
- workflow/run/job: Merge Gate `35915719740` / aggregate `game-gate` `107369795402`; Architecture Semantic Audit `35915719959`; Agent Governance `35915719696`; Item Wiki-First Census `35915719841`
- runner assignment: repository-selected
- classification: content evidence tooling
- result: SUCCESS

## Self-review

- exact head: `28032e4a43e5eaaa73b8bbb0504cc85e3c234925`
- method/reviewer: implementing/coordinating agent
- material findings: repaired six bounded generations: checkout expression escaping; Counter key aggregation; unsafe template-dependency discovery; MediaWiki Item infobox spelling; heterogeneous no-infobox Item pages; explicit conflicting-infobox parse-error retention
- verdict: PASS — final exact-head census, governance, architecture and repository Merge Gate all succeeded after synchronization with protected main

## Independent review

- required: NO unless final diff introduces authority/security/durable-state semantics
- exact head: NOT_APPLICABLE
- method/auditor: NOT_APPLICABLE
- material findings: NOT_APPLICABLE
- verdict: NOT_APPLICABLE

## PR and closeout

- changed-file review: PASS — implementation PR #803 changed only collector, synthetic test, focused workflow, compact evidence manifest and this task packet; no runtime/schema/WorldProject/Reference semantic mutation
- unresolved review threads: 0
- related/superseded PRs: #767 #770 #774 #782 #792 #798
- protected auto-merge: forbidden substitute; native Merge Queue only
- merge commit/result: `c07240d50473b8697cbe10641028cd0e7eb2d1e4` via protected Merge Queue run `35916776942`; final merge-group `game-gate` job `107373242888` SUCCESS
- ownership release: completed

## Context checkpoint

```yaml
last_progress: PR #803 merged through protected Merge Queue; protected main readback equals integrated commit c07240d50473b8697cbe10641028cd0e7eb2d1e4; lifecycle archived and ownership released
status: completed
branch: agent/item-wiki-first-census-20260923
head_sha: 28032e4a43e5eaaa73b8bbb0504cc85e3c234925
pr: 803
final_head_sha: 28032e4a43e5eaaa73b8bbb0504cc85e3c234925
final_head_frozen_at: 2026-09-23T22:24:18+02:00
ci_trigger_source: pull_request
ci_check_generation: exact-head-28032e4
ci_checks_for_current_head: 4
ci_run_ids: [35915719740, 35915719959, 35915719696, 35915719841, 35916776942]
ci_job_ids: [107369795402, 107373242888]
runner_assignment_state: complete
terminal_ci_wait_started_at: null
terminal_ci_checks_for_current_generation: 4
unchanged_state_checks: 0
identical_failure_retries: 0
repair_cycles_for_current_gate: 6
ci_recovery_actions_for_current_head: 6
stall_warnings: 0
owner_action_required: null
blocker: null
next_action: none; archived lifecycle complete
```
