# OTV2-20260904-canonical-pr-pg-sim-gate

```yaml
task_id: OTV2-20260904-canonical-pr-pg-sim-gate
title: Canonicalize PR PostgreSQL and simulation exact-head gates
mode: GOVERNANCE
status: QUALIFYING
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
updated_at: 2026-09-04T23:05:48Z
execution_budget_minutes: 60
large_budget_reason: required-check composition with hosted RED/GREEN and review-repair evidence
owned_paths:
  - .github/workflows/merge-gate.yml
  - tools/repository/validate_repository_policy.py
  - tools/repository/validate_repository_policy_core.py
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

Issue #279 now allocates seven paths: after the witnessed core-policy failure and exact proposed digest were presented, the owner instructed continuation. The added path is tools/repository/validate_repository_policy_core.py, solely to update the existing canonical scope digest; its enforcement remains unchanged. Existing Rust path applicability, dependency review, CodeQL, supply chain, Linux/Windows validation and aggregate fan-in remain intact. No `rust.yml`, merge-group gate, protected audit or ruleset mutation belongs to this task. #284/#285 separately own merge-group gate strengthening.

The upstream scope now re-reads the PR after file enumeration and rejects changed state/head/repository/base SHA/base ref/count before publishing any routing output. Stable Rust path classification is preserved. The current-PR target classifier consumes exact expected head and base SHA, validates initial identity, enumerates the immutable base/head comparison, and re-reads the PR before emitting any result. Changes to open state, head, repository, base or file count fail closed. The PG/SIM evidence steps remain unconditional inside their applicable Rust jobs.

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
- GREEN5 `10c8813aa718be46591f95ee0317c5a918f3aa2a` passed canonical run `33925804076` including PG 115/115 and SIM 7/7. Review `PRR_kwDOT8SzxM8AAAABMRQeNw` accepted P1 `3938338845`: before/after PR sampling cannot detect A-to-B-to-A substitution.
- RED6 `22ebdd0c893b6c2aa4ab39acce24d011fc331e7f` reproduced ABA at BOTH output boundaries: upstream emitted rust=false and downstream removed=false from substituted mutable files while initial/final metadata matched. GREEN6 enumerates `/compare/{base_sha}...{head_sha}?per_page=1` at both boundaries, preserving metadata rechecks, positive routing and fail-closed count validation. The canonical scope pin is updated under the already-approved core allocation.
- GitHub comparison files are capped at 300; larger PRs and missing/incomplete file arrays fail closed. This deliberate limit is tested at 300/301 and below-cap count mismatch, and documented in BUILD_TEST_MATRIX. See https://docs.github.com/en/rest/commits/commits#compare-two-commits.
- GREEN6 `9017f2179ad8f3ea1b96773e7fad672136879389` passed canonical run `33926994377`, PG 115/115 and SIM 7/7. Review `PRR_kwDOT8SzxM8AAAABMRWEmA` found P2 `3938427638`: quoted job-level continue-on-error escaped the validator.
- RED7 `7932a3533a4a7a25db8a2edaa1b1119d21bfbe62` independently reproduced both quoted forms on both jobs. The renewed family sweep also proved an early PG exit could preserve required strings and pass the old validator. GREEN7 handles quoted keys and pins both complete evidence-job blocks with the existing canonical SHA256 pattern, closing semantic edits that preserve fragments. The workflow itself remains byte-identical to reviewed GREEN6. Twelve regression functions now pass, including job/step forms and early-exit rejection. Future intentional evidence-job changes require reviewed pin updates.
- P2 `3936176060` accepted and fixed: this task and BUILD_TEST_MATRIX now identify #252 as integrated and real PostgreSQL as applicable.

## AuthorityInvariant × ConsumerBoundary × MutationOperator sweep

| Invariant | Consumer boundary | One-field mutation / evidence |
| --- | --- | --- |
| PR remains open | upstream scope and PG target classification before output | closed state after two file pages is rejected |
| Head remains exact | same boundary | moved head with unchanged count is rejected |
| Repository remains the same | same boundary | changed head repository is rejected |
| Base remains exact | both classifiers after enumeration; PG initial admission | missing/malformed expected base, initial mismatch and later base change are rejected |
| Enumeration count remains bound | both classifiers post-enumeration | changed count is rejected |
| Diff provenance is immutable | both classifiers | A-to-B-to-A metadata with substituted mutable files cannot suppress Rust/PG evidence |
| Diff is complete | both classifiers | 300-file stable control passes; 301 files and count mismatch fail closed |
| Main remains the scope target | upstream scope before routing | changed base ref rejected |
| Rust lanes cannot be omitted using another head | upstream scope | 101-file docs listing with moved event head rejected; stable docs/Rust/rename controls pass |
| Allocated target cannot become an absence skip | classification / Linux E2E | stable removal and rename produce `removed=true`; existing workflow fails missing removed target |
| Applicable evidence runs | Linux PG / Windows SIM jobs and steps | unquoted/quoted failure-tolerance and step-condition mutations rejected; early-success PG exit rejected; complete evidence-job blocks pinned |
| Stable input remains accepted | classifier happy paths | unchanged target, historical empty diff, deletion and rename outputs checked independently |
| Exact checkout and aggregate remain enforced | existing jobs / final game-gate | unchanged source and core policy validation |

Historical RED fixtures used 101 files across two mutable pages. Current tests run the same actual scripts with SHA-bound comparison responses; metadata mutations keep unrelated facts valid. The ABA fixture separately supplies wrong mutable files and the correct immutable diff, proving both consumers use immutable provenance. No mutable `/pulls/{number}/files` request remains in either production classifier.

Fenced Game writes, game-session authority, replay and runtime concurrency are `NOT_APPLICABLE` to this workflow-only repair. Real PostgreSQL and simulation execution are still required hosted predicates.

## Acceptance and validation

- [x] Published RED4 and fresh local behavioral RED observed before workflow repair.
- [x] Minimal classifier GREEN preserves original positive behavior and fails closed for the accepted race family.
- [x] `python3 tools/repository/test_validate_pr_gate_pg_sim.py`: 12 test functions PASS, including both classifiers' metadata and ABA mutation families, comparison-cap/count checks, base failures, stable scope/target controls and both skip-condition families.
- [x] `python3 tools/repository/validate_pr_gate_pg_sim.py`: PASS.
- [x] Repository policy PASS after the owner-authorized exact scope digest update. Before expansion it failed only on the old scope pin. No wrapper suppression, bypass, aggregate-pin change or other core-policy modification.
- [x] `python3 tools/agents/validate_governance.py`: PASS.
- [x] Whole-diff/family self-review and `git diff --check`: PASS before publication.
- [ ] Fresh published-head canonical game-gate including real PostgreSQL 17.6 and Windows simulation.
- [ ] One independent deep review on the stable repaired material candidate; accepted material findings repaired if any.
- [ ] Native review-thread dispositions and final exact-head readback; no merge by this writer.

## Self-review and independent review

The implementation binds both file-list consumers to immutable comparison URLs and revalidates their PR metadata (scope routing and PostgreSQL target classification), adds executable regression coverage to the existing governance job, and reconciles metadata. Aggregate predicates, path applicability, permissions, service pin and PG/SIM commands are unchanged. The full diff remains inside the seven allocated paths. The independent upstream-scope P1 is accepted and repaired; a fresh whole-diff/family sweep includes both output-producing consumers. The subsequent ABA finding is accepted and repaired at both consumers. Final current-head qualification and review remain pending.

The current META AI review policy selects one deep review because this is required-check control-plane behavior. Deterministic hosted validation must pass first. No final review is claimed yet. Review/head/run/READY bookkeeping after this material commit belongs on Issue/PR, not in a new bookkeeping-only commit. `final_head_sha` is resolved externally after publication, not self-referentially embedded here.

## Context checkpoint

```yaml
last_progress: GREEN7 whole evidence-job pinning and quoted failure-tolerance guards pass twelve regressions after witnessed RED7
status: QUALIFYING
branch: ci/canonical-pr-pg-sim-279
head_sha: null
pr: 287
final_head_sha: null
final_head_frozen_at: null
ci_trigger_source: pull_request
ci_check_generation: review_repair_green7
ci_checks_for_current_head: pending_publication
ci_run_ids: []
ci_job_ids: []
runner_assignment_state: pending_publication
terminal_ci_wait_started_at: null
terminal_ci_checks_for_current_generation: 0
unchanged_state_checks: 0
identical_failure_retries: 0
repair_cycles_for_current_gate: 7
ci_recovery_actions_for_current_head: 0
stall_warnings: 0
owner_action_required: null
blocker: null
next_action: qualify the published repair with exact-head PG/SIM game-gate and one material-repair deep review, then dispose findings and return without merge
```
