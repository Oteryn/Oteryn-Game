# OTV2-20260923-tibiawiki-worldproject-v2-full-coverage

~~~yaml
task_id: OTV2-20260923-tibiawiki-worldproject-v2-full-coverage
title: TibiaWiki full content-schema coverage and WorldProject/v2 closure
mode: IMPLEMENT
status: implementing
repository: Oteryn/Oteryn-Game
base_branch: main
branch: agent/tibiawiki-worldproject-v2-full-coverage-20260923
pr: null
base_sha: aabf0b9a30a6153b7e34d2afedcb1b2b19c27fd6
head_sha: null
final_head_sha: null
final_head_frozen_at: null
owner: "single autonomous TibiaWiki coverage + Content/World schema writer"
created_at: 2026-09-23T18:33:00+02:00
updated_at: 2026-09-23T18:33:00+02:00
execution_policy: continuous_progress
owned_paths:
  - apps/game-server/src/content/project/v2.rs
  - apps/game-server/tests/content_world_project_v2.rs
  - docs/architecture/OTERYN_WORLD_PROJECT_SOURCE_PROFILE_V2_DECISION.md
  - docs/agents/evidence/OTV2-20260923-tibiawiki-worldproject-v2-coverage.json
  - docs/agents/tasks/active/OTV2-20260923-tibiawiki-worldproject-v2-full-coverage.md
public_contracts: []
depends_on:
  - protected WorldProject/v2 from PR #788
  - protected Item authoring completeness from PR #792
blocks:
  - wiki-wide content population/import
cross_repository_coordination_id: null
external_repositories: []
~~~

## Outcome

Close the stalled TibiaWiki audit by assigning every material discovered wiki concept to an existing Oteryn owner or a bounded new WorldProject/v2 typed authoring home, with UNCLASSIFIED=0 in the committed coverage manifest. Implement only representation-level P0/P1 gaps; mutable player/item/world state stays with its runtime/durability owner.

## Architecture and source of truth

- PROVEN: protected main@aabf0b9a30a6153b7e34d2afedcb1b2b19c27fd6 contains canonical WorldProject/v2 and the Item authoring extension.
- PROVEN: v2 declarations are source-only/candidate-only unless an existing Reference/runtime owner explicitly lowers them.
- PROVEN: Character progression, quest progress, inventory/equipment and entitlements are durable Character/Account state; raid/event activity and NPC conversation are runtime owner state; House ownership/rent/access is House-domain state.
- PROVEN: TibiaWiki API access is Cloudflare-blocked from the earlier worker, but rendered category/template/special-page surfaces remain usable for bounded census and exception discovery.
- DERIVED: wiki navigation categories are not automatically Oteryn gameplay families. New types are justified only for reusable static identities/relationships that otherwise cannot be represented without loss.

## High-risk authority/recovery qualification

~~~yaml
applicable: false
model: NOT_APPLICABLE
authority_invariants: []
consumer_boundaries:
  - immutable candidate-only Content/World authoring
mutation_operators:
  applicable: []
  considered_not_applicable:
    - production authority
    - persistence mutation
    - session/lease fencing
    - PREPARE/COMMIT
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

- [ ] Wiki domains/categories/templates/infobox parameter families and exceptional systems are classified in one committed coverage manifest.
- [ ] Every material concept has exactly one Oteryn owner/disposition and UNCLASSIFIED=0.
- [ ] Static document/book identity and Item->Document binding are representable without putting player-written text in WorldProject.
- [ ] Static achievement, charm/progression-definition, outfit/mount, creature/bestiary/bosstiary/familiar, ability, house, quest, encounter/event and recipe facts have a typed or explicitly justified existing home.
- [ ] Mutable current state remains outside WorldProject/v2.
- [ ] No project_v3, second Item/Creature model, arbitrary executable blob or copied copyrighted wiki corpus.
- [ ] Existing v2 canonical bytes remain unchanged when all new optional collections/profiles are absent.
- [ ] Focused tests plus repository-required exact-head PR qualification pass.
- [ ] Protected integration uses only the bound Merge Queue route; no direct merge or generic auto-merge substitute.

## Excluded scope

No bulk copying of TibiaWiki prose/assets, no full content population, no new runtime progression/commerce/house/event authority, no mutable ItemInstance/Character state, no balance changes, no protocol/client changes, no production mutation.

## Implementation / findings

The earlier checkpoint is accepted only as a partial seed. Continuation expanded discovery through rendered Special:Categories/AllPages, core infobox/template families and modern system pages. Confirmed material gaps include static Document content identity/binding, Achievement definition, Charm definition, typed Creature/Ability authoring overlays, Outfit/Mount definitions, immutable House/Quest/Encounter profiles and typed crafting recipes. Bestiary/Bosstiary/Familiar are modeled as overlays of existing Creature rather than duplicate creature families. Mutable progress/assignments/current event state are routed to Character/Account/House/Event owners.

Integration capability preflight: this session exposes direct merge and generic auto-merge only; bound META 3.1 requires native exact-head merge-async with merge_action=merge_queue. Therefore implementation/PR qualification may proceed, but terminal integration must remain BLOCKED_CAPABILITY_UNAVAILABLE unless that exact route becomes available.

## Validation

### Focused
- command/run: repository PR gate / affected Rust tests after frozen head
- result: pending

### Component/integration
- command/run: WorldProject/v2 canonical round-trip and strict-validation tests
- result: pending

### E2E
- scenario: NOT_APPLICABLE; candidate-only source schema has no executable runtime lowering
- result: NOT_APPLICABLE

### Exact-head CI
- final head: pending
- trigger source: pull_request
- workflow/run/job: pending
- runner assignment: repository-selected
- classification: server/content source
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
- related/superseded PRs: #788 #792 #797
- protected auto-merge: forbidden substitute; native Merge Queue only
- merge commit/result: pending
- ownership release: pending

## Context checkpoint

~~~yaml
last_progress: dedicated branch allocated from protected main; partial checkpoint accepted as seed; expanded wiki/domain discovery completed
status: implementing
branch: agent/tibiawiki-worldproject-v2-full-coverage-20260923
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
blocker: integration merge-async unavailable in current session only
next_action: author bounded typed WorldProject/v2 coverage closure and tests
~~~
