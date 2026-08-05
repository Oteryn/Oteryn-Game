# OTV2-20260805-github-governance-hardening

```yaml
task_id: OTV2-20260805-github-governance-hardening
title: Harden GitHub contribution, merge, CI and security governance
mode: IMPLEMENTATION
status: validating
repository: blakinio/Oteryn-v2
base_branch: main
branch: fix/repository-policy-api-fields-20260805
pr: null
base_sha: 664bd9a2ede227220705d1058cc420c4a08bd6b2
head_sha: null
owner: repository-governance-agent
created_at: 2026-08-05T15:33:00+02:00
updated_at: 2026-08-05T16:23:00+02:00
execution_budget_minutes: 60
large_budget_reason: null
owned_paths:
  - .github/**
  - .editorconfig
  - .gitignore
  - CONTRIBUTING.md
  - SECURITY.md
  - docs/repository/GITHUB_GOVERNANCE.md
  - tools/repository/**
  - docs/agents/tasks/active/OTV2-20260805-github-governance-hardening.md
public_contracts:
  - .github/repository-policy.json
depends_on:
  - REPO_ADMIN_TOKEN retained from initial repository configuration
blocks: []
cross_repository_coordination_id: null
external_repositories: []
```

## Outcome

Make the documented pull-request and squash-merge discipline enforceable through GitHub, add retained contribution and security policy, harden Actions supply-chain usage, and enable repository security controls without changing product architecture or Rust runtime behavior.

## Acceptance criteria

- [x] `main` has an active no-routine-bypass ruleset requiring PR, exact-head CI, up-to-date branch, resolved threads, squash, linear history, and no force-push/deletion.
- [ ] GitHub Actions default token permissions are read-only and verified by the configuration workflow.
- [x] Pull request and issue templates, CODEOWNERS, contribution policy, security policy, ignore rules, and editor configuration exist.
- [x] Every workflow action is pinned to a full commit SHA.
- [x] Governance validation runs on every PR and push to `main`.
- [x] PR title/body policy is executable and supplies the permanent squash commit.
- [x] Dependency review, Dependabot for Actions, and CodeQL are configured.
- [x] Managed labels and repository topics are applied.
- [ ] Vulnerability alerts, automated fixes, private reporting, secret scanning, and push protection are fully verified where supported.
- [x] Exact-head PR workflows passed and the full diff had no open material audit finding.
- [x] Implementation PR #23 was squash-merged as `30324872af421d0d2bdcb91b360a76a3d44a2592`.
- [x] API/check-context repair PR #25 was squash-merged as `287e7fea0c50d1a29f09e82e6df31fcee9ce9c0f`.
- [x] Solo-maintainer autonomy repair PR #27 was squash-merged as `9c4942d3429cb2eb1596bb5333cc3dc6950cbaa3`.
- [x] Configuration-concurrency repair PR #28 was squash-merged as `664bd9a2ede227220705d1058cc420c4a08bd6b2`.
- [ ] The task is archived in a separate closeout PR after live E2E verification succeeds.

## Excluded scope

- Rust workspace, product code, gameplay architecture, protocol, persistence, deployment, release signing, license selection, organization-level policies, and external repositories.

## Validation

- implementation exact head: `8175a9d016b7e204b0cdf1d55895d71952714e4b`
- implementation Agent governance: `31012515907` — PASS
- implementation Dependency review: `31012516187` — PASS
- implementation CodeQL: `31012516757` — PASS
- implementation merge: `30324872af421d0d2bdcb91b360a76a3d44a2592`
- API/check-context repair exact head: `a40f0f5fe5fdef3fbcd8384850094623ec99b6d7`
- API/check-context repair Agent governance: `31013637676` — PASS
- API/check-context repair Dependency review: `31013638308` — PASS
- API/check-context repair CodeQL: `31013637848` — PASS
- API/check-context repair merge: `287e7fea0c50d1a29f09e82e6df31fcee9ce9c0f`
- autonomy repair exact head: `5b1dd49bfc71952e3e3c4dc2874096b6be75cffd`
- autonomy repair Agent governance: `31014156876` — PASS
- autonomy repair Dependency review: `31014156309` — PASS
- autonomy repair CodeQL: `31014156500` — PASS
- autonomy repair merge: `9c4942d3429cb2eb1596bb5333cc3dc6950cbaa3`
- concurrency repair exact head: `09a58ba3d29576fe5e7e0f7376b1e6d24a5a863c`
- concurrency repair Agent governance: `31014453379` — PASS
- concurrency repair Dependency review: `31014453260` — PASS
- concurrency repair CodeQL: `31014453330` — PASS
- concurrency repair merge: `664bd9a2ede227220705d1058cc420c4a08bd6b2`
- live ruleset: `Protect main`, id `20462155`, active, no bypass — PASS
- private vulnerability reporting: enabled — PASS
- repository metadata/topics/merge settings: applied — PASS
- legacy administration environment: removed — PASS
- configuration E2E run `31014596403`: current apply ran without approval/concurrency blocking; final verification found only obsolete API field `has_downloads` absent from the 2026 response
- API-field repair: remove legacy downloads feature from the canonical repository policy instead of treating an unobservable field as enforceable

## Context checkpoint

```yaml
last_progress: Configuration apply now executes autonomously and removes obsolete state; only the deprecated has_downloads field prevents terminal verification.
status: validating
branch: fix/repository-policy-api-fields-20260805
head_sha: null
pr: null
ci_check_generation: repository-policy-api-field-repair
ci_checks_for_current_head: 0
terminal_ci_wait_started_at: null
terminal_ci_checks_for_current_generation: 0
unchanged_state_checks: 0
identical_failure_retries: 0
repair_cycles_for_current_gate: 4
stall_warnings: 0
blocker: null
next_action: Merge the API-field repair through the active ruleset, rerun repository configuration E2E, verify live state, and archive the task.
```
