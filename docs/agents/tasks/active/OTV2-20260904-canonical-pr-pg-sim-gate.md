# OTV2-20260904-canonical-pr-pg-sim-gate

```yaml
task_id: OTV2-20260904-canonical-pr-pg-sim-gate
title: Canonicalize PR PostgreSQL and simulation exact-head gates
mode: GOVERNANCE
status: BLOCKED_SCOPE
repository: Oteryn/Oteryn-Game
base_branch: main
branch: ci/canonical-pr-pg-sim-279
pr: 287
issue: 279
parent_issue: 277
base_sha: d8e6233fa6b6b06f9ef643d5fdd9083d7bb3314d
head_sha: null
final_head_sha: null
final_head_frozen_at: null
owner: Codex single mutating writer authorized by owner continuation
created_at: 2026-09-04T13:56:00Z
updated_at: 2026-09-04T22:25:55Z
execution_budget_minutes: 60
large_budget_reason: required-check composition with hosted RED/GREEN and review-repair evidence
owned_paths:
  - .github/workflows/merge-gate.yml
  - tools/repository/validate_repository_policy.py
  - tools/repository/validate_pr_gate_pg_sim.py
  - tools/repository/test_validate_pr_gate_pg_sim.py
  - docs/agents/BUILD_TEST_MATRIX.md
  - docs/agents/tasks/active/OTV2-20260904-canonical-pr-pg-sim-gate.md
public_contracts:
  - canonical PR game-gate composition
depends_on:
  - issue: 277
  - issue: 278
    state: completed
    protected_main_readback: 68ecbad7f6a0dbe7d6214654f8a57c75a3d7c705
blocks:
  - issue: 283
cross_repository_coordination_id: null
external_repositories: []
```

## Outcome and authority

Every applicable PR Linux job executes real PostgreSQL durability tests and every applicable Windows job executes deterministic simulation. Removing or renaming the allocated PostgreSQL target fails closed. Historical pre-allocation absence is explicit `NOT_APPLICABLE`, never DB E2E PASS.

Protected main at this repair admission is `68ecbad7f6a0dbe7d6214654f8a57c75a3d7c705`; the existing branch includes it. Original admission provenance remains unchanged above. PR #252 is integrated and the PostgreSQL target exists on both protected main and this candidate.

The owner explicitly authorized this session to take over repair and qualification of #287 and #291, still without merge. One writer handles the branches serially because they share the repository-policy wrapper. Read-only #291 inspection can run independently. This grants no production, protected-setting, external-repository or merge authority.

## Scope and invariants

The six-path Issue #279 allocation is unchanged. Existing Rust path applicability, dependency review, CodeQL, supply chain, Linux/Windows validation and aggregate fan-in remain intact. No `rust.yml`, merge-group gate, protected audit or ruleset mutation belongs to this task. #284/#285 separately own merge-group gate strengthening.

The upstream scope now re-reads the PR after file enumeration and rejects changed state/head/repository/base SHA/base ref/count before publishing any routing output. Stable Rust path classification is preserved. The current-PR target classifier consumes exact expected head and base SHA, validates initial identity, enumerates all file pages, and re-reads the PR before emitting any result. Changes to open state, head, repository, base or file count fail closed. The PG/SIM evidence steps remain unconditional inside their applicable Rust jobs.

## TDD lineage and findings

- RED1 `95812aaffe88974958b73803760e070e8c2abe2b`: canonical PG/SIM contracts absent.
- Initial runtime GREEN `fe8e76c617472b6281e519647cc099ebc7b7d1ad`: Windows simulation passed; historical PG absence correctly surfaced.
- RED2 `891adbf70723ef5f558e15aa69e58ce1a6c957a1`: missing deletion/rename routing contracts.
- GREEN2 `2ac8ac57d75310510f56e4426cf3cd5e5cfc7113`, later normally reconciled through `f2d5bf340e3e5c256424f017275bcac66be33460`.
- P1 `3936176055` accepted: substring checks permitted skipped evidence steps. RED3 `8636afc54da0c9a900aca1a37a490432cf764c87` independently failed PostgreSQL and simulation skip regressions. GREEN3 `e33bcabf3a0710ff0857addcd3691f0fde3abd8b` added scoped unconditional-step validation.
- P2 `3936294911` accepted: mutable file enumeration was not revalidated. Published RED4 `445bc91de4c4b0f8c9415e194c6ea2ea06c6b947` failed 17 exact-base/post-enumeration contracts in hosted job `101111526699`.
- RED4 was reproduced locally on the clean exact branch. New behavioral regressions execute the actual embedded classifier, replacing only GitHub HTTP responses. Before GREEN it emitted a result for all five independently changed post-enumeration fields (closed/head/repository/base/count) and accepted an invalid or mismatched expected base. Stable non-removal/removal/rename controls passed.
- GREEN4 adds expected-base input/validation and the post-enumeration identity check before output, preserving classification behavior for stable inputs. The behavioral regressions and all focused validators pass locally. The required governance job now executes the focused regressions.
- GREEN4 `480e376b37c71c407d27902d3ec81387f9526711` passed exact-head canonical run `33924254526`, real PostgreSQL 17.6 (115/115) and Windows SIM (7/7). Independent review `PRR_kwDOT8SzxM8AAAABMRK09Q` then found upstream scope P1 `3938249542`; prior self-review was incomplete at this consumer boundary.
- RED5 `f8c6d86d37996e5a6084024d04c4d8424b6c0528` executes the actual upstream scope script: it emitted routing authority after each of six post-enumeration identity mutations. Existing six regression functions still passed. GREEN5 adds the same before-output revalidation at scope, with an additional main-base-ref check, and preserves stable docs/Rust/rename routing.
- P2 `3936176060` accepted and fixed: this task and BUILD_TEST_MATRIX now identify #252 as integrated and real PostgreSQL as applicable.

## AuthorityInvariant × ConsumerBoundary × MutationOperator sweep

| Invariant | Consumer boundary | One-field mutation / evidence |
| --- | --- | --- |
| PR remains open | upstream scope and PG target classification before output | closed state after two file pages is rejected |
| Head remains exact | same boundary | moved head with unchanged count is rejected |
| Repository remains the same | same boundary | changed head repository is rejected |
| Base remains exact | both classifiers after enumeration; PG initial admission | missing/malformed expected base, initial mismatch and later base change are rejected |
| Enumeration count remains bound | both classifiers post-enumeration | changed count is rejected |
| Main remains the scope target | upstream scope before routing | changed base ref rejected |
| Rust lanes cannot be omitted using another head | upstream scope | 101-file docs listing with moved event head rejected; stable docs/Rust/rename controls pass |
| Allocated target cannot become an absence skip | classification / Linux E2E | stable removal and rename produce `removed=true`; existing workflow fails missing removed target |
| Applicable evidence runs | Linux PG / Windows SIM | unquoted/quoted `if` and `continue-on-error` mutations rejected for both steps |
| Stable input remains accepted | classifier happy paths | unchanged target, historical empty diff, deletion and rename outputs checked independently |
| Exact checkout and aggregate remain enforced | existing jobs / final game-gate | unchanged source and core policy validation |

The race fixture uses 101 files across two pages; head/base/repository/state mutations preserve the file count. It does not infer correctness from strings alone. The controlled HTTP seam models the accepted before/after identity contract; this is not a claim of an atomic snapshot from GitHub's mutable files API.

Fenced Game writes, game-session authority, replay and runtime concurrency are `NOT_APPLICABLE` to this workflow-only repair. Real PostgreSQL and simulation execution are still required hosted predicates.

## Acceptance and validation

- [x] Published RED4 and fresh local behavioral RED observed before workflow repair.
- [x] Minimal classifier GREEN preserves original positive behavior and fails closed for the accepted race family.
- [x] `python3 tools/repository/test_validate_pr_gate_pg_sim.py`: 8 test functions PASS, including both upstream and downstream post-enumeration mutation families, base failures, stable scope/target controls and both skip-condition families.
- [x] `python3 tools/repository/validate_pr_gate_pg_sim.py`: PASS.
- [ ] Repository policy currently FAILS only because the repaired scope differs from EXPECTED_MERGE_GATE_SCOPE_JOB_SHA256 in unallocated tools/repository/validate_repository_policy_core.py. Owner scope expansion is required before changing that pin; no bypass or wrapper suppression is permitted.
- [x] `python3 tools/agents/validate_governance.py`: PASS.
- [x] Whole-diff/family self-review and `git diff --check`: PASS before publication.
- [ ] Fresh published-head canonical game-gate including real PostgreSQL 17.6 and Windows simulation.
- [ ] One independent deep review on the stable repaired material candidate; accepted material findings repaired if any.
- [ ] Native review-thread dispositions and final exact-head readback; no merge by this writer.

## Self-review and independent review

The implementation revalidates both mutable-files consumers (scope routing and PostgreSQL target classification), adds executable regression coverage to the existing governance job, and reconciles metadata. Aggregate predicates, path applicability, permissions, service pin and PG/SIM commands are unchanged. The full diff remains inside the six allocated paths. The independent upstream-scope P1 is accepted and repaired; a fresh whole-diff/family sweep includes both output-producing consumers. Final current-head qualification and review remain pending.

The current META AI review policy selects one deep review because this is required-check control-plane behavior. Deterministic hosted validation must pass first. No final review is claimed yet. Review/head/run/READY bookkeeping after this material commit belongs on Issue/PR, not in a new bookkeeping-only commit. `final_head_sha` is resolved externally after publication, not self-referentially embedded here.

## Context checkpoint

```yaml
last_progress: GREEN5 upstream scope revalidation and eight regression functions pass after witnessed RED5
status: BLOCKED_SCOPE
branch: ci/canonical-pr-pg-sim-279
head_sha: null
pr: 287
final_head_sha: null
final_head_frozen_at: null
ci_trigger_source: pull_request
ci_check_generation: review_repair_green5
ci_checks_for_current_head: pending_publication
ci_run_ids: []
ci_job_ids: []
runner_assignment_state: pending_publication
terminal_ci_wait_started_at: null
terminal_ci_checks_for_current_generation: 0
unchanged_state_checks: 0
identical_failure_retries: 0
repair_cycles_for_current_gate: 5
ci_recovery_actions_for_current_head: 0
stall_warnings: 0
owner_action_required: null
blocker: owner scope expansion required for tools/repository/validate_repository_policy_core.py exact scope digest update
next_action: obtain evidence-backed owner scope expansion for the core validator, update only the exact repaired-scope digest, rerun full deterministic qualification and one material-repair deep review, then dispose findings and return without merge
```
