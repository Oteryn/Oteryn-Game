# OTV2-YYYYMMDD-short-slug

```yaml
task_id: OTV2-YYYYMMDD-short-slug
title: <short title>
mode: IMPLEMENT | AUDIT | CONTRACT | REPAIR | COORDINATE | MIGRATE | GOVERNANCE
status: investigating | implementing | validating | ready | waiting | blocked | completed
repository: Oteryn/Oteryn-Game
base_branch: main
branch: <dedicated branch>
pr: null
base_sha: null
head_sha: null
final_head_sha: null
final_head_frozen_at: null
owner: <agent/session identity>
created_at: <ISO-8601>
updated_at: <ISO-8601>
execution_policy: continuous_progress
owned_paths: []
public_contracts: []
depends_on: []
blocks: []
cross_repository_coordination_id: null
external_repositories: []
```

`execution_policy: continuous_progress` means productive authorized work has no wall-clock stop window. Apply `docs/agents/ANTI_STALL_AND_EXECUTION_BUDGET.md` for no-progress, repeated-failure and CI-wait bounds; do not add `execution_budget_minutes`, `large_budget_reason` or equivalent time-window fields to new task records.

## Outcome

Describe the observable repository/product result, not only files to edit.

## Architecture and source of truth

List accepted ADRs/contracts and exact external revisions. Label material statements `PROVEN`, `DERIVED`, `UNKNOWN` or `CONFLICT`.

## High-risk authority/recovery qualification

For work that performs a production mutation gated by current session, lease, generation, authority or other fence evidence; authorizes PREPARE/COMMIT; installs/restores a controller; replaces an authority-bearing session; or interprets persisted recovery evidence, complete this section before material freeze. Otherwise record `NOT_APPLICABLE` with a concrete reason.

```yaml
applicable: pending
model: AuthorityInvariant_x_ConsumerBoundary_x_MutationOperator
authority_invariants: []
consumer_boundaries: []
mutation_operators:
  applicable: []
  considered_not_applicable: []
one_invariant_per_negative_case: pending
independent_current_fact_sources: []
record_derived_matching_helper:
  allowed_for_positive_happy_path: pending
  forbidden_for_negative_authority_or_provenance_cases: pending
finding_family_sweep:
  sibling_apis: pending
  protocol_versions: pending
  direct_and_reconciled_paths: pending
  fenced_durable_writes: pending
  restart_retry_replay_concurrency_pg_reload: pending
  evidence: []
finding_dispositions:
  p0_p1_accepted_and_repaired: []
  p0_p1_rejected_with_exact_evidence: []
  p2_fixed_accepted_or_deferred: []
```

Immutable prepared/persisted evidence may define the expected binding but is not current authority evidence. Enumerate concrete applicable mutation operators; consider at least missing facts, stale facts/generations, mismatched identity/binding, expired/future/non-monotonic time, provenance substitution and boundary-specific replay/concurrency. Each negative case changes exactly one applicable invariant while keeping unrelated facts semantically valid. Historical terminal outcomes may preserve typed disposition without current live-authority equality, but must never reacquire authority through a weaker compatibility path.

## Acceptance criteria

- [ ] Concrete criterion with named evidence.

## Excluded scope

State what this task must not change or claim.

## Implementation / findings

Maintain concise durable progress and decisions. For applicable high-risk authority/recovery work, complete focused RED → minimal GREEN, deterministic affected validation, the finding-family sweep and adversarial whole-diff self-review before material freeze.

For every material P0/P1 report, first verify applicability and correctness on the exact reviewed head. A verified rejection with exact evidence preserves the frozen candidate and prior representative review; it does not trigger repair, supersession or re-review. Only an accepted/verified material finding supersedes the generation. Repair that finding test-first and expand its family across applicable sibling APIs, versions, direct/reconciled paths, fenced durable writes, restart, retry/replay, concurrent and PostgreSQL reload paths before requesting another deep review. A P2 requires explicit `fixed`, `accepted` or `deferred` disposition. External AI findings are advisory evidence and never merge authority.

Prepare all known closeout metadata before freezing the final head; do not move a frozen head merely to copy review/audit or CI status into this file.

A commit cannot contain its own SHA. Record the final exact head in immutable PR/check evidence after the final commit exists rather than creating a self-referential follow-up commit.

## Validation

### Focused

- command/run: pending
- result: pending

### Component/integration

- command/run: pending or `NOT_APPLICABLE` with reason
- result: pending

### E2E

- scenario: pending or `NOT_APPLICABLE` with reason
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
- method/reviewer: implementing/coordinating agent (mandatory; cannot be delegated away)
- material findings: pending
- verdict: pending

## Independent review

- required: pending (`YES` with reason or `NO` with risk-policy reason)
- exact head: pending or `NOT_APPLICABLE`
- method/auditor: pending or `NOT_APPLICABLE`
- material findings: pending or `NOT_APPLICABLE`
- verdict: pending or `NOT_APPLICABLE`

## PR and closeout

- changed-file review: pending
- unresolved review threads: pending
- related/superseded PRs: pending
- protected auto-merge: pending
- merge commit/result: pending
- ownership release: pending

## Context checkpoint

```yaml
last_progress: <material event>
status: investigating
branch: <branch>
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
blocker: null
next_action: <exactly one concrete action>
```
