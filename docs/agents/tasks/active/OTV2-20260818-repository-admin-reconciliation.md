# OTV2-20260818-repository-admin-reconciliation

```yaml
task_id: OTV2-20260818-repository-admin-reconciliation
title: Reconcile live Oteryn-Game GitHub repository administration
mode: GOVERNANCE
status: blocked
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
updated_at: 2026-08-18T15:14:00+02:00
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
- `PROVEN`: owner manually dispatched `Repository configuration` on `main`; `Apply and verify GitHub settings` failed before any apply with `REPO_ADMIN_TOKEN is unavailable.` and process exit code `2`.
- `PROVEN`: target-local `REPO_ADMIN_TOKEN` is not available to the administration workflow at the failed manual-dispatch run.
- `PROVEN`: current GitHub fine-grained PAT permission data places the repository-update, Actions-permission, environment administration, vulnerability/security toggle, private-vulnerability-reporting, ruleset and topic endpoints used by the canonical apply script under repository `Administration`; the repository label endpoints used by the script are available under repository `Issues`. A target fine-grained PAT restricted to `Oteryn/Oteryn-Game` therefore needs repository `Administration: read/write` plus `Issues: read/write` for the script's declared calls; implicit metadata read remains GitHub-managed.
- `UNKNOWN`: exact live Actions default permissions, security toggles, labels and ruleset readback until the canonical administration workflow can execute its authenticated verify path.

## Acceptance criteria

- [x] Resolve the canonical target policy and apply/verify mechanism from trusted `main`.
- [x] Compare live public repository metadata/merge settings with the canonical policy and record proven drift.
- [x] Inspect live `main` branch readback and record the current protection signal.
- [x] Manually dispatch `Repository configuration` on `main` and classify the first actionable failure.
- [x] Resolve a least-privilege target token shape from current GitHub fine-grained PAT permission data.
- [ ] Create an authorized target-local `REPO_ADMIN_TOKEN` secret for `Oteryn/Oteryn-Game` without copying/reusing the legacy source secret.
- [ ] Re-run `Repository configuration` against `main` and obtain a successful apply+verify result.
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

### Manual administration dispatch

The owner executed the canonical `Repository configuration` workflow on `main`. The `Apply and verify GitHub settings` step produced:

```text
REPO_ADMIN_TOKEN is unavailable.
Process completed with exit code 2.
```

This is a fail-closed precondition failure from `tools/repository/apply_github_settings.py`: the script exits before repository mutation when `GH_TOKEN`, populated from `${{ secrets.REPO_ADMIN_TOKEN }}`, is empty. The subsequent public live readback remained unchanged, consistent with no apply having occurred.

### Least-privilege target token

Current GitHub fine-grained PAT permission data was checked against every API family invoked by the canonical apply script. The required target token can be restricted to resource owner `Oteryn`, repository `Oteryn-Game`, with repository permissions:

- `Administration`: read and write;
- `Issues`: read and write.

`Administration` covers the script's repository settings, Actions default workflow permissions, legacy environment removal, repository security toggles, private vulnerability reporting, rulesets and topics. `Issues` covers repository label list/create/update operations. The workflow checkout continues to use its ordinary read-only `GITHUB_TOKEN`; `REPO_ADMIN_TOKEN` does not need repository contents write for this script.

### Capability exhaustion

Attempted safe routes before requiring owner secret provisioning:

1. connected GitHub repository read/write/admin identity: available;
2. connector repository-settings mutation: unavailable;
3. connector ruleset/protection mutation: unavailable;
4. connector Actions `workflow_dispatch`: unavailable; owner manually dispatched instead;
5. connector repository secret mutation/readback: unavailable;
6. generic authenticated admin endpoint through GitHub fetch: rejected by the connector allowlist;
7. delete-ref: unavailable;
8. alternate installable plugin exposing GitHub repository administration: none found.

The existing trusted workflow remains the required execution route. Creating a no-op policy/workflow commit solely to generate a `push` event is explicitly forbidden by GitHub-only and anti-stall governance.

## Validation

### Focused

- live repository metadata readback: `PASS` as drift evidence; repository ID `1338291140` and admin/push connector permission observed.
- live `main` readback: `PASS` as drift evidence at `5d50f56da8216ea33773c34b320c620f26b52f7f`; protection signal reports disabled.
- canonical policy/apply/workflow inspection: `PASS`.
- retained migration-branch discovery: `PASS`, both expected branches observed.
- manual administration dispatch: `FAIL_CLOSED`, missing `REPO_ADMIN_TOKEN`, exit code `2`.
- least-privilege token permission mapping: `PASS` against current GitHub fine-grained PAT permission data.
- PR #6 initial governance CI failure: root cause `PR body is missing ## Summary`; PR metadata repaired without moving the then-current head.

### Component/integration

- canonical `Repository configuration` apply+verify: `BLOCKED` on missing target-local `REPO_ADMIN_TOKEN` secret.

### E2E

- scenario: apply policy to live target and read it back through the canonical verifier.
- result: `BLOCKED` before mutation because `REPO_ADMIN_TOKEN` is unavailable.

### Exact-head CI

- final head: `NOT_APPLICABLE` while live administration remains blocked before delivery readiness.
- trigger source: PR task-record CI is not the administration gate.
- administration workflow: owner manual dispatch reached the apply step and failed with exit code `2` due to missing secret.
- classification: `EXTERNAL_CREDENTIAL_BLOCKER`, not repository-code validation failure.
- result: `BLOCKED`.

## Self-review

- exact head: pending after material blocker checkpoint commit.
- method/reviewer: implementing/coordinating chat GitHub session.
- material findings: zero in blocker classification; missing credential is now directly proven rather than inferred.
- verdict: `PASS` for bounded task state; delivery remains blocked.

## Independent review

- required: `YES` only if a later repository/control-plane content change alters authority/safety; not required merely to run the already accepted canonical policy apply/readback.
- exact head: `NOT_APPLICABLE` for current task-record-only blocked state.
- method/auditor: `NOT_APPLICABLE`.
- material findings: `NOT_APPLICABLE`.
- verdict: `NOT_APPLICABLE`.

## PR and closeout

- PR: `#6`, draft until live administration is reconciled.
- changed-file review: task record only.
- unresolved review threads: none observed before blocker checkpoint.
- related/superseded PRs: PR #4/#5 completed predecessor reconciliation; Dependabot PR #2 is unrelated.
- protected auto-merge: `NOT_APPLICABLE` while task is blocked.
- merge commit/result: pending.
- ownership release: pending completion of live administration and archive.

## Context checkpoint

```yaml
last_progress: Owner manually dispatched the canonical administration workflow; the apply step failed closed with `REPO_ADMIN_TOKEN is unavailable.` and exit code 2. Current GitHub fine-grained PAT mappings were then checked to resolve the least-privilege replacement token shape.
status: blocked
branch: docs/otv2-20260818-repository-admin-reconciliation
head_sha: null
pr: 6
final_head_sha: null
final_head_frozen_at: null
ci_trigger_source: owner_workflow_dispatch_main
ci_check_generation: repository_configuration_missing_secret
ci_checks_for_current_head: 0
ci_run_ids: []
ci_job_ids: []
runner_assignment_state: assigned_and_executed
terminal_ci_wait_started_at: null
terminal_ci_checks_for_current_generation: 0
unchanged_state_checks: 1
identical_failure_retries: 0
repair_cycles_for_current_gate: 0
ci_recovery_actions_for_current_head: 0
stall_warnings: 0
owner_action_required: Create a new fine-grained PAT owned by `Oteryn`, restricted to `Oteryn-Game`, with repository `Administration: read/write` and `Issues: read/write`; save it as the `Oteryn/Oteryn-Game` Actions repository secret `REPO_ADMIN_TOKEN`. Do not copy or reuse the legacy source secret. Then rerun `Repository configuration` on `main` once.
blocker: Canonical administration workflow executed but failed closed before mutation because `${{ secrets.REPO_ADMIN_TOKEN }}` resolved empty; active connector cannot create repository secrets.
next_action: Provision target-local `REPO_ADMIN_TOKEN` with the resolved least-privilege permissions and rerun `Repository configuration` on `main` once.
```
