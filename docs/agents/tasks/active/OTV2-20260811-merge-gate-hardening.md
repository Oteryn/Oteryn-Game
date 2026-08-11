# OTV2-20260811-merge-gate-hardening

```yaml
task_id: OTV2-20260811-merge-gate-hardening
title: Harden PR merge gating and repository engineering drift controls
mode: REPAIR
status: blocked
repository: blakinio/Oteryn-v2
base_branch: main
branch: ci/OTV2-20260811-merge-gate-hardening
pr: 162
base_sha: f184930fac66fdf9ae0cc7f606d3502c17626a79
head_sha: null
final_head_sha: null
final_head_frozen_at: null
owner: ChatGPT repository engineering agent
created_at: 2026-08-11T10:30:00+02:00
updated_at: 2026-08-11T11:28:00+02:00
execution_budget_minutes: 120
large_budget_reason: Repository merge-authority transition plus exact-head Linux/Windows/security validation and independent review repair.
owned_paths:
  - .github/workflows/merge-gate.yml
  - .github/workflows/codeql.yml
  - .github/workflows/dependency-review.yml
  - .github/workflows/rust.yml
  - .github/workflows/rust-cutover-terminal-audit.yml
  - .github/dependabot.yml
  - .github/repository-policy.json
  - .github/CODEOWNERS
  - tools/repository/validate_repository_policy.py
  - tools/repository/apply_github_settings.py
  - docs/repository/GITHUB_GOVERNANCE.md
  - docs/agents/BUILD_TEST_MATRIX.md
  - docs/agents/tasks/active/OTV2-20260811-merge-gate-hardening.md
public_contracts:
  - .github/repository-policy.json
  - docs/repository/GITHUB_GOVERNANCE.md
  - docs/agents/BUILD_TEST_MATRIX.md
depends_on: []
blocks: []
cross_repository_coordination_id: null
external_repositories: []
```

## Outcome

Replace the weak single-purpose required PR check with one stable `Merge gate / validate` context that composes governance, Dependency Review, CodeQL and path-proportional Rust Linux/Windows/policy/supply-chain checks. Repair stale repository-engineering documentation, add Cargo Dependabot, and retire completed/duplicated CI workflows.

## Implemented scope

- Added `.github/workflows/merge-gate.yml` for every PR to `main`, without workflow-level path filters.
- Added bounded `workflow_dispatch` recovery requiring an open PR number, exact full head SHA, dispatch from that unchanged head, same-repository/main-target validation and explicit validated base/head refs.
- Added fail-closed aggregate validation and full Rust gates when the current or rename-source path is Rust/workspace-sensitive.
- Changed retained machine policy to require `Merge gate / validate` after merge.
- Extended `apply_github_settings.py` to verify the live required-status list exactly after policy application.
- Extended repository policy validation for aggregate-gate invariants, exact-head recovery, rename-source classification and documentation alignment.
- Added Cargo Dependabot.
- Refreshed `BUILD_TEST_MATRIX.md`, `CODEOWNERS` and `GITHUB_GOVERNANCE.md` to current repository truth.
- Removed standalone Dependency Review and the completed one-off Rust cutover terminal audit workflow.
- Converted retained Rust/CodeQL workflows to post-merge/manual use to avoid duplicate PR execution once the aggregate gate is active.
- Intentionally retained the old PR `Agent governance / validate` during this transition so the currently live ruleset can validate PR #162 before the new required context is applied.

No gameplay, protocol, persistence, content, Platform, production, deployment or cross-repository authority is changed.

## Repair history

### Cycle 1

Independent review found and the task repaired:

1. P1 — no exact-head manual recovery for the future sole required gate;
2. P1 — incomplete authoritative task/recovery fields;
3. P2 — stale canonical GitHub governance documentation.

### Cycle 2

Independent review of `6244f2a134a791d53ee9ebcd39cab00cf4e3d8db` found and the task repaired:

4. P1 — renamed Rust-sensitive files could evade scope classification because `previous_filename` was ignored.

The gate now classifies both `filename` and `previous_filename`.

### Cycle 3

Exact-head governance on `2283d8a5f1d075d1c68f58221c554c26249ba3c5` proved a validator-only false positive: substring matching for `paths:` also matched the Python identifier `classification_paths:`. The task replaced it with a narrower anchored YAML regex. Merge-gate runtime behavior did not change in this cycle.

Repair budget for this gate is now **3/3 exhausted**.

## Final candidate evidence before blocker

Candidate reviewed head: `af834e10605be7088df444d474d203c9fa3c43eb`.

- branch compare against `main@f184930fac66fdf9ae0cc7f606d3502c17626a79`: `behind_by=0`, exactly 13 intended repository-engineering paths;
- full-diff self-review: PASS, PR comment `5251287815`;
- transition `Agent governance / validate`: PASS, run `31477148834`, job `93733376330`;
- aggregate Merge gate run `31477148862` was pending/queued when the independent final-head blocker arrived and is not merge-readiness evidence;
- final independent review was explicitly requested on `af834e10605be7088df444d474d203c9fa3c43eb`.

## BLOCKER — independent final-head finding after repair budget exhaustion

Independent review of exact candidate `af834e10605be7088df444d474d203c9fa3c43eb` produced a new material P2 on `tools/repository/validate_repository_policy.py`:

> the static validator rejects `paths` / `paths-ignore` only for one exact indentation/layout, while valid YAML can represent the `pull_request` mapping with different indentation/formatting; a future path-filtered merge gate could therefore escape this guard.

Thread: `PRRT_kwDOTuGrds6YLEL3`.

**Classification: VALID MATERIAL GOVERNANCE FINDING.**

The current `merge-gate.yml` itself has no path filter, so this is not evidence of a current runtime bypass. It is evidence that the repository's anti-regression validator is insufficiently representation-independent for the future sole merge-authority workflow.

Repository anti-stall policy allows at most three repair cycles for one gate. The task has consumed `3/3`. A fourth routine patch would violate the governing execution budget and is therefore not authorized by this task.

PR #162 was converted back to **draft** and remains unmerged. Historical material review threads remain unresolved intentionally.

## Acceptance status

- [x] aggregate merge gate implemented;
- [x] exact-head recovery implemented;
- [x] dependency/security/Rust sub-gates composed;
- [x] rename-source classification implemented;
- [x] Cargo Dependabot added;
- [x] stale developer/repository documentation repaired;
- [x] old/duplicate CI workflow cleanup implemented;
- [x] final candidate self-review PASS;
- [x] transition governance PASS on final reviewed candidate;
- [ ] independent final-head review clean — **BLOCKED by P2 thread `PRRT_kwDOTuGrds6YLEL3`**;
- [ ] aggregate final-head CI PASS — not sufficient/terminal while blocker exists;
- [ ] PR merge — prohibited while blocker exists;
- [ ] post-merge live ruleset verification — not applicable until merge;
- [ ] task archive/ownership release — prohibited until terminal delivery or explicit abandonment/supersession.

## Required next decision

One of the following must occur before work can proceed:

1. the owner explicitly authorizes a bounded successor repair package / renewed repair budget for the representation-independent YAML path-filter validator; or
2. the owner explicitly abandons/supersedes this merge-gate transition.

A successor repair must not silently reuse this task as a fourth repair cycle. It must preserve PR #162 evidence, address the exact independent finding, receive new exact-head CI and independent review, and only then resume merge readiness.

## Context checkpoint

```yaml
last_progress: Final independent review of candidate af834e10605be7088df444d474d203c9fa3c43eb found a valid P2 in representation-dependent path-filter validation after the task exhausted its 3/3 repair budget. PR #162 was converted to draft and the finding was recorded without a prohibited fourth patch.
status: blocked
branch: ci/OTV2-20260811-merge-gate-hardening
head_sha: null
pr: 162
final_head_sha: null
final_head_frozen_at: null
ci_trigger_source: pull_request/synchronize
ci_check_generation: blocked-after-final-independent-review
ci_checks_for_current_head: 0
ci_run_ids:
  - 31477148834
  - 31477148862
ci_job_ids:
  - 93733376330
runner_assignment_state: not_material_while_blocked
terminal_ci_wait_started_at: null
terminal_ci_checks_for_current_generation: 0
unchanged_state_checks: 0
identical_failure_retries: 0
repair_cycles_for_current_gate: 3
ci_recovery_actions_for_current_head: 0
stall_warnings: 0
owner_action_required: Explicitly authorize a bounded successor repair package / renewed repair budget, or explicitly abandon/supersede the transition.
blocker: Independent final-head P2 thread PRRT_kwDOTuGrds6YLEL3; representation-dependent static detection of pull_request paths/paths-ignore can miss valid YAML forms; repair budget 3/3 exhausted.
next_action: Do not patch or merge PR #162. Await explicit owner authorization for a successor repair package or an explicit abandon/supersede decision.
```
