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
updated_at: 2026-08-11T10:52:00+02:00
execution_budget_minutes: 120
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

## Preflight and reconciliation

- task started from `main@c94f331ec20849e8306875a2f88b87fa7e1974f9`;
- PR #161 was a non-overlapping player-promise task closeout and merged while this package was being prepared;
- branch was reconciled non-destructively with current `main@f184930fac66fdf9ae0cc7f606d3502c17626a79` by a merge commit preserving both histories;
- current compare is `behind_by=0`; no player-promise active/archive path appears in this task diff;
- existing `Agent governance / validate` pull-request workflow is intentionally left unchanged in this transition package so the currently active ruleset can validate this PR before the aggregate gate becomes the protected status check;
- no runtime/gameplay/product behavior is changed;
- production/deployment authority is not granted.

## Implemented repair

- Added always-on `.github/workflows/merge-gate.yml` for PRs to `main`.
- Aggregate gate always requires governance/repository-policy validation, Dependency Review and CodeQL.
- Aggregate gate detects Rust/workspace-sensitive paths and then requires Rust policy/metadata, Linux build/Clippy/tests/harness, Windows release build/Clippy/smoke/harness and cargo-deny.
- Changed retained repository policy from `Agent governance / validate` to the stable `Merge gate / validate` required context.
- Extended repository validators so the declared and live ruleset required-status contexts must match exactly.
- Added Cargo Dependabot alongside GitHub Actions Dependabot.
- Converted standalone Rust and CodeQL workflows to post-merge/manual validation to avoid duplicate PR execution once the aggregate gate is active.
- Removed standalone Dependency Review because it is composed into the aggregate gate.
- Removed the completed migration-only Rust cutover terminal audit workflow.
- Refreshed `BUILD_TEST_MATRIX.md` to the actual current root workspace and exact validation commands.
- Updated CODEOWNERS from future-workspace wording to current Rust workspace and machine-boundary ownership.

## Acceptance

- [x] one PR workflow always emits `Merge gate / validate`;
- [x] governance, dependency review and CodeQL are mandatory sub-gates;
- [x] Rust Linux/Windows/supply-chain checks run when Rust/workspace-relevant paths change;
- [x] repository policy requires only the stable aggregate merge-gate context;
- [x] repository-policy validators prove declared/live required-status consistency;
- [x] Cargo Dependabot is enabled;
- [x] stale workspace/test-matrix and CODEOWNERS wording is corrected;
- [x] obsolete migration-only workflow is removed;
- [ ] exact-head full-diff self-review has no open material finding;
- [ ] exact-head GitHub Actions validation passes, including both the transition `Agent governance / validate` and new `Merge gate / validate`;
- [ ] independent review/audit is reconciled if the repository's automatic independent reviewer runs; otherwise record the exact availability result before readiness;
- [ ] squash merge only after unchanged-head readiness.

## Validation notes

- Local checkout validation is unavailable in this session because the local environment cannot resolve `github.com`; this is not treated as GitHub connector unavailability.
- Exact-head GitHub Actions is the authoritative executable validation for this workflow/policy change.
- Because this changes the repository merge authority path, review the full final diff for accidental gate weakening, permission expansion, path-filter bypass and transition deadlock before readiness.

## Context checkpoint

```yaml
last_progress: Implemented the aggregate PR merge gate, dependency/security/workspace composition, repository-policy consistency checks, Cargo Dependabot and stale CI/documentation cleanup; reconciled the branch with the non-overlapping PR #161 closeout and preserved the currently-required governance PR check for the transition.
status: implementing
branch: ci/OTV2-20260811-merge-gate-hardening
pr: null
final_head_sha: null
owner_action_required: null
blocker: null
next_action: Open the bounded PR, record its exact head, perform full-diff self-review, then verify transition governance plus every new aggregate merge-gate job on that unchanged head.
```
