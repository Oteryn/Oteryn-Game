# OTV2-20260906-durable-fresh-resource-envelope-337

```yaml
task_id: OTV2-20260906-durable-fresh-resource-envelope-337
title: Bound complete durable fresh records and retained asynchronous work
mode: CONTRACT
status: validating
repository: Oteryn/Oteryn-Game
base_branch: main
branch: arch/durable-fresh-resource-envelope-337
issue: 337
pr: null
base_sha: 1bcdc951e90a56310d24dfb5f3953ec0f86e1695
admission_main_sha: 1bcdc951e90a56310d24dfb5f3953ec0f86e1695
head_sha: null
final_head_sha: null
final_head_frozen_at: null
owner: Oteryn Sol Supervising Architect
coordinator: Oteryn Work Delivery Coordinator
created_at: 2026-09-06T08:51:28Z
updated_at: 2026-09-06
execution_budget_minutes: 30
execution_window_started_at: 2026-09-06T08:51:28Z
large_budget_reason: null
owned_paths:
  - docs/architecture/reviews/OTERYN_GAME_DURABLE_FRESH_RESOURCE_ENVELOPE_DECISION_2026-09-06.md
  - docs/agents/tasks/active/OTV2-20260906-durable-fresh-resource-envelope-337.md
public_contracts: [DUR-FRESH-RESOURCE-ENVELOPE-V1]
depends_on: [accepted_fresh_claim_contracts, accepted_B_allocation]
blocks: [durable_fresh_resource_acceptance]
cross_repository_coordination_id: null
external_repositories: []
```

## Outcome

A minimal candidate resource envelope covers complete B operations, typed guard/history/lifecycle records, row mirrors, SQL result/lock footprints, queued work and end-to-end submission/completion/ambiguity custody. Numeric maxima are explicit first-slice policy proposals with costs and unknowns. No runtime or registry is edited. Work alone publishes, independently qualifies and integrates; exact native head is recorded after publication rather than manufactured in its own bytes.

## Architecture and source of truth

Issue #337 and Work comment `5558133186` grant exactly these two docs. Live protected main, branch and clean isolated worktree were verified at immutable `1bcdc951e90a56310d24dfb5f3953ec0f86e1695`. Root/docs instructions, task template, supervising architect prompt, architecture decision discipline, current Foundation status/refinements, registry, accepted fresh/claim decisions and B plan were read. GitHub facts outrank old programme summaries.

PROVEN: Foundation complete operation retains four expected guards and two predecessor/successor records; registry has no exact B record/async mappings; accepted B requires complete codec and bounded async work. B worker supplied measured small fixture sizes and current copy mechanics from its mutable worktree based on `d371e0a`; these are supplementary implementation evidence, not immutable accepted maxima. DERIVED: wire-frame and reconnect-attempt ceilings do not close this gap. UNKNOWN: representative production field distributions, full lifecycle/SQL/queue peak allocations and throughput. CONFLICT: none after separating candidate policy, measurements and accepted semantic authority.

## High-risk authority/recovery qualification

```yaml
applicable: true
model: AuthorityInvariant_x_ConsumerBoundary_x_MutationOperator
authority_invariants: [complete_lossless_operation, bounded_allocation, atomic_effects, exact_original_identity, current_authority, retained_ambiguity, restart_custody, durable_nonrollback]
consumer_boundaries: [encode_decode, guard_publication, SQL_read_write, enqueue, final_L, completion_adoption, reconcile, lifecycle, restart]
mutation_operators:
  applicable: [max_plus_one, checked_arithmetic_overflow, excess_vector_count, deep_clone_amplification, mirror_size_bypass, result_size_TOCTOU, full_queue, stalled_SQL, unconsumed_completion, cancellation, lost_commit_response, new_executor_identity, stale_current_facts, changed_original_operation]
  considered_not_applicable: [compression_excluded, history_retention_policy_excluded, production_capacity_unauthorized]
one_invariant_per_negative_case: required_for_future_implementation
independent_current_fact_sources: [registered_Game_owners, authenticated_Platform_provenance, fenced_durable_current_rows, original_immutable_operation_for_expected_binding_only]
record_derived_matching_helper:
  allowed_for_positive_happy_path: historical_replay_only
  forbidden_for_negative_authority_or_provenance_cases: true
finding_family_sweep:
  sibling_apis: fresh_publication_release_replacement_claim_preserving_writes
  protocol_versions: existing_V1_V2_semantics_preserved
  direct_and_reconciled_paths: same_original_operation_and_shared_budget
  fenced_durable_writes: no_partial_effects_on_precommit_exhaustion
  restart_retry_replay_concurrency_pg_reload: explicit_future_matrix
  evidence: [full_candidate_adversarial_self_review, B_codec_and_registry_readback]
finding_dispositions:
  p0_p1_accepted_and_repaired: []
  p0_p1_rejected_with_exact_evidence: []
  p2_fixed_accepted_or_deferred: [P2_accepted_FND02_nested_counts_vs_SQL_row_key_limits_fixed]
```

## Acceptance criteria

- [x] Verify actual resource gap against accepted B/registry/Foundation, with B fixture/copy evidence labeled precisely.
- [x] Propose complete finite count/byte ceilings, units, inclusive arithmetic, pre-allocation checks and all-copy accounting.
- [x] Retain ambiguity, completion and restart custody without new authority or unbounded detached work.
- [x] Preserve history, original L, provenance, sealed claims, lifecycle and post-grace decisions; separate registry followup and implementation allocation.
- [x] Local governance/whitespace and whole-candidate self-review.
- [ ] Independent exact candidate review, selected CI and protected integration under Work.

## Excluded scope

No B/Foundation/registry/JSON/runtime/SQL/migration/schema/dependency/workflow/external/production changes. No PR/post/local commit/merge/global overlays or lifecycle closeout. No new history horizon, deployment sizing, source transport, lease or grace policy. #336 qualification and B parameterized work proceed independently. The candidate does not release ServerSeam or supply missing authenticated bootstrap.

## Implementation / findings

One bounded authoring window starts at the verified time above. Exactly one writer owns this worktree. Serial two-doc authoring is required by shared candidate/task custody; B implementation and independent #336 review continue in parallel. Limits distinguish complete operation, individual guard, complete row, per-pass SQL/key work and all retained asynchronous work. Two active slots persist through checkpoint ambiguity, SQL, completion and original-operation reconciliation; no timeout/disconnect frees uncertain work. Durable history remains outside resident-slot retention and is never evicted to meet these limits.

The candidate uses measured fixture sizes only as scale context. Codec and runtime measurements remain a future acceptance obligation; a 12 MiB charged-work ceiling is not measured process RSS or a safe production throughput claim. All numeric subdimensions require separate exact registry integration after decision acceptance. No unrelated Foundation transport ceiling is imported.

## Independent finding repair — first cycle

Work accepted the independent P2 (P0=0/P1=0): blanket row/key collection wording and a 32-row result cap conflicted with the actual sibling mirror readers' 64 pending-command rows and risked shrinking the accepted 256-domain vector. Source verification confirmed both functions and Foundation maxima. Repair preserves 64/256, requires complete bounded per-attempt SQL aggregation under the same protected snapshot, charges every payload byte/element, and requires shared parent fencing across all child writers with required physical locks before L. Eight attempts can retain 512 pending children without 512 returned rows or extra advisory keys. The actual 14-table baseline plus a pending checkpoint relation fits 16; exact B key/table inventory remains mandatory.

The family sweep covers both mirror APIs, full legacy decode/reload/reconcile, V1/V2 and eight-attempt pending state. It explicitly prevents treating eight attempts per epoch as a lifetime history cap or narrowing the inherited cross-epoch predecessor mutation. Required positive cases include all 64 pending and 256 domain elements together, with 65/257 negatives and child phantom/substitution races. Numeric byte/queue/active maxima and authority are unchanged. Full repaired candidate requires independent re-review. The original 08:51:28Z–09:21:28Z authoring window and cumulative counters are preserved.

## Validation

### Focused

- command/run: `python tools/agents/validate_governance.py`; `git diff --check`; `git diff --cached --check`
- result: PASS on the reviewed staged candidate; no implementation test is claimed.

### Component/integration

- command/run: NOT_APPLICABLE — exactly two documentation files; future codec/resource/SQL matrix is in the decision.
- result: No runtime or SQL result claimed. Actual future PostgreSQL cases must execute in enforced `--test durability_postgres`, not merely exist under cfg(test).

### E2E

- scenario: NOT_APPLICABLE — no runtime or production mutation.
- result: No ServerSeam, actual source availability, capacity or production proof.

### Exact-head CI

- final head: Pending Work native publication of these staged bytes.
- trigger source: Work-owned ordinary PR/check lifecycle.
- workflow/run/job: Pending; no CI triggered by this author.
- runner assignment: Not assigned by this task.
- classification: Documentation candidate awaiting independent qualification.
- result: Not claimed.

## Self-review

- exact head: Immutable admission above plus exact staged two-file candidate; tree returned separately to Work without a local commit.
- method/reviewer: Author mandatory adversarial whole-candidate review, checking source and task together.
- material findings: Independent P2 accepted and fixed in cycle 1; no unresolved P0/P1/P2 identified in whole repaired-candidate self-review. Reviewed complete/mirror/nested bytes, validation clones, SQL-before-transfer checks, queue-to-active overlap, unconsumed completion, checkpoint/restart ambiguity, immutable history, lifecycle siblings and no invented authority.
- verdict: Ready for Work publication and independent review; not canonical acceptance.

## Independent review

- required: YES — material durability resource and recovery custody contract.
- exact head: Pending native publication.
- method/auditor: Separate reviewer selected by Work; author has no integration authority.
- material findings: Initial independent P0=0/P1=0/P2=1; accepted P2 repaired.
- verdict: Full repaired-candidate re-review pending.

## PR and closeout

- changed-file review: Exactly the two allocated docs; no additional path needed.
- unresolved review threads: Independent review not yet begun for this candidate.
- related/superseded PRs: B and #336 remain independently allocated; no PR superseded here.
- protected auto-merge: Work only.
- merge commit/result: Pending; not claimed.
- ownership release: Staged byte custody returns to Work at handoff; author performs no commit or publication.

## Context checkpoint

```yaml
last_progress: Two-doc candidate self-reviewed and locally validated for native publication
status: validating
branch: arch/durable-fresh-resource-envelope-337
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
repair_cycles_for_current_gate: 1
ci_recovery_actions_for_current_head: 0
stall_warnings: 0
owner_action_required: null
blocker: Independent publication/review/acceptance and separate registry integration remain required
next_action: Work publishes the exact staged two-doc candidate for independent qualification and protected integration.
```
