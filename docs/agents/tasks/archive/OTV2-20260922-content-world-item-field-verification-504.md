> **CORRECTION — exact protected #770 final head supersedes the original pre-final counts below.**
> Protected final head `8ac4c20dd6d2790592f7c0a0cd921733c3157207` passed exact-head Item field verification in run `35773500272`, job `106900759673`.
> Final compiler SHA-256: `b6532886dc224d18024ded38260049ed8dae9bc7429e1a2e387a51d7ae391a86`.
> Final exact-head result: 38,157 identities x 107 atomic field slots = 4,082,799 slots; `CORROBORATED_CURRENT=69`, `OTS_ONLY=30,216`, `CONFLICT=8`, `UNKNOWN=4,052,506`; all 4,082,799 promotion states remain BLOCKED; full output SHA-256 `ad3d3b16801979bd67fbf5e14f1d36f6e321f3526d133b17efa9e49a80d224f2`.
> The earlier `53 / 2,022,321 / 67 / 29,307 / 322` counts in this archived historical body came from a pre-final authoring head and are retained below only as provenance, not current protected truth.
>
> Lifecycle closeout: **COMPLETED / ARCHIVED / OWNERSHIP RELEASED**. Canonical PR #770 merged through governed Merge Queue from frozen head `8ac4c20dd6d2790592f7c0a0cd921733c3157207` as protected `main@d72bb070b91aa86ffce1dc610dfa89710be42792`. Real merge-group `35774787270` and aggregate `game-gate` job `106908205635` completed SUCCESS. Exact-head field verification closed 38,157 identities x 53 logical field slots = 2,022,321 slots: CORROBORATED_CURRENT=67, OTS_ONLY=29,307, CONFLICT=322, UNKNOWN=1,992,625; all promotion states remained BLOCKED because target continuity was not yet proven/derived. No Crystal/B1 reimport, identity regeneration, semantic promotion, runtime/client/schema mutation or second Item system occurred. Writer custody is released. The next programme gate is target continuity evidence, then semantic promotion only for eligible atomic fields.
# OTV2-20260922-content-world-item-field-verification-504

~~~yaml
task_id: OTV2-20260922-content-world-item-field-verification-504
title: Item field verification and continuity rule engine
mode: IMPLEMENT
status: validating
repository: Oteryn/Oteryn-Game
base_branch: main
branch: agent/content-world-item-field-verification-504
pr: 770
base_sha: b55f9ade00a47fb0ee2d781fb24ed46e09a34e85
head_sha: pending
final_head_sha: null
final_head_frozen_at: null
owner: "single autonomous Item content implementation agent"
created_at: 2026-09-22T18:46:38Z
updated_at: 2026-09-22T18:46:38Z
execution_policy: continuous_progress
owned_paths:
  - tools/reference-world-corridor-census/item_field_verification.py
  - tools/reference-world-corridor-census/item_field_verification_self_test.py
  - docs/agents/evidence/OTV2-20260922-content-world-item-field-verification.json
  - docs/agents/tasks/active/OTV2-20260922-content-world-item-field-verification-504.md
  - .github/workflows/item-content-verification.yml
public_contracts:
  - docs/contracts/REFERENCE_EVIDENCE_PARITY_MANIFEST_V1.json
  - docs/architecture/OTERYN_REFERENCE_ITEM_ARTIFACT_RESOURCE_PROFILE_V1.md
depends_on:
  - "#749 protected Item schema/resource profile"
  - "#763 protected 38157 Item identity/classification crosswalk"
  - "#767 protected current-source TibiaWiki successor"
  - "#768 protected lifecycle closeout"
blocks:
  - ITEM_SEMANTIC_PROMOTION
cross_repository_coordination_id: null
external_repositories: []
~~~

## Outcome

Produce one deterministic, fail-closed atomic Item field verification/rule-engine result over all 38,157 protected identities without re-importing Crystal/B1, reminting identities, or promoting unqualified gameplay values.

The full 38,157 result remains a reproducible scratch artifact. Git retains only the compact evidence manifest, compiler/self-test and lifecycle packet.

## Architecture and source of truth

- PROVEN: protected Item identity denominator is exactly 38,157 and must be consumed rather than regenerated.
- PROVEN: #749 already supplies the canonical typed Item path and the 90-entry B1 source-field -> typed destination matrix.
- PROVEN: #763 source observations are OTS_HYPOTHESIS_ONLY; OTS agreement is not Reference truth.
- PROVEN: #767 current source is TIBIAWIKI_STRUCTURED / STRUCTURED_REFERENCE_DATA; identity is never resolved from name/title alone.
- PROVEN: Reference target remains global-tibia-observable-2026-07-28-post-server-save; current September evidence does not automatically equal target truth.
- PROVEN: unresolved fields may terminally remain UNKNOWN, CONFLICT, NOT_APPLICABLE, OTS_ONLY or an explicit Oteryn difference according to evidence.
- DERIVED: a sparse per-source-profile rule representation plus per-identity current-source overrides can assign a deterministic disposition to every identity/field slot while keeping absent observations explicitly UNKNOWN without materializing a second semantic graph.

## High-risk authority/recovery qualification

~~~yaml
applicable: false
model: NOT_APPLICABLE
authority_invariants: []
consumer_boundaries:
  - evidence compiler only
mutation_operators:
  applicable: []
  considered_not_applicable:
    - runtime authority
    - persistence authority
    - session or lease replacement
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

This generation has no production mutation authority; it compiles evidence dispositions only.

## Acceptance criteria

- [ ] Exactly 38,157 protected identities join #763 and #767 by exact native key + source numeric ID.
- [ ] Every admitted logical field has exactly one deterministic effective field state and target-continuity state; absence is UNKNOWN, never false/zero.
- [ ] Field states are restricted to CONFIRMED_CURRENT, CORROBORATED_CURRENT, OTS_ONLY, CONFLICT, UNKNOWN, NOT_APPLICABLE, DECLARED_OTERYN_DIFFERENCE.
- [ ] Continuity states are restricted to PROVEN, DERIVED, UNKNOWN, CONFLICT.
- [ ] OTS-only values, unresolved conflicts and ordinary unknowns are never promotion-eligible.
- [ ] A current value is never target-eligible unless continuity is separately PROVEN or DERIVED.
- [ ] Name/title alone never binds an Item or its field values.
- [ ] Existing #749 typed destinations are consumed; no second schema/parser/semantic graph is introduced.
- [ ] Compiler is deterministic/idempotent and full output is byte-identical on unchanged inputs.
- [ ] Compact committed manifest records exact counts/digests and states that semantic promotion was not performed.
- [ ] Focused self-test and hosted qualification are green on the exact candidate head.

## Excluded scope

No Crystal/B1 re-import, identity allocation, family-scale minting, canonical Item schema widening, semantic promotion, server/client artifact mutation, runtime consumer, native-client consumer, protocol, persistence, registry or gameplay-balance mutation.

## Implementation / findings

Initial implementation adds a pure evidence/rule compiler plus synthetic self-test. The compiler reuses the exact protected #749 90-field destination matrix and #767's already-admitted simple current-vs-B1 comparison surface. It does not widen #767's comparison vocabulary.

WIKI_AMBIGUOUS and WIKI_NOT_FOUND bind no page fields. WIKI_CONFLICT can mark only explicitly contradicted non-name comparable signals as field conflicts. A matched current value can be CORROBORATED_CURRENT, but remains promotion=BLOCKED while #767 target continuity is UNKNOWN.

Hosted full-corpus qualification on pre-final authoring head `2618818df5fdfe7351c5ba4e0478fbd9200a1d38` passed in run `35771517554`, job `106894516735`. It reproduced the protected 38,157 identity map and #763 crosswalk exactly, then collected a fresh current-source observation under the protected #767 collector. The fresh observation preserved the same identity/disposition counts while its mutable web snapshot digest differed from protected #767; both digests are retained separately. Full field verification closed 2,022,321 slots (38,157 x 53): CORROBORATED_CURRENT=67, OTS_ONLY=29,307, CONFLICT=322, UNKNOWN=1,992,625, with all 2,022,321 promotion states BLOCKED. Compact manifest is retained under `docs/agents/evidence/OTV2-20260922-content-world-item-field-verification.json`.

## Validation

### Focused

- command/run: python tools/reference-world-corridor-census/item_field_verification_self_test.py
- result: PASS on pre-final authoring head `2618818df5fdfe7351c5ba4e0478fbd9200a1d38`; exact-final rerun required after evidence/task metadata commit

### Component/integration

- command/run: regenerate protected identity map + #763 crosswalk + #767 current-source scratch, then compile full Item field verification
- result: PASS on run `35771517554` / job `106894516735`; exact-final rerun required after evidence/task metadata commit

### E2E

- scenario: NOT_APPLICABLE for this evidence-only generation; programme E2E remains downstream
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
- related/superseded PRs: #749, #763, #767, #768 protected predecessors
- protected auto-merge: pending
- merge commit/result: pending
- ownership release: pending

## Context checkpoint

~~~yaml
last_progress: full 38157 field verifier PASS on pre-final authoring head; compact evidence committed, exact-final rerun pending
status: validating
branch: agent/content-world-item-field-verification-504
head_sha: pending
pr: 770
final_head_sha: null
final_head_frozen_at: null
ci_trigger_source: pull_request
ci_check_generation: null
ci_checks_for_current_head: 0
ci_run_ids:
  - 35771517554
ci_job_ids:
  - 106894516735
runner_assignment_state: repository-selected
terminal_ci_wait_started_at: null
terminal_ci_checks_for_current_generation: 0
unchanged_state_checks: 0
identical_failure_retries: 0
repair_cycles_for_current_gate: 0
ci_recovery_actions_for_current_head: 0
stall_warnings: 0
owner_action_required: NONE
blocker: null
next_action: fresh-read final authoring head, rerun required exact-head CI/review, then freeze only if all gates pass
~~~
