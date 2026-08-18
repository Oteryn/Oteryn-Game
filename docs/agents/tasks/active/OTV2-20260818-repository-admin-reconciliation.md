# OTV2-20260818-repository-admin-reconciliation

```yaml
task_id: OTV2-20260818-repository-admin-reconciliation
title: Reconcile live Oteryn-Game GitHub repository administration
mode: GOVERNANCE
status: ready
repository: Oteryn/Oteryn-Game
base_branch: main
branch: docs/otv2-20260818-repository-admin-reconciliation
pr: 6
base_sha: 5d50f56da8216ea33773c34b320c620f26b52f7f
head_sha: null
final_head_sha: null
final_head_frozen_at: null
owner: chat-github-20260818-repository-admin-reconciliation
created_at: 2026-08-18T14:46:00+02:00
updated_at: 2026-08-18T15:56:00+02:00
execution_budget_minutes: 60
large_budget_reason: null
owned_paths:
  - docs/agents/tasks/active/OTV2-20260818-repository-admin-reconciliation.md
public_contracts:
  - .github/repository-policy.json
depends_on:
  - OTV2-20260818-target-coordinate-reconciliation
blocks:
  - canonical target repository administration closeout
cross_repository_coordination_id: OTERYN-GAME-ADMIN-20260818
external_repositories: []
```

## Outcome

Apply and verify `.github/repository-policy.json` against the live `Oteryn/Oteryn-Game` repository, restore protected merge administration, remove the retained migration refs, then close the task lifecycle.

Entry alias: `OTGAME-REPOSITORY-ADMIN-RECONCILIATION`.

## Source of truth and resolved failures

- `PROVEN`: policy: `.github/repository-policy.json` blob `a3e34eee7a84c32c31f55d65eeccd549ab377007`.
- `PROVEN`: apply/verify implementation: `tools/repository/apply_github_settings.py` blob `209c56049a0eea84eaabdf6e9d7558ed1985989d`.
- `PROVEN`: workflow: `.github/workflows/repository-configuration.yml` blob `e3b752f5b4712d07de13e88b909f99e40b3a99b6`.
- `PROVEN`: the first manual configuration run failed closed before mutation with `REPO_ADMIN_TOKEN is unavailable.` and exit code `2`.
- `PROVEN`: after target credential provisioning, the next run reached repository administration and failed HTTP `403 Resource not accessible by personal access token` on repository PATCH.
- `PROVEN`: the target fine-grained PAT was corrected to include repository `Administration: read/write`; it remained scoped to resource owner `Oteryn` and repository `Oteryn-Game`.
- `PROVEN`: the corrected `Repository configuration` run was reported green by the owner.
- `DERIVED`: a green canonical run means `tools/repository/apply_github_settings.py` completed its final `verify()` path and exited zero; the exact manual run ID is not exposed by the active connector.

## Acceptance criteria

- [x] Resolve canonical target policy and apply/verify mechanism.
- [x] Prove initial repository-settings drift.
- [x] Classify and repair the missing-credential failure.
- [x] Provision a target-scoped administration credential without copying the legacy source credential.
- [x] Classify and repair the insufficient-Administration HTTP 403 failure.
- [x] Re-run `Repository configuration` on `main` and obtain a green canonical apply+verify result.
- [x] Verify live description, topics, wiki, merge methods, auto-merge, update-branch, squash defaults and automatic merged-branch deletion match policy.
- [x] Verify live `main` reports `protected: true` through repository ruleset enforcement.
- [x] Verify Actions/security/ruleset policy through the canonical authenticated verifier by successful final workflow completion.
- [x] Remove retained migration branches `docs/otv2-20260818-target-coordinate-reconciliation` and `docs/otv2-20260818-target-coordinate-reconciliation-closeout` through an exact-SHA-guarded one-off cleanup.
- [x] Verify the temporary cleanup branch removed itself and left no persistent workflow/control-plane delta on `main`.
- [ ] Freeze one unchanged final PR head, complete exact-head review/CI, squash merge, archive the task and release ownership.

## Post-reconciliation live state

Direct repository readback matches policy for the observable administration surface:

- description: `Greenfield native Rust multichannel game server, client, protocol, content, and tooling platform.`
- topics: `game-engine`, `game-server`, `mmorpg`, `multichannel`, `oteryn`, `rust`.
- wiki: disabled.
- squash merge: enabled; merge commits and rebase disabled.
- auto-merge: enabled.
- automatic source-branch deletion: enabled.
- update-branch support: enabled.
- squash PR title default: enabled.
- squash title/message: `PR_TITLE` / `PR_BODY`.
- `main@5d50f56da8216ea33773c34b320c620f26b52f7f`: `protected: true`; classic branch-protection sub-object remains disabled, consistent with ruleset-based protection.

The canonical workflow completed green after credential repair, providing authenticated verification of Actions defaults, security toggles, labels and `Protect main` ruleset state that are not all exposed by the connected read API.

## Retained-ref cleanup

The predecessor PRs explicitly declared `Branch-Disposition: delete`:

- PR #4 branch `docs/otv2-20260818-target-coordinate-reconciliation`, expected head `0b1f8288c20a69a50628e401fe3a7fb60681f050`;
- PR #5 branch `docs/otv2-20260818-target-coordinate-reconciliation-closeout`, expected head `3541adcefdc3c9c6c930eb50b3dcbee38390a5fa`.

Before deletion, connector compare readback proved each live branch was still exactly identical to its expected merged PR head. A temporary branch `ops/one-off-migration-residue-cleanup-20260818` was created from `main`; commit `235159abc89e99ca5bed1d6b7e36b215e3faa39f` added a push-triggered one-off workflow with `contents: write` and hard-coded exact-SHA guards. It deleted only those two verified refs and then deleted its own branch. Post-run branch searches return no match for either retained migration branch or the temporary cleanup branch.

The one-off workflow was never merged into `main`; therefore the cleanup produced no persistent repository control-plane source change.

## Validation

### Focused

- canonical policy/apply/workflow inspection: `PASS`.
- final canonical `Repository configuration`: `PASS` — owner-observed green run; live readback corroborates the expected state.
- public repository metadata/merge-policy readback: `PASS`.
- public `main` protection signal: `PASS`.
- exact predecessor branch identity before delete: `PASS` for both refs.
- predecessor branch absence after cleanup: `PASS`.
- one-off cleanup branch self-removal: `PASS`.

### Component / integration

- repository administration application and authenticated verify: `PASS`.
- automatic merged-branch deletion setting: `PASS`.
- explicit historical residue cleanup: `PASS`.

### E2E

- scenario: apply canonical GitHub policy to the migrated target, verify protected merge administration, then remove the two retained predecessor refs without changing `main` product/control-plane source.
- result: `PASS`.
- gameplay/runtime E2E: `NOT_APPLICABLE` — this task changes repository administration only.

## Self-review

- method/reviewer: implementing/coordinating chat GitHub session.
- scope challenged: live policy drift, credential failure modes, ruleset protection, retained-ref identity, cleanup safety and persistent-diff scope.
- material findings: zero open after credential repairs and exact-SHA cleanup.
- transient cleanup workflow: bounded to a self-deleting non-main branch, no external actions, exact target SHA guards, no persistent `main` delta.
- verdict: `PASS` before final-head freeze.

## Independent review

- required: `NO` for final delivery diff because PR #6 changes only this task record; canonical governance code/policy was not modified, and the transient branch cleanup only removed already-merged refs explicitly marked `Branch-Disposition: delete`.

## PR and closeout

- PR: `#6`.
- delivery changed-file scope: task record only.
- final-head SHA: record after this commit in immutable PR evidence; do not create a self-referential task commit.
- merge method: squash only.
- post-merge archive movement: required as a separate bounded closeout change because merge SHA is not knowable before merge.
- ownership release: after archive merge.

## Context checkpoint

```yaml
last_progress: Canonical administration verification is green and all retained migration refs were removed by exact-SHA-guarded autonomous cleanup; repository is ready for final PR freeze and exact-head CI.
status: ready
branch: docs/otv2-20260818-repository-admin-reconciliation
head_sha: null
pr: 6
final_head_sha: null
final_head_frozen_at: null
ci_trigger_source: pending_final_pr_generation
ci_check_generation: final_delivery_pending
ci_checks_for_current_head: 0
ci_run_ids: []
ci_job_ids: []
runner_assignment_state: pending
terminal_ci_wait_started_at: null
terminal_ci_checks_for_current_generation: 0
unchanged_state_checks: 0
identical_failure_retries: 0
repair_cycles_for_current_gate: 2
ci_recovery_actions_for_current_head: 0
stall_warnings: 0
owner_action_required: null
blocker: null
next_action: Freeze the current PR #6 head, run exact-head review and required CI, squash merge, then archive this task and release ownership.
```
