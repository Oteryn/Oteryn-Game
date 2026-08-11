# OTV2-20260811-merge-gate-hardening

```yaml
task_id: OTV2-20260811-merge-gate-hardening
title: Harden PR merge gating and repository engineering drift controls
mode: REPAIR
status: implementing
repository: blakinio/Oteryn-v2
base_branch: main
branch: ci/OTV2-20260811-merge-gate-hardening
pr: null
base_sha: c94f331ec20849e8306875a2f88b87fa7e1974f9
owner: ChatGPT repository engineering agent
created_at: 2026-08-11T10:30:00+02:00
updated_at: 2026-08-11T10:30:00+02:00
execution_budget_minutes: 120
owned_paths:
  - .github/workflows/merge-gate.yml
  - .github/workflows/agent-governance.yml
  - .github/workflows/codeql.yml
  - .github/workflows/dependency-review.yml
  - .github/workflows/rust.yml
  - .github/workflows/rust-cutover-terminal-audit.yml
  - .github/dependabot.yml
  - .github/repository-policy.json
  - .github/CODEOWNERS
  - tools/repository/validate_repository_policy.py
  - tools/repository/apply_github_settings.py
  - docs/agents/BUILD_TEST_MATRIX.md
  - docs/agents/tasks/active/OTV2-20260811-merge-gate-hardening.md
public_contracts:
  - .github/repository-policy.json
  - docs/agents/BUILD_TEST_MATRIX.md
depends_on: []
blocks: []
external_repositories: []
```

## Outcome

Make one always-present `Merge gate / validate` check the protected-branch merge authority for pull requests, with governance, dependency review, CodeQL and path-proportional Rust validation composed behind it. Remove stale workspace/bootstrap wording, add Cargo dependency maintenance and retire the completed one-off Rust cutover audit workflow.

## Preflight

- base verified: `main@c94f331ec20849e8306875a2f88b87fa7e1974f9`;
- open PR #161 is task-closeout documentation only and owns different paths;
- no runtime/gameplay/product behavior is changed;
- production/deployment authority is not granted.

## Acceptance

- [ ] one PR workflow always emits `Merge gate / validate`;
- [ ] governance, dependency review and CodeQL are mandatory sub-gates;
- [ ] Rust Linux/Windows/supply-chain checks run when Rust/workspace-relevant paths change;
- [ ] repository policy requires only the stable aggregate merge-gate context;
- [ ] repository-policy validators prove policy/live required-status consistency;
- [ ] Cargo Dependabot is enabled;
- [ ] stale workspace/test-matrix and CODEOWNERS wording is corrected;
- [ ] obsolete migration-only workflow is removed;
- [ ] exact-head self-review and all applicable CI pass before merge.

## Context checkpoint

```yaml
last_progress: Claimed an isolated repository-engineering repair package from current main with no overlap with PR #161.
status: implementing
branch: ci/OTV2-20260811-merge-gate-hardening
pr: null
final_head_sha: null
owner_action_required: null
blocker: null
next_action: Implement the aggregate merge gate and supporting repository-policy/dependency/documentation repairs, then open a PR and validate the exact head.
```
