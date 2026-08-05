# OTV2-20260805-github-governance-hardening

```yaml
task_id: OTV2-20260805-github-governance-hardening
title: Harden GitHub contribution, merge, CI and security governance
mode: IMPLEMENTATION
status: validating
repository: blakinio/Oteryn-v2
base_branch: main
branch: fix/repository-config-verification-20260805
pr: 23
base_sha: 30324872af421d0d2bdcb91b360a76a3d44a2592
head_sha: null
owner: repository-governance-agent
created_at: 2026-08-05T15:33:00+02:00
updated_at: 2026-08-05T16:06:00+02:00
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
- [ ] The task is archived in a separate closeout PR after live E2E verification succeeds.

## Excluded scope

- Rust workspace, product code, gameplay architecture, protocol, persistence, deployment, release signing, license selection, organization-level policies, and external repositories.

## Validation

- implementation exact head: `8175a9d016b7e204b0cdf1d55895d71952714e4b`
- Agent governance: run `31012515907` — PASS
- Dependency review: run `31012516187` — PASS
- CodeQL: run `31012516757` — PASS
- implementation merge: `30324872af421d0d2bdcb91b360a76a3d44a2592`
- live ruleset: `Protect main`, id `20462155`, active, no bypass — PASS
- private vulnerability reporting: enabled — PASS
- repository metadata/topics/merge settings: applied — PASS
- configuration E2E run `31012977676`: settings applied, verification failed only because API version 2026 omits legacy response field `use_squash_pr_title_as_default`
- repair: verify the equivalent canonical field `squash_merge_commit_title == PR_TITLE` when the legacy alias is absent

## Context checkpoint

```yaml
last_progress: PR #23 merged and live protection was applied; API verification exposed one response-compatibility defect after all mutations completed.
status: validating
branch: fix/repository-config-verification-20260805
head_sha: null
pr: null
ci_check_generation: repository-config-verifier-repair
ci_checks_for_current_head: 0
terminal_ci_wait_started_at: null
terminal_ci_checks_for_current_generation: 0
unchanged_state_checks: 0
identical_failure_retries: 0
repair_cycles_for_current_gate: 1
stall_warnings: 0
blocker: null
next_action: Merge the verifier repair after exact-head checks, rerun repository configuration, verify live state, then archive the task.
```
