# OTV2-20260818-repository-admin-reconciliation

```yaml
task_id: OTV2-20260818-repository-admin-reconciliation
title: Reconcile live Oteryn-Game GitHub repository administration
mode: GOVERNANCE
status: blocked
repository: Oteryn/Oteryn-Game
base_branch: main
branch: docs/otv2-20260818-repository-admin-reconciliation
pr: null
base_sha: 5d50f56da8216ea33773c34b320c620f26b52f7f
head_sha: null
final_head_sha: null
final_head_frozen_at: null
owner: chat-github-20260818-repository-admin-reconciliation
created_at: 2026-08-18T14:46:00+02:00
updated_at: 2026-08-18T14:46:00+02:00
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

Apply and verify the canonical `.github/repository-policy.json` against the live `Oteryn/Oteryn-Game` GitHub repository after the history-preserving migration, including repository merge policy, metadata, topics, Actions permissions, security settings, `Protect main`, and automatic merged-branch deletion. Remove the two retained migration branches after the live administration policy is restored.

Entry alias: `OTGAME-REPOSITORY-ADMIN-RECONCILIATION`.

## Architecture and source of truth

- `PROVEN`: canonical repository policy is `.github/repository-policy.json` at blob `a3e34eee7a84c32c31f55d65eeccd549ab377007` on `main`.
- `PROVEN`: canonical apply/readback implementation is `tools/repository/apply_github_settings.py` at blob `209c56049a0eea84eaabdf6e9d7558ed1985989d`.
- `PROVEN`: canonical administration workflow is `.github/workflows/repository-configuration.yml` at blob `e3b752f5b4712d07de13e88b909f99e40b3a99b6`; it supports `workflow_dispatch` and requires target secret `REPO_ADMIN_TOKEN`.
- `PROVEN`: archived coordinate reconciliation explicitly left repository-settings reconciliation as this follow-up and did not copy/reuse the source repository administration token.
- `PROVEN`: the active GitHub connector reports admin/push access to `Oteryn/Oteryn-Game` but exposes no repository-settings/ruleset/secret mutation, no workflow dispatch, and no delete-ref operation.
- `UNKNOWN`: presence and validity of a target-local `REPO_ADMIN_TOKEN`; secret values/presence are not exposed by the active connector.
- `UNKNOWN`: exact live Actions default permissions, security toggles, labels and ruleset readback until the canonical administration workflow can execute its authenticated verify path.

## Acceptance criteria

- [x] Resolve the canonical target policy and apply/verify mechanism from trusted `main`.
- [x] Compare live public repository metadata/merge settings with the canonical policy and record proven drift.
- [x] Inspect live `main` branch readback and record the current protection signal.
- [ ] Run `Repository configuration` against `main` with a target-local authorized `REPO_ADMIN_TOKEN` and obtain a successful apply+verify result.
- [ ] Verify live repository metadata and merge policy match `.github/repository-policy.json`.
- [ ] Verify Actions default workflow permissions and PR-review authority match policy.
- [ ] Verify required security features and private vulnerability reporting match policy.
- [ ] Verify `Protect main` is active, no-bypass, squash-only, requires `Merge gate / validate`, strict up-to-date state, Code Owner review and review-thread resolution.
- [ ] Verify unsupported public-repository push ruleset is absent and the base-branch CODEOWNERS fallback is intact.
- [ ] Verify automatic merged-branch deletion is enabled.
- [ ] Remove retained migration branches `docs/otv2-20260818-target-coordinate-reconciliation` and `docs/otv2-20260818-target-coordinate-reconciliation-closeout` through an authorized delete-ref path.
- [ ] Perform full governance E2E/readback, archive this task and release ownership.

## Excluded scope

No gameplay/runtime/protocol/persistence change. No mutation of `blakinio/Oteryn-v2` or any other repository. No weakening or bypass of merge/ruleset/CODEOWNERS controls. No copying or reuse of a source repository secret. No no-op control-plane commit merely to manufacture a workflow event.

## Implementation / findings

### Proven live drift at task start

Live repository API readback for `Oteryn/Oteryn-Game` repository ID `1338291140` differs from the canonical policy in at least these directly observable fields:

- description: live `null`; policy `Greenfield native Rust multichannel game server, client, protocol, content, and tooling platform.`
- topics: live `[]`; policy `rust`, `game-engine`, `game-server`, `mmorpg`, `multichannel`, `oteryn`.
- wiki: live `true`; policy `false`.
- merge commits: live `true`; policy `false`.
- rebase merge: live `true`; policy `false`.
- auto-merge: live `false`; policy `true`.
- automatic source-branch deletion: live `false`; policy `true`.
- update-branch support: live `false`; policy `true`.
- squash PR title default: live `false`; policy `true`.
- squash title/message live defaults differ from policy `PR_TITLE` / `PR_BODY`.

Live `GET /branches/main` readback at `main@5d50f56da8216ea33773c34b320c620f26b52f7f` reports `protected: false` and classic protection disabled. Exact ruleset readback is not exposed by the active connector, so the canonical `Protect main` ruleset state remains `UNKNOWN` until authenticated workflow verification.

The two migration branches intentionally retained by the preceding task are still present.

### Capability exhaustion

Attempted safe routes before declaring the external-operation blocker:

1. connected GitHub repository read/write/admin identity: available;
2. connector repository-settings mutation: unavailable;
3. connector ruleset/protection mutation: unavailable;
4. connector Actions `workflow_dispatch`: unavailable;
5. connector repository secret mutation/readback: unavailable;
6. generic authenticated admin endpoint through GitHub fetch: rejected by the connector allowlist;
7. delete-ref: unavailable;
8. alternate installable plugin exposing GitHub repository administration: none found.

The existing trusted workflow is therefore the required next execution route. Creating a no-op policy/workflow commit solely to generate a `push` event is explicitly forbidden by GitHub-only and anti-stall governance.

## Validation

### Focused

- live repository metadata readback: `PASS` as drift evidence; repository ID `1338291140` and admin/push connector permission observed.
- live `main` readback: `PASS` as drift evidence at `5d50f56da8216ea33773c34b320c620f26b52f7f`; protection signal reports disabled.
- canonical policy/apply/workflow inspection: `PASS`.
- retained migration-branch discovery: `PASS`, both expected branches observed.

### Component/integration

- canonical `Repository configuration` apply+verify: `BLOCKED` because the active connector exposes neither `workflow_dispatch` nor equivalent repository administration mutation.

### E2E

- scenario: apply policy to live target and read it back through the canonical verifier.
- result: `BLOCKED` on workflow dispatch capability before execution.

### Exact-head CI

- final head: `NOT_APPLICABLE` while administration task remains blocked before delivery readiness.
- trigger source: `NOT_APPLICABLE`.
- workflow/run/job: `NOT_APPLICABLE`.
- runner assignment: `NOT_APPLICABLE`.
- classification: external-operation blocker, not CI failure.
- result: `BLOCKED`.

## Self-review

- exact head: pending task-record commit readback.
- method/reviewer: implementing/coordinating chat GitHub session.
- material findings: zero in live-drift classification; unknown admin-only surfaces are explicitly not asserted.
- verdict: `PASS` for blocker classification and bounded task state.

## Independent review

- required: `YES` only if a later repository/control-plane content change alters authority/safety; not required merely to run the already accepted canonical policy apply/readback.
- exact head: `NOT_APPLICABLE` for current task-record-only blocked state.
- method/auditor: `NOT_APPLICABLE`.
- material findings: `NOT_APPLICABLE`.
- verdict: `NOT_APPLICABLE`.

## PR and closeout

- changed-file review: task record only.
- unresolved review threads: pending PR creation.
- related/superseded PRs: PR #4/#5 completed predecessor reconciliation; Dependabot PR #2 is unrelated.
- protected auto-merge: `NOT_APPLICABLE` while task is blocked.
- merge commit/result: pending.
- ownership release: pending completion of live administration and archive.

## Context checkpoint

```yaml
last_progress: Proven live target repository drift against canonical policy and exhausted all safe connector-native administration routes.
status: blocked
branch: docs/otv2-20260818-repository-admin-reconciliation
head_sha: pending task-record commit readback
pr: null
final_head_sha: null
final_head_frozen_at: null
ci_trigger_source: null
ci_check_generation: null
ci_checks_for_current_head: 0
ci_run_ids: []
ci_job_ids: []
runner_assignment_state: not_applicable
terminal_ci_wait_started_at: null
terminal_ci_checks_for_current_generation: 0
unchanged_state_checks: 0
identical_failure_retries: 0
repair_cycles_for_current_gate: 0
ci_recovery_actions_for_current_head: 0
stall_warnings: 0
owner_action_required: Run the existing `Repository configuration` workflow manually on branch `main` in `Oteryn/Oteryn-Game`; it must execute with a target-local authorized `REPO_ADMIN_TOKEN` and must not reuse/copy the legacy source secret.
blocker: Active connector has admin repository identity but no workflow-dispatch, repository-settings/ruleset/secret mutation, or delete-ref operation; the trusted apply workflow cannot be started from this session.
next_action: Dispatch `Repository configuration` on `main` in `Oteryn/Oteryn-Game` once through GitHub Actions.
```
