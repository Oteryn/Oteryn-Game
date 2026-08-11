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
updated_at: 2026-08-11T11:20:00+02:00
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

Make one always-present `Merge gate / validate` check the protected-branch merge authority for pull requests, with governance, dependency review, CodeQL and path-proportional Rust validation composed behind it. Provide bounded exact-head dispatch recovery for a suppressed initial PR event, make path classification robust to file renames, remove stale workspace/bootstrap wording, add Cargo dependency maintenance and retire the completed one-off Rust cutover audit workflow.

## Architecture and source of truth

- `PROVEN`: the live `Protect main` ruleset at task start required only `Agent governance / validate`.
- `PROVEN`: Rust, CodeQL and Dependency Review validation existed separately but was not represented by the protected required context.
- `PROVEN`: `main` already contains the canonical root Rust workspace, while `BUILD_TEST_MATRIX.md` and CODEOWNERS retained bootstrap/future-workspace wording.
- `PROVEN`: `.github/dependabot.yml` covered GitHub Actions but not Cargo.
- `DERIVED`: one stable aggregate required context avoids required-check deadlocks from path-proportional jobs while still failing unless every applicable sub-gate passes.
- `DERIVED`: because the aggregate becomes the sole required context, it must retain a trusted exact-head dispatch recovery path equivalent in safety to the existing governance recovery path.
- `PROVEN`: GitHub PR changed-file metadata may include a `previous_filename` for renamed files; scope classification must account for both source and destination paths so a rename cannot move a Rust-sensitive file out of a watched root and evade Rust validation.

## Acceptance criteria

- [x] one PR workflow always emits `Merge gate / validate`;
- [x] governance, dependency review and CodeQL are mandatory sub-gates;
- [x] Rust Linux/Windows/supply-chain checks run when Rust/workspace-relevant paths change;
- [x] repository policy requires only the stable aggregate merge-gate context;
- [x] repository-policy validators prove declared/live required-status consistency;
- [x] exact-head `workflow_dispatch` recovery requires an open PR number, unchanged full head SHA and dispatch ref resolving to that same head;
- [x] Dependency Review receives explicit validated base/head refs for PR and recovery-dispatch modes;
- [x] Rust/workspace scope classification includes both current/destination paths and rename-source `previous_filename` values;
- [x] repository validator asserts that rename-source classification cannot silently disappear from the merge gate;
- [x] canonical GitHub governance documentation names the aggregate required context and recovery path;
- [x] Cargo Dependabot is enabled;
- [x] stale workspace/test-matrix and CODEOWNERS wording is corrected;
- [x] obsolete migration-only workflow is removed;
- [ ] repair-cycle-3 exact-head full-diff self-review has no open material finding;
- [ ] repair-cycle-3 exact-head GitHub Actions validation passes, including transition `Agent governance / validate` and new `Merge gate / validate`;
- [ ] independent automatic review of the repair-cycle-3 exact head has no open material finding;
- [ ] squash merge only after unchanged-head readiness.

## Excluded scope

- no gameplay/client/server runtime behavior;
- no protocol, persistence or content semantics;
- no production deployment, protected-environment approval or secret expansion;
- no cross-repository writes;
- no lowering of review, exact-head or branch-protection evidence requirements;
- no cleanup of unrelated historical branches in this package.

## Implementation / findings

Initial implementation introduced the aggregate gate and the repository-engineering cleanup described above.

### Independent review repair cycle 1

Independent review of the earlier candidate surfaced three valid findings:

1. **P1 — missing dispatch recovery:** repaired by adding `workflow_dispatch` inputs for PR number and full expected head SHA. The scope job verifies that a manual dispatch ref resolves to that exact SHA, re-fetches the open same-repository PR targeting `main`, rejects head movement, and exports validated base/head revisions consumed by all downstream jobs. Dependency Review receives explicit `base-ref`/`head-ref` values so recovery mode does not depend on a `pull_request` event payload.
2. **P1 — incomplete authoritative task record:** repaired by restoring all task-template metadata fields and explicit focused/component/E2E/exact-head/self-review/independent-review/closeout sections with pending values where final-head evidence cannot yet exist.
3. **P2 — stale canonical GitHub governance:** repaired by updating `docs/repository/GITHUB_GOVERNANCE.md` to the aggregate required context, exact-head recovery semantics and Cargo Dependabot, and by extending the repository validator to assert this documentation remains aligned.

### Independent review repair cycle 2

Independent review of repaired head `6244f2a134a791d53ee9ebcd39cab00cf4e3d8db` surfaced one further valid P1:

4. **P1 — rename-source path could bypass Rust classification:** repaired by collecting both each changed entry's current/destination `filename` and any `previous_filename` into the Rust/workspace classification set. This preserves Rust validation when a file is renamed out of a watched root as well as when it is renamed into one. The repository-policy validator requires the `previous_filename` classification logic to remain present.

### Validation repair cycle 3

Exact-head transition governance on candidate `2283d8a5f1d075d1c68f58221c554c26249ba3c5` correctly checked out the exact PR head and passed agent-governance validation, but repository-policy validation failed on its own overly broad static assertion. The validator used substring search for `paths:` and therefore misclassified the Python type annotation `classification_paths: list[str]` inside the workflow as a workflow-level YAML path filter.

5. **Validator false positive — workflow-level path-filter detection:** repaired by replacing the substring search with an anchored multiline regular expression matching only four-space-indented YAML trigger keys `paths:` or `paths-ignore:`. No merge-gate runtime semantics, permissions, recovery behavior or classification policy changed in this cycle.

This is repair cycle `3/3` for the gate. No further repair cycle may be silently started: any new material final-head failure requires an exact blocker/new hypothesis decision under the anti-stall policy rather than another routine repair loop.

The existing `Agent governance / validate` pull-request workflow remains unchanged in this transition package so the currently live old ruleset can validate PR #162. It may be deduplicated only after the new required context has been applied and verified live on `main`.

## Validation

### Focused

- transition `Agent governance / validate` on candidate `2283d8a5f1d075d1c68f58221c554c26249ba3c5`: exact checkout + agent governance PASS; repository-policy validator failed only on the proven `classification_paths:` false positive, run `31476895797`, job `93732558762`; superseded by cycle-3 validator repair;
- merge-gate scope and older CI results are historical only and cannot prove the final repaired head;
- repair-cycle-3 focused validation: pending exact final candidate head.

### Component/integration

- command/run: repository configuration apply/verify is post-merge by design and must be verified on the resulting `main` merge revision;
- result: pending post-merge configuration workflow; no product component integration applies.

### E2E

- scenario: `NOT_APPLICABLE` — repository CI/governance transition only; no executable game journey changes;
- result: `NOT_APPLICABLE`.

### Exact-head CI

- final head: pending repair-cycle-3 checkpoint commit;
- trigger source: `pull_request/synchronize`;
- workflow/run/job: fresh transition Agent governance and Merge gate runs pending final candidate head;
- runner assignment: pending final candidate head;
- classification: pending;
- result: pending.

## Self-review

- exact head: `2283d8a5f1d075d1c68f58221c554c26249ba3c5` PASS, superseded only by cycle-3 validator false-positive repair;
- method/reviewer: implementing/coordinating agent full-diff review;
- material findings: no merge-gate logic finding in cycle 3; static validator false positive repaired without changing gate semantics;
- verdict: repair-cycle-3 exact-head self-review pending.

## Independent review

- required: `YES` — changing the sole protected merge-authority path has unusual repository-wide blast radius and common-mode-error risk;
- exact head: review of `6244f2a134a791d53ee9ebcd39cab00cf4e3d8db` found one P1; review of `2283d8a5f1d075d1c68f58221c554c26249ba3c5` was requested before the validator-only cycle-3 repair and is superseded for merge readiness;
- method/auditor: automatic `chatgpt-codex-connector` independent PR review;
- material findings: cycle 1 fixed dispatch recovery/task recoverability/canonical governance alignment; cycle 2 fixed rename-source scope bypass; cycle 3 changes only the static validator false-positive pattern;
- verdict: pending independent review of repair-cycle-3 exact head.

## PR and closeout

- changed-file review: repair-cycle-3 full diff pending final candidate head;
- unresolved review threads: historical/material threads pending final repaired-head evidence and resolution;
- related/superseded PRs: PR #161 was non-overlapping and merged before branch reconciliation;
- protected auto-merge: pending;
- merge commit/result: pending;
- ownership release: pending archive after terminal merge/configuration verification.

## Context checkpoint

```yaml
last_progress: Exact-head governance exposed a false positive in repository-policy validation because the validator matched Python classification_paths as YAML paths; cycle 3 narrows the assertion to real workflow-level trigger keys without changing merge-gate behavior.
status: validating
branch: ci/OTV2-20260811-merge-gate-hardening
head_sha: null
pr: 162
final_head_sha: null
final_head_frozen_at: null
ci_trigger_source: pull_request/synchronize
ci_check_generation: repair-cycle-3-final-head-pending
ci_checks_for_current_head: 0
ci_run_ids: []
ci_job_ids: []
runner_assignment_state: unknown
terminal_ci_wait_started_at: null
terminal_ci_checks_for_current_generation: 0
unchanged_state_checks: 0
identical_failure_retries: 0
repair_cycles_for_current_gate: 3
ci_recovery_actions_for_current_head: 0
stall_warnings: 0
owner_action_required: null
blocker: null
next_action: Freeze the repair-cycle-3 candidate head, perform complete exact-head self-review, then verify fresh transition governance, aggregate merge-gate CI and independent review on that unchanged head; do not start a fourth repair cycle.
```
