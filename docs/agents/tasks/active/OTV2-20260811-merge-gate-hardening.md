# OTV2-20260811-merge-gate-hardening

```yaml
task_id: OTV2-20260811-merge-gate-hardening
title: Harden PR merge gating and repository engineering drift controls
mode: REPAIR
status: validating
repository: blakinio/Oteryn-v2
base_branch: main
branch: ci/OTV2-20260811-merge-gate-hardening
pr: 162
base_sha: c94f331ec20849e8306875a2f88b87fa7e1974f9
head_sha: null
final_head_sha: null
final_head_frozen_at: null
owner: ChatGPT repository engineering agent
created_at: 2026-08-11T10:30:00+02:00
updated_at: 2026-08-11T10:57:00+02:00
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

Make one always-present `Merge gate / validate` check the protected-branch merge authority for pull requests, with governance, dependency review, CodeQL and path-proportional Rust validation composed behind it. Provide bounded exact-head dispatch recovery for a suppressed initial PR event, remove stale workspace/bootstrap wording, add Cargo dependency maintenance and retire the completed one-off Rust cutover audit workflow.

## Architecture and source of truth

- `PROVEN`: the live `Protect main` ruleset at task start required only `Agent governance / validate`.
- `PROVEN`: Rust, CodeQL and Dependency Review validation existed separately but was not represented by the protected required context.
- `PROVEN`: `main` already contains the canonical root Rust workspace, while `BUILD_TEST_MATRIX.md` and CODEOWNERS retained bootstrap/future-workspace wording.
- `PROVEN`: `.github/dependabot.yml` covered GitHub Actions but not Cargo.
- `DERIVED`: one stable aggregate required context avoids required-check deadlocks from path-proportional jobs while still failing unless every applicable sub-gate passes.
- `DERIVED`: because the aggregate becomes the sole required context, it must retain a trusted exact-head dispatch recovery path equivalent in safety to the existing governance recovery path.

## Acceptance criteria

- [x] one PR workflow always emits `Merge gate / validate`;
- [x] governance, dependency review and CodeQL are mandatory sub-gates;
- [x] Rust Linux/Windows/supply-chain checks run when Rust/workspace-relevant paths change;
- [x] repository policy requires only the stable aggregate merge-gate context;
- [x] repository-policy validators prove declared/live required-status consistency;
- [x] exact-head `workflow_dispatch` recovery requires an open PR number, unchanged full head SHA and dispatch ref resolving to that same head;
- [x] Dependency Review receives explicit validated base/head refs for PR and recovery-dispatch modes;
- [x] canonical GitHub governance documentation names the aggregate required context and recovery path;
- [x] Cargo Dependabot is enabled;
- [x] stale workspace/test-matrix and CODEOWNERS wording is corrected;
- [x] obsolete migration-only workflow is removed;
- [ ] repaired exact-head full-diff self-review has no open material finding;
- [ ] repaired exact-head GitHub Actions validation passes, including transition `Agent governance / validate` and new `Merge gate / validate`;
- [ ] independent automatic review of the repaired exact head has no open material finding;
- [ ] squash merge only after unchanged-head readiness.

## Excluded scope

- no gameplay/client/server runtime behavior;
- no protocol, persistence or content semantics;
- no production deployment, protected-environment approval or secret expansion;
- no cross-repository writes;
- no lowering of review, exact-head or branch-protection evidence requirements;
- no cleanup of unrelated historical branches in this package.

## Implementation / findings

Initial implementation on exact head `8248a7f3f4dda50101e922a3bc1fcc0b18232468` introduced the aggregate gate and passed the transition governance check plus all completed Linux/policy/security jobs before independent review surfaced three material findings.

Independent review repair cycle 1:

1. **P1 — missing dispatch recovery:** valid. Added `workflow_dispatch` inputs for PR number and full expected head SHA. The scope job now verifies that a manual dispatch ref resolves to that exact SHA, re-fetches the open same-repository PR targeting `main`, rejects head movement, and exports validated base/head revisions consumed by all downstream jobs. Dependency Review now receives explicit `base-ref`/`head-ref` values so recovery mode does not depend on a `pull_request` event payload.
2. **P1 — incomplete authoritative task record:** valid. Restored all task-template metadata fields and explicit focused/component/E2E/exact-head/self-review/independent-review/closeout sections with pending values where final-head evidence cannot yet exist.
3. **P2 — stale canonical GitHub governance:** valid. Updated `docs/repository/GITHUB_GOVERNANCE.md` to the aggregate required context, exact-head recovery semantics and Cargo Dependabot, and extended the repository validator to assert this documentation remains aligned.

The existing `Agent governance / validate` pull-request workflow remains unchanged in this transition package so the currently live old ruleset can validate PR #162. It may be deduplicated only after the new required context has been applied and verified live on `main`.

## Validation

### Focused

- repository/agent governance validators on pre-repair exact head `8248a7f3f4dda50101e922a3bc1fcc0b18232468`: PASS in Merge gate governance job `93726822735`, superseded by repair cycle 1;
- Dependency Review on pre-repair head: PASS, job `93726822717`, superseded by repair cycle 1;
- Rust policy/metadata on pre-repair head: PASS, job `93726843566`, superseded by repair cycle 1;
- CodeQL Python/Actions on pre-repair head: PASS, jobs `93726822752` / `93726822916`, superseded by repair cycle 1;
- repaired-head focused validation: pending exact repaired candidate head.

### Component/integration

- command/run: repository configuration apply/verify is post-merge by design and must be verified on the resulting `main` merge revision;
- result: pending post-merge configuration workflow; no product component integration applies.

### E2E

- scenario: `NOT_APPLICABLE` — repository CI/governance transition only; no executable game journey changes;
- result: `NOT_APPLICABLE`.

### Exact-head CI

- final head: pending repair-cycle-1 checkpoint commit;
- trigger source: `pull_request/synchronize`;
- workflow/run/job: new Merge gate run pending repaired head; transition Agent governance run also required;
- runner assignment: GitHub-hosted Linux and Windows runners expected from workflow contract; exact repaired assignments pending;
- classification: pending;
- result: pending.

## Self-review

- exact head: pre-repair `8248a7f3f4dda50101e922a3bc1fcc0b18232468` PASS, superseded by review-driven repairs;
- method/reviewer: implementing/coordinating agent full-diff review;
- material findings: no self-review finding on old head; independent review later found three issues repaired in cycle 1;
- verdict: repaired exact-head self-review pending.

## Independent review

- required: `YES` — changing the sole protected merge-authority path has unusual repository-wide blast radius and common-mode-error risk;
- exact head: pre-repair `8248a7f3f4dda50101e922a3bc1fcc0b18232468`, repaired-head review pending;
- method/auditor: automatic `chatgpt-codex-connector` independent PR review;
- material findings: P1 dispatch recovery, P1 task recoverability, P2 canonical governance alignment; all repaired in cycle 1, repaired-head verification pending;
- verdict: pending repaired exact-head review.

## PR and closeout

- changed-file review: pre-repair full diff reviewed; repaired full diff pending;
- unresolved review threads: 3 from pre-repair independent review, pending repaired-head reply/resolution;
- related/superseded PRs: PR #161 was non-overlapping and merged before branch reconciliation;
- protected auto-merge: pending;
- merge commit/result: pending;
- ownership release: pending archive after terminal merge/configuration verification.

## Context checkpoint

```yaml
last_progress: Independent review of pre-repair head found three valid issues; repair cycle 1 added trusted exact-head dispatch recovery, completed the task schema and aligned canonical GitHub governance plus validator enforcement.
status: validating
branch: ci/OTV2-20260811-merge-gate-hardening
head_sha: null
pr: 162
final_head_sha: null
final_head_frozen_at: null
ci_trigger_source: pull_request/synchronize
ci_check_generation: repair-cycle-1-final-head-pending
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
blocker: null
next_action: Freeze the repair-cycle-1 candidate head, perform a complete repaired-head self-review, then verify fresh transition governance, aggregate merge-gate CI and independent review on that unchanged head.
```
