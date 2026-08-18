# OTV2-20260818-repository-admin-reconciliation

```yaml
task_id: OTV2-20260818-repository-admin-reconciliation
title: Reconcile live Oteryn-Game GitHub repository administration
mode: GOVERNANCE
status: validating
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
updated_at: 2026-08-18T15:46:00+02:00
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

Apply and verify `.github/repository-policy.json` against the live `Oteryn/Oteryn-Game` repository, restore protected merge administration, then remove the two retained migration branches and close the task lifecycle.

Entry alias: `OTGAME-REPOSITORY-ADMIN-RECONCILIATION`.

## Source of truth

- `PROVEN`: policy: `.github/repository-policy.json` blob `a3e34eee7a84c32c31f55d65eeccd549ab377007`.
- `PROVEN`: apply/verify implementation: `tools/repository/apply_github_settings.py` blob `209c56049a0eea84eaabdf6e9d7558ed1985989d`.
- `PROVEN`: workflow: `.github/workflows/repository-configuration.yml` blob `e3b752f5b4712d07de13e88b909f99e40b3a99b6`.
- `PROVEN`: first manual run failed before mutation because the required administration credential was unavailable.
- `PROVEN`: after provisioning the credential, a second run reached repository administration but failed HTTP 403 because the credential lacked effective Administration write permission.
- `PROVEN`: the owner corrected that permission and reran the canonical workflow on `main`.
- `PROVEN`: post-rerun public live readback now matches the canonical repository metadata/merge-policy surface and reports `main` protected.
- `DERIVED`: the apply script reaches ruleset configuration only after repository settings, labels and security configuration. The observed transition of `main` from unprotected to protected therefore proves the corrected run advanced through those preceding mutation phases without an API error.
- `UNKNOWN`: final corrected manual-run conclusion and exact authenticated verifier output are not exposed by the active connector.

## Acceptance criteria

- [x] Resolve canonical target policy and apply/verify mechanism.
- [x] Prove initial repository-settings drift.
- [x] Classify the first missing-credential failure.
- [x] Provision a target-scoped administration credential without copying the legacy source credential.
- [x] Classify and repair the HTTP 403 caused by insufficient Administration permission.
- [x] Re-run the canonical repository-configuration workflow.
- [x] Verify live description, topics, wiki, merge methods, auto-merge, update-branch, squash defaults and automatic merged-branch deletion match policy.
- [x] Verify `main` public branch readback reports `protected: true`.
- [ ] Capture successful final canonical apply+verify conclusion for the corrected run.
- [ ] Capture exact authenticated Actions/security/ruleset verifier result.
- [ ] Remove retained migration branches `docs/otv2-20260818-target-coordinate-reconciliation` and `docs/otv2-20260818-target-coordinate-reconciliation-closeout` through a delete-ref-capable path.
- [ ] Run final PR exact-head review/CI, merge, archive and release ownership.

## Excluded scope

No gameplay/runtime/protocol/persistence change. No mutation of `blakinio/Oteryn-v2` or another repository. No weakening/bypass of merge controls. No no-op control-plane commit merely to generate an event.

## Post-rerun live state

Public repository readback now matches policy for directly observable fields:

- description: `Greenfield native Rust multichannel game server, client, protocol, content, and tooling platform.`
- topics: `game-engine`, `game-server`, `mmorpg`, `multichannel`, `oteryn`, `rust`.
- wiki: disabled.
- squash merge: enabled; merge commits and rebase disabled.
- auto-merge: enabled.
- automatic source-branch deletion: enabled.
- update-branch support: enabled.
- squash PR title default: enabled.
- squash title/message: `PR_TITLE` / `PR_BODY`.

`GET /branches/main` now reports `protected: true` at unchanged `main@5d50f56da8216ea33773c34b320c620f26b52f7f`. The classic-protection sub-object remains disabled, consistent with protection being provided by a repository ruleset.

The two predecessor migration branches are still present. The active connector exposes no delete-ref operation; automatic delete-on-merge is not retroactive.

## Validation

### Focused

- policy/apply/workflow inspection: `PASS`.
- corrected workflow rerun: `EXECUTED`.
- public repository metadata/merge-policy readback: `PASS`.
- public main protection signal: `PASS`.
- retained migration branch discovery: `PASS`, both still present.

### Component/integration

- repository administration application: `PASS` for all public live fields.
- exact authenticated verifier conclusion: `PENDING_EXTERNAL_READBACK`.

### E2E

- scenario: apply canonical policy and observe live target administration.
- result: `PARTIAL_PASS`; public observable state matches policy and `main` is protected, but final canonical verifier conclusion has not been captured by this session.

## Self-review

- method/reviewer: implementing/coordinating chat GitHub session.
- material findings: no mismatch in directly observable post-rerun fields; final verifier readback and two retained refs remain open.
- verdict: `PASS` for current validating state.

## Independent review

- required: `NO` for invoking an already accepted canonical administration apply/readback; this task has not modified repository control-plane source.

## PR and closeout

- PR: `#6`, draft until final external evidence/cleanup is complete.
- changed-file scope: task record only.
- merge/archive/ownership release: pending.

## Context checkpoint

```yaml
last_progress: Corrected administration credential rerun reconciled the public live repository surface and restored protected main.
status: validating
branch: docs/otv2-20260818-repository-admin-reconciliation
head_sha: null
pr: 6
final_head_sha: null
final_head_frozen_at: null
ci_trigger_source: owner_workflow_dispatch_main
ci_check_generation: repository_configuration_corrected_credential
ci_checks_for_current_head: 0
ci_run_ids: []
ci_job_ids: []
runner_assignment_state: assigned_and_executed
terminal_ci_wait_started_at: null
terminal_ci_checks_for_current_generation: 0
unchanged_state_checks: 0
identical_failure_retries: 0
repair_cycles_for_current_gate: 2
ci_recovery_actions_for_current_head: 0
stall_warnings: 0
owner_action_required: Capture the final corrected Repository configuration result and delete the two retained migration branches through the GitHub UI or another authorized delete-ref-capable path.
blocker: Active connector cannot enumerate corrected manual-dispatch runs or delete Git refs.
next_action: Confirm corrected Repository configuration is green and delete the two retained migration branches, then resume final PR validation/merge/archive.
```
