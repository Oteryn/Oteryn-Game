# OTV2-20260805-github-governance-hardening

```yaml
task_id: OTV2-20260805-github-governance-hardening
title: Harden GitHub contribution, merge, CI and security governance
mode: IMPLEMENTATION
status: validating
repository: blakinio/Oteryn-v2
base_branch: main
branch: ci/github-governance-hardening-20260805
pr: 23
base_sha: 0cff8ae0c98cddefd18a29b1c4da0935f94a74fd
head_sha: null
owner: repository-governance-agent
created_at: 2026-08-05T15:33:00+02:00
updated_at: 2026-08-05T15:57:00+02:00
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

- [ ] `main` has an active no-routine-bypass ruleset requiring PR, exact-head CI, up-to-date branch, resolved threads, squash, linear history, and no force-push/deletion.
- [ ] GitHub Actions default token permissions are read-only.
- [ ] Pull request and issue templates, CODEOWNERS, contribution policy, security policy, ignore rules, and editor configuration exist.
- [ ] Every workflow action is pinned to a full commit SHA.
- [ ] Governance validation runs on every PR and push to `main`.
- [ ] PR title/body policy is executable and supplies the permanent squash commit.
- [ ] Dependency review, Dependabot for Actions, and CodeQL are configured.
- [ ] Managed labels and repository topics are applied.
- [ ] Vulnerability alerts, automated fixes, private reporting, secret scanning, and push protection are enabled where supported.
- [ ] Exact-head workflows pass and the full diff has no open material audit finding.
- [ ] The implementation PR is squash-merged and the task is archived in a separate closeout PR.

## Excluded scope

- Rust workspace, product code, gameplay architecture, protocol, persistence, deployment, release signing, license selection, organization-level policies, and external repositories.

## Validation

- focused: `python tools/repository/validate_repository_policy.py`
- governance: `python tools/agents/validate_governance.py`
- component: GitHub Actions workflows on exact PR head
- E2E: repository-settings workflow applies and verifies live GitHub configuration after merge
- audit: full changed-file and workflow-security review
- exact_head: pending

## Context checkpoint

```yaml
last_progress: Full policy audit completed; final repair makes the PR title/body the canonical squash commit while preserving the bounded dependency-review bootstrap exception.
status: validating
branch: ci/github-governance-hardening-20260805
head_sha: null
pr: 23
ci_check_generation: final-audit-repair
ci_checks_for_current_head: 0
terminal_ci_wait_started_at: null
terminal_ci_checks_for_current_generation: 0
unchanged_state_checks: 0
identical_failure_retries: 0
repair_cycles_for_current_gate: 3
stall_warnings: 0
blocker: null
next_action: Run final exact-head Agent governance, CodeQL, and Dependency review, then merge if clean.
```
